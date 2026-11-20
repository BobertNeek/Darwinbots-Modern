using System.Buffers.Binary;
using System.Reflection;
using System.Text.Json;
using System.Text.Json.Nodes;
using Avalonia.Controls;
using Avalonia.Interactivity;
using Avalonia.Threading;
using Avalonia.VisualTree;
using Darwinbots.Desktop.Controls;
using Darwinbots.Desktop.Core;
using Darwinbots.Desktop.Core.LegacyMenus;
using Darwinbots.Desktop.ViewModels;
using Darwinbots.Desktop.Views;
using Xunit;

namespace Darwinbots.Desktop.Tests;

public sealed class SurvivalInfoRenderedTests
{
    [Fact]
    public async Task SurvivalInfoFollowsTheNativeRunAndItsTestingPhase()
    {
        var setup = new WorldSetupOptions
        {
            Backend = "Cpu", PopulationCapacity = 64, VegetablePopulationCap = 0,
            GlobalSettings = new GlobalSettings { Main = new GlobalMainSettings { NewSeed = false } },
            Species = [new SpeciesImport("World", "start stop", false, 0xff239ac0, 1, 1_000, 0, false, 0)],
        };
        await DesktopTestApplication.Session.Dispatch(async () =>
        {
            var window = new MainWindow([], setup, NativeEngineClientFactory.Instance, new AuditStorageService());
            window.Show();
            try
            {
                using var timeout = new CancellationTokenSource(TimeSpan.FromSeconds(10));
                var model = Assert.IsType<MainWindowViewModel>(window.DataContext);
                while (!model.Status.StartsWith("WORLD READY", StringComparison.Ordinal))
                    await Task.Delay(10, timeout.Token);
                var session = (SimulationSession)typeof(MainWindow)
                    .GetField("_session", BindingFlags.Instance | BindingFlags.NonPublic)!.GetValue(window)!;
                var command = window.FindControl<LegacyMenuBar>("HistoricMenuBar")!
                    .FindCommandItem(LegacyCommandId.HelpSurvivalInfo)!;

                var ordinary = await session.SaveAsync();
                Assert.False(command.IsVisible);
                await session.ConfigureLocalModeAsync(LocalModeConfig.Default with
                {
                    Kind = LocalModeKind.SimpleSurvival,
                    StagedEvolution = new(EvolutionMode.SimpleSurvival, "start stop", 2.25f, 150, true, 255, false),
                });
                await Dispatcher.UIThread.InvokeAsync(() => { }, DispatcherPriority.Background);
                Assert.False(command.IsVisible);

                await session.StartLocalModeAsync();
                await Dispatcher.UIThread.InvokeAsync(() => { }, DispatcherPriority.Background);
                Assert.True(command.IsVisible);
                var state = Assert.IsType<StagedEvolutionSnapshot>(session.LatestSnapshot.LocalMode.Evolution);
                Assert.NotNull(state.CurrentDnaSize);
                var live = await session.SaveAsync();
                command.RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));
                var dialog = Assert.Single(window.OwnedWindows.OfType<LegacyDocumentWindow>());
                Assert.Contains("Survival information", dialog.Title, StringComparison.Ordinal);
                var report = Assert.Single(dialog.GetVisualDescendants().OfType<TextBox>());
                Assert.Contains($"Target DNA size: {state.CurrentDnaSize}", report.Text, StringComparison.Ordinal);
                Assert.Contains("Next DNA size change: 250", report.Text, StringComparison.Ordinal);
                Assert.Contains($"Reduction unit: {2.25f}", report.Text, StringComparison.Ordinal);
                Assert.Contains("On/Off cycles: 150", report.Text, StringComparison.Ordinal);
                Assert.Contains("Current Handicap: 0 - 0 = 0", report.Text, StringComparison.Ordinal);
                Assert.Equal(live, await session.SaveAsync());
                dialog.Close();

                await session.LoadAsync(TestingCheckpoint(live));
                await Dispatcher.UIThread.InvokeAsync(() => { }, DispatcherPriority.Background);
                Assert.True(command.IsVisible);
                command.RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));
                var testingDialog = Assert.Single(window.OwnedWindows.OfType<LegacyDocumentWindow>());
                var testingReport = Assert.Single(testingDialog.GetVisualDescendants().OfType<TextBox>()).Text;
                Assert.Contains("Target DNA size: N/A", testingReport, StringComparison.Ordinal);
                Assert.Contains("Next DNA size change: N/A", testingReport, StringComparison.Ordinal);
                Assert.Contains("On/Off cycles: 271", testingReport, StringComparison.Ordinal);
                Assert.DoesNotContain("Current Handicap", testingReport, StringComparison.Ordinal);
                testingDialog.Close();

                await session.LoadAsync(ordinary);
                await Dispatcher.UIThread.InvokeAsync(() => { }, DispatcherPriority.Background);
                Assert.False(command.IsVisible);
            }
            finally
            {
                foreach (var owned in window.OwnedWindows.ToArray()) owned.Close();
                window.Close();
            }
        }, CancellationToken.None);
    }

    private static byte[] TestingCheckpoint(byte[] save)
    {
        const int headerSize = 10;
        var payload = JsonNode.Parse(save.AsSpan(headerSize))!;
        var mode = payload["local_mode"]!;
        var evolution = mode["evolution"]!;
        mode["config"]!["staged_evolution"]!["normalize_dna_size"] = false;
        evolution["config"]!["normalize_dna_size"] = false;
        evolution["phase"] = "testing";
        evolution["candidate"] = evolution["mutators"]!.DeepClone();
        evolution["hide_cycles"] = 271;
        evolution["handicap"] = -3;
        evolution["inverse_handicap"] = -1;
        var json = JsonSerializer.SerializeToUtf8Bytes(payload);
        var bytes = new byte[headerSize + json.Length];
        save.AsSpan(0, headerSize).CopyTo(bytes);
        BinaryPrimitives.WriteInt32LittleEndian(bytes.AsSpan(6, 4), json.Length);
        json.CopyTo(bytes, headerSize);
        return bytes;
    }
}
