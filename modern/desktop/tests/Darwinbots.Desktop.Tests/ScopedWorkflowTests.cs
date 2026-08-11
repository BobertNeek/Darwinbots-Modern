using System.Reflection;
using Avalonia.Controls;
using Avalonia.Headless;
using Darwinbots.Desktop.Core;
using Darwinbots.Desktop.Views;
using Xunit;

namespace Darwinbots.Desktop.Tests;

public sealed class ScopedWorkflowTests
{
    [Fact]
    public async Task SetupExposesSixTabWorkingCopyAndCommitControls()
    {
        await DesktopTestApplication.Session.Dispatch(() =>
        {
            var window = new SetupWindow();
            try
            {
                var tabs = window.FindControl<TabControl>("SimulationSettingsTabs");
                Assert.NotNull(tabs);
                Assert.Equal(6, tabs!.Items.Count);
                Assert.NotNull(window.FindControl<Button>("StartNewButton"));
                Assert.NotNull(window.FindControl<Button>("ChangeButton"));
                Assert.NotNull(window.FindControl<Button>("CancelButton"));
                Assert.NotNull(window.FindControl<Border>("OnboardingPanel"));
            }
            finally
            {
                window.Close();
            }
        }, CancellationToken.None);
    }

    [Fact]
    public void SettingsWorkflowKeepsWorkingCopySeparateUntilStartNewOrChange()
    {
        var workflowType = typeof(WorldSetupOptions).Assembly.GetType(
            "Darwinbots.Desktop.Core.SimulationSettingsWorkflow");
        Assert.NotNull(workflowType);

        var initial = new WorldSetupOptions { WorldWidth = 16_000f };
        var workflow = Activator.CreateInstance(workflowType!, initial)!;
        var update = workflowType!.GetMethod("UpdateWorkingCopy")!;
        update.Invoke(workflow, [new Func<WorldSetupOptions, WorldSetupOptions>(
            settings => settings with { WorldWidth = 91_000f })]);

        Assert.Equal(16_000f, ((WorldSetupOptions)workflowType.GetProperty("Live")!.GetValue(workflow)!).WorldWidth);
        Assert.Equal(91_000f, ((WorldSetupOptions)workflowType.GetProperty("WorkingCopy")!.GetValue(workflow)!).WorldWidth);

        workflowType.GetMethod("Cancel")!.Invoke(workflow, null);
        Assert.Equal(16_000f, ((WorldSetupOptions)workflowType.GetProperty("WorkingCopy")!.GetValue(workflow)!).WorldWidth);

        update.Invoke(workflow, [new Func<WorldSetupOptions, WorldSetupOptions>(
            settings => settings with { WorldWidth = 72_000f })]);
        workflowType.GetMethod("Change")!.Invoke(workflow, null);
        Assert.Equal(72_000f, ((WorldSetupOptions)workflowType.GetProperty("Live")!.GetValue(workflow)!).WorldWidth);

        update.Invoke(workflow, [new Func<WorldSetupOptions, WorldSetupOptions>(
            settings => settings with { WorldHeight = 88_000f })]);
        workflowType.GetMethod("StartNew")!.Invoke(workflow, null);
        var committed = (WorldSetupOptions)workflowType.GetProperty("Live")!.GetValue(workflow)!;
        Assert.Equal(88_000f, committed.WorldHeight);

        var settings = DesktopSettings.FromWorldSetup(committed with
        {
            Species = [new SpeciesImport("Zerobot", "0 0", false, 1, 10, 100, 0, false, 1f)],
        });
        var jsonRoundTrip = DesktopSettings.FromJson(settings.ToJson());
        var setRoundTrip = Db2SettingsCodec.Decode(Db2SettingsCodec.Encode(settings));
        Assert.Equal(88_000f, jsonRoundTrip.WorldHeight);
        Assert.Single(jsonRoundTrip.Species);
        Assert.Single(setRoundTrip.Species);
    }

    [Fact]
    public void ZerobotSetupUsesTheNativeOfflineSelectionStage()
    {
        var kindMethod = typeof(MainWindow).GetMethod(
            "LocalModeKindFromIndex", BindingFlags.Static | BindingFlags.NonPublic);
        Assert.NotNull(kindMethod);
        Assert.Equal("Zerobot", kindMethod!.Invoke(null, [4])!.ToString());
        var nativeName = typeof(NativeEngineClient).GetMethod(
            "ModeKindName", BindingFlags.Static | BindingFlags.NonPublic);
        Assert.Equal("zerobot", nativeName!.Invoke(null, [LocalModeKind.Zerobot]));
    }

    [Fact]
    public async Task ZerobotSetupConfiguresAndStartsTheNativeOfflineWorkflow()
    {
        var factory = new ScopedRecordingEngineClientFactory();
        await DesktopTestApplication.Session.Dispatch(async () =>
        {
            var window = new MainWindow(
                [],
                new WorldSetupOptions
                {
                    StartingMode = StartingMode.Zerobots,
                    AutomaticZerobotProgression = true,
                },
                factory,
                new SettingsWorkflowStorage());
            window.Show();
            try
            {
                await WaitFor(() => factory.Client.Calls.Contains("ConfigureLocalMode")
                    && factory.Client.Calls.Contains("StartLocalMode"));
            }
            finally
            {
                window.Close();
            }
        }, CancellationToken.None);

        Assert.Contains(factory.Client.LocalModeConfigs, config => config.Kind == LocalModeKind.Zerobot);
    }

    [Fact]
    public void RecoveryDecisionKeepsTheCausalFaultWindowAndArtifact()
    {
        var diagnostics = new OfflineDiagnosticRecovery();
        diagnostics.BeginSession("last-start.json");
        diagnostics.RecordCheckpoint(10_000, "autosave.db3s");
        var decision = diagnostics.HandleError(new InvalidOperationException("engine fault"), 11_987, "autosave.db3s");

        Assert.Equal(DiagnosticRecoveryAction.Recover, decision.Action);
        Assert.True(decision.StopSimulation);
        Assert.Equal(10_000ul, decision.FaultWindow!.StartTick);
        Assert.Equal(11_987ul, decision.FaultWindow.EndTick);
        Assert.Equal("autosave.db3s", Path.GetFileName(diagnostics.RecoveryArtifactPath));
    }

    [Fact]
    public async Task RecoveryAndFirstRunSurfacesAreStructuredNativeControls()
    {
        await DesktopTestApplication.Session.Dispatch(() =>
        {
            var setup = new SetupWindow();
            var runtime = new MainWindow([], new WorldSetupOptions());
            try
            {
                Assert.NotNull(setup.FindControl<Border>("OnboardingPanel"));
                Assert.NotNull(setup.FindControl<Button>("OnboardingGotItButton"));
                Assert.NotNull(runtime.FindControl<Border>("RecoveryPanel"));
                Assert.NotNull(runtime.FindControl<TextBlock>("RecoveryFaultWindowText"));
                Assert.NotNull(runtime.FindControl<Button>("RecoveryRecoverButton"));
            }
            finally
            {
                setup.Close();
                runtime.Close();
            }
        }, CancellationToken.None);
    }

    private static async Task WaitFor(Func<bool> condition)
    {
        for (var attempt = 0; attempt < 100; attempt++)
        {
            if (condition()) return;
            await Task.Delay(10);
        }

        Assert.True(condition(), "Timed out waiting for the native Zerobot workflow.");
    }
}

internal sealed class ScopedRecordingEngineClientFactory : IEngineClientFactory
{
    public RecordingEngineClient Client { get; } = new();

    public IEngineClient Create(WorldSetupOptions setup) => Client;
}
