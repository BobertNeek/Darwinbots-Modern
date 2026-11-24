using System.Reflection;
using Avalonia.Controls;
using Avalonia.Headless;
using Avalonia.Interactivity;
using Avalonia.VisualTree;
using Darwinbots.Desktop.Controls;
using Darwinbots.Desktop.Core;
using Darwinbots.Desktop.Core.LegacyMenus;
using Darwinbots.Desktop.ViewModels;
using Darwinbots.Desktop.Views;
using Xunit;

namespace Darwinbots.Desktop.Tests;

public sealed class SelectedRobotToolsRenderedTests
{
    [Fact]
    public async Task AutoTagMenuEditsMatchingNativeRobotsAndKeepsCanceledChangesOut()
    {
        await using var simulation = new SimulationSession(new NativeEngineClient(new WorldSetupOptions
        {
            Backend = "Cpu", PopulationCapacity = 64,
        }));
        const string dna = "start stop\n'#tag: Old tag";
        await simulation.ImportSpeciesAsync(new SpeciesImport("Founder.txt", dna, false, 0xff239ac0, 2, 1000, 0, false, 0));
        await simulation.ImportSpeciesAsync(new SpeciesImport("founder.txt", dna, false, 0xff239ac0, 1, 1000, 0, false, 0));

        await DesktopTestApplication.Session.Dispatch(async () =>
        {
            var window = new MainWindow([], new WorldSetupOptions(), NativeEngineClientFactory.Instance, new AuditStorageService());
            DesktopTestApplication.UseSession(window, simulation);
            window.Show();
            try
            {
                var router = (LegacyCommandRouter)typeof(MainWindow)
                    .GetField("_legacyCommands", BindingFlags.Instance | BindingFlags.NonPublic)!.GetValue(window)!;
                const LegacyCommandId command = LegacyCommandId.EditAutomaticallyTagByName;
                Assert.False(router.GetState(command).Checked);

                var apply = router.ExecuteAsync(command);
                CompleteTagDialog(await WaitForTagDialog(window), "Founder.txt", "APPLY");
                CompleteTagDialog(await WaitForTagDialog(window), " leadéÿ", "APPLY");
                Assert.Equal("TAGGED · 2 ROBOT(S)", (await apply).Status);

                var cancel = router.ExecuteAsync(command);
                CompleteTagDialog(await WaitForTagDialog(window), "Founder.txt", "APPLY");
                CompleteTagDialog(await WaitForTagDialog(window), "Discard this", "CANCEL");
                Assert.True((await cancel).Canceled);

                var snapshot = simulation.LatestSnapshot;
                Assert.Equal(3, snapshot.Population);
                Assert.Equal(0ul, snapshot.Tick);
                foreach (var robot in snapshot.Organisms)
                {
                    var label = snapshot.Species[(int)robot.Species].Name;
                    var expected = label == "Founder.txt" ? " lead?ÿ" : "Old tag";
                    Assert.EndsWith($"'#tag: {expected}", await simulation.ExportDnaAsync(robot.Slot, robot.Generation), StringComparison.Ordinal);
                }
            }
            finally
            {
                foreach (var dialog in window.OwnedWindows.ToArray()) dialog.Close();
                window.Close();
            }
        }, CancellationToken.None);
    }

    [Fact]
    public async Task MakeNewSpeciesMenuConfirmsAndForksTheLiveDescendantBranch()
    {
        var options = new WorldSetupOptions
        {
            Backend = "Cpu", PopulationCapacity = 64,
            GlobalSettings = new GlobalSettings { Main = new GlobalMainSettings { NewSeed = false } },
        };
        using var engine = new NativeEngineClient(options);
        engine.ImportSpecies(new("Founder.txt", "start stop", false, 0xff239ac0, 2, 1000, 0, false, 0));
        var before = engine.Snapshot();
        var root = before.Organisms[0];
        var sibling = before.Organisms[1];
        var child = engine.CloneOrganism(root.Slot, root.Generation, [100f, 100f]);
        await using var simulation = new SimulationSession(engine);
        await simulation.RefreshSnapshotAsync();

        await DesktopTestApplication.Session.Dispatch(async () =>
        {
            var window = new MainWindow([], options, new AuditEngineClientFactory(), new AuditStorageService());
            try
            {
                DesktopTestApplication.UseSession(window, simulation);
                window.Show();
                var model = (MainWindowViewModel)window.DataContext!;
                var viewport = window.FindControl<WorldViewport>("Viewport")!;
                var select = (Action<OrganismKey>)typeof(WorldViewport)
                    .GetField("OrganismKeySelected", BindingFlags.Instance | BindingFlags.NonPublic)!
                    .GetValue(viewport)!;
                var menu = window.FindControl<LegacyMenuBar>("HistoricMenuBar")!;
                var action = menu.FindCommandItem(LegacyCommandId.RobotMakeNewSpecies)!;

                using var timeout = new CancellationTokenSource(TimeSpan.FromSeconds(5));
                model.ClearSelection();
                Assert.True(action.IsEnabled);
                action.RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));
                while (model.Status != "CANCELED")
                    await Task.Delay(10, timeout.Token);
                Assert.Empty(window.OwnedWindows.OfType<LegacyConfirmationWindow>());
                Assert.Equal(before.Species.Count, simulation.LatestSnapshot.Species.Count);

                model.Status = "READY";
                select(new OrganismKey(root.Slot, root.Generation));
                action.RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));
                Choose("NO");
                while (model.Status != "CANCELED")
                    await Task.Delay(10, timeout.Token);
                Assert.Equal(before.Species.Count, simulation.LatestSnapshot.Species.Count);

                action.RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));
                Choose("YES");
                while (simulation.LatestSnapshot.Species.Count != before.Species.Count + 1)
                    await Task.Delay(10, timeout.Token);

                var after = simulation.LatestSnapshot;
                var newSpecies = (uint)after.Species.Count - 1;
                Assert.Matches(@"^Founder \d{1,5}$", after.Species[(int)newSpecies].Name);
                Assert.Equal(newSpecies, Assert.Single(after.Organisms, robot => robot.Slot == root.Slot).Species);
                Assert.Equal(newSpecies, Assert.Single(after.Organisms, robot => robot.Slot == child.Slot).Species);
                Assert.Equal(sibling.Species, Assert.Single(after.Organisms, robot => robot.Slot == sibling.Slot).Species);
                Assert.Equal(before.Tick, after.Tick);

                void Choose(string answer)
                {
                    var dialog = Assert.Single(window.OwnedWindows.OfType<LegacyConfirmationWindow>());
                    var button = Assert.Single(dialog.GetVisualDescendants().OfType<Button>(),
                        value => Equals(value.Content, answer));
                    button.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                }
            }
            finally
            {
                foreach (var dialog in window.OwnedWindows.ToArray()) dialog.Close();
                window.Close();
            }
        }, CancellationToken.None);
    }

    [Fact]
    public async Task EyeDesignerOpensWithoutAWorldAndExportsWithoutReplacingTheFocusedGenome()
    {
        var vision = VisionSnapshot.Default;
        vision.Eyes[0] = vision.Eyes[0] with { Direction = 12, Width = 34 };
        var organism = new OrganismSnapshot(
            7,
            3,
            [100f, 200f],
            [1f, -1f],
            875,
            19,
            Species: 1,
            Parents: [new OrganismKey(2, 1), null])
        {
            Vision = vision,
            Memory = [
                new MemoryCellSnapshot(310, 16),
                new MemoryCellSnapshot(311, 32),
                new MemoryCellSnapshot(312, 48),
            ],
        };
        var snapshot = new EngineSnapshot(12, 1, "CPU", [organism])
        {
            Species = [new SpeciesSnapshot("Unassigned", false, 0xff858982, 0, false)],
        };
        var engine = new RecordingEngineClient(snapshot);
        await using var simulation = new SimulationSession(engine);
        await simulation.StepAsync(0);

        await DesktopTestApplication.Session.Dispatch(async () =>
        {
            var focus = new OrganismKey(7, 3);
            var storage = new AuditStorageService();
            var player = new MainWindow([], new WorldSetupOptions(), new AuditEngineClientFactory(), storage);
            player.Opened -= typeof(MainWindow).GetMethod("Window_Opened", BindingFlags.Instance | BindingFlags.NonPublic)!
                .CreateDelegate<EventHandler>(player);
            var rgb = new RgbMemoryMonitorWindow(focus, RgbMemoryMonitorSettings.Default, _ => { });
            var phylogeny = new PhylogenyWindow(snapshot, focus);
            try
            {
                player.Show();
                var eye = OpenEyeDesigner(player);
                Assert.False(EyeButton(eye, "Reset Aim").IsEnabled);
                Assert.False(EyeButton(eye, "Turn Off Brownian Motion").IsEnabled);
                Assert.False(EyeButton(eye, "Disable Costs").IsEnabled);
                rgb.SetSnapshot(snapshot);
                rgb.Show();
                phylogeny.Show();
                AvaloniaHeadlessPlatform.ForceRenderTimerTick();

                AssertRendered(eye);
                AssertRendered(rgb);
                AssertRendered(phylogeny);

                var fields = eye.GetVisualDescendants().OfType<TextBox>().ToArray();
                Assert.Equal(18, fields.Length);
                Assert.All(fields, field => Assert.Equal("0", field.Text));
                Assert.Single(fields, field => field.Name == "Eye1Direction").Text = "17";
                Assert.Single(fields, field => field.Name == "Eye1Width").Text = "-45";
                await eye.WriteDnaAsync();
                Assert.Equal(EyeFile(firstDirection: 17, firstWidth: -45), storage.SavedDna);
                Assert.Null(storage.SavedDnaMutationProfile);

                DesktopTestApplication.UseSession(player, simulation);
                Assert.IsType<MainWindowViewModel>(player.DataContext).Select(focus);
                Assert.Same(eye, OpenEyeDesigner(player));
                await eye.WriteDnaAsync();
                Assert.Equal(EyeFile(firstDirection: 12, firstWidth: 34), storage.SavedDna);
                Assert.Null(storage.SavedDnaMutationProfile);
                Assert.Empty(engine.DnaReplacements);
            }
            finally
            {
                player.Close();
                if (rgb.IsVisible) rgb.Close();
                if (phylogeny.IsVisible) phylogeny.Close();
            }

            return 0;
        }, CancellationToken.None);
    }

    [Fact]
    public async Task EyeEditsWriteOnlyTheChangedCellOnTheCurrentlyFocusedRobot()
    {
        var setup = new WorldSetupOptions { Backend = "Cpu", WorldWidth = 1000, WorldHeight = 1000, PopulationCapacity = 8 };
        await using var session = new SimulationSession(new NativeEngineClient(setup));
        await session.ImportDnaAsync("start\nstop", [300, 300]);
        await session.ImportDnaAsync("start\nstop", [600, 300]);
        var bots = session.LatestSnapshot.Organisms.OrderBy(bot => bot.Position[0]).ToArray();
        var first = new OrganismKey(bots[0].Slot, bots[0].Generation);
        var second = new OrganismKey(bots[1].Slot, bots[1].Generation);

        await DesktopTestApplication.Session.Dispatch(async () =>
        {
            var player = new MainWindow([], setup, new AuditEngineClientFactory(), new AuditStorageService());
            DesktopTestApplication.UseSession(player, session);
            try
            {
                player.Show();
                var model = Assert.IsType<MainWindowViewModel>(player.DataContext);
                model.Select(first);
                var eye = OpenEyeDesigner(player);
                var fields = eye.GetVisualDescendants().OfType<TextBox>().ToArray();
                var direction = Assert.Single(fields, field => field.Name == "Eye1Direction");
                var width = Assert.Single(fields, field => field.Name == "Eye1Width");

                direction.Text = "invalid";
                width.Text = "110";
                await WaitFor(() => ReadEye(session, first).Width == 110);
                Assert.Equal(0, ReadEye(session, first).Direction);
                Assert.Equal(0, ReadEye(session, second).Width);

                direction.Text = "90";
                await WaitFor(() => ReadEye(session, first).Direction == 90);
                model.Select(second);
                Assert.Equal("0", direction.Text);
                Assert.Equal("0", width.Text);
                width.Text = "40";
                await WaitFor(() => ReadEye(session, second).Width == 40);
                Assert.Equal(110, ReadEye(session, first).Width);

                model.ClearSelection();
                direction.Text = "77";
                await eye.WriteDnaAsync();
                await session.RefreshSnapshotAsync();
                Assert.Equal(90, ReadEye(session, first).Direction);
                Assert.Equal(0, ReadEye(session, second).Direction);
            }
            finally { player.Close(); }
        }, CancellationToken.None);
    }

    [Fact]
    public async Task EyeDesignerControlsApplyToTheLiveWorldAndKeepSetupDefaults()
    {
        var setup = new WorldSetupOptions
        {
            Backend = "Cpu", WorldWidth = 1000, WorldHeight = 1000, PopulationCapacity = 8, BrownianMotion = 0.75f,
            GlobalSettings = new() { Costs = new() { GlobalMultiplier = 2.5 } },
        };
        await using var session = new SimulationSession(new NativeEngineClient(setup));
        await session.ImportDnaAsync("start\nstop", [300, 300]);
        await session.ImportDnaAsync("start\nstop", [600, 300]);
        var bots = session.LatestSnapshot.Organisms.OrderBy(bot => bot.Position[0]).ToArray();
        var first = new OrganismKey(bots[0].Slot, bots[0].Generation);
        var second = new OrganismKey(bots[1].Slot, bots[1].Generation);
        await session.UpdateCostSettingsAsync(setup.GlobalSettings.Costs with { GlobalMultiplier = 3.0, Movement = 0.125 });
        var before = Assert.IsType<WorldConfiguration>(await session.ReadConfigurationAsync());
        Assert.Equal(0.75f, before.Environment.BrownianMotion);
        Assert.Equal(3.0, before.GlobalCostMultiplier);

        await DesktopTestApplication.Session.Dispatch(async () =>
        {
            var player = new MainWindow([], setup, new AuditEngineClientFactory(), new AuditStorageService());
            DesktopTestApplication.UseSession(player, session);
            try
            {
                player.Show();
                var model = Assert.IsType<MainWindowViewModel>(player.DataContext);
                model.ClearSelection();
                var eye = OpenEyeDesigner(player);
                var resetAim = EyeButton(eye, "Reset Aim");
                Assert.False(resetAim.IsEnabled);
                Click(EyeButton(eye, "Turn Off Brownian Motion"));
                Click(EyeButton(eye, "Disable Costs"));

                var checkpoint = await session.SaveAsync();
                using (var reopened = new NativeEngineClient(setup))
                {
                    reopened.Load(checkpoint);
                    var saved = Assert.IsType<WorldConfiguration>(reopened.ReadConfiguration());
                    Assert.Equal(0f, saved.Environment.BrownianMotion);
                    Assert.Equal(0.0, saved.GlobalCostMultiplier);
                    Assert.Equal(before.Costs with { GlobalMultiplier = 0.0 }, saved.Costs);
                }

                var next = Assert.IsType<WorldSetupOptions>(typeof(MainWindow)
                    .GetMethod("CreateNextWorldSetup", BindingFlags.Instance | BindingFlags.NonPublic)!.Invoke(player, null));
                Assert.Equal(setup.BrownianMotion, next.BrownianMotion);
                Assert.Equal(setup.GlobalSettings.Costs, next.GlobalSettings.Costs);

                await session.SetOrganismMemoryAsync(first.Slot, first.Generation, 19, 300);
                await session.SetOrganismMemoryAsync(second.Slot, second.Generation, 19, 700);
                await session.StepAsync();
                Assert.Equal(300, ReadRobot(session, first).Aim);
                Assert.Equal(700, ReadRobot(session, second).Aim);
                model.Select(first);
                Click(resetAim);
                await session.RefreshSnapshotAsync();
                Assert.Equal(0, ReadRobot(session, first).Memory.SingleOrDefault(cell => cell.Address == 19)?.Value ?? 0);
                Assert.Equal(300, ReadRobot(session, first).Aim);
                await session.StepAsync();
                Assert.Equal(0, ReadRobot(session, first).Aim);
                Assert.Equal(700, ReadRobot(session, second).Aim);
            }
            finally { player.Close(); }
        }, CancellationToken.None);
    }

    private static EyeSnapshot ReadEye(SimulationSession session, OrganismKey key) => session.LatestSnapshot.Organisms
        .Single(bot => bot.Slot == key.Slot && bot.Generation == key.Generation).Vision.Eyes[0];

    private static OrganismSnapshot ReadRobot(SimulationSession session, OrganismKey key) => session.LatestSnapshot.Organisms
        .Single(bot => bot.Slot == key.Slot && bot.Generation == key.Generation);

    private static Button EyeButton(EyeDesignerWindow window, string label) => Assert.Single(
        window.GetVisualDescendants().OfType<Button>(), button => Equals(button.Content, label));

    private static void Click(Button button)
    {
        Assert.True(button.IsEnabled);
        button.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
    }

    private static async Task WaitFor(Func<bool> condition)
    {
        for (var attempt = 0; attempt < 100 && !condition(); attempt++) await Task.Delay(25);
        Assert.True(condition(), "The eye memory write did not reach the simulation.");
    }

    private static EyeDesignerWindow OpenEyeDesigner(MainWindow player)
    {
        var command = player.FindControl<LegacyMenuBar>("HistoricMenuBar")!.FindCommandItem(LegacyCommandId.EditEyeDesigner)!;
        Assert.True(command.IsEnabled);
        command.RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));
        return Assert.Single(player.OwnedWindows.OfType<EyeDesignerWindow>());
    }

    private static string EyeFile(int firstDirection, int firstWidth) => "Cond\r\n*.robage 0 =\r\nStart\r\n"
        + $"{firstDirection} .eye1dir store\r\n{firstWidth} .eye1width store\r\n'\r\n"
        + string.Concat(Enumerable.Range(2, 8).Select(eye => $"0 .eye{eye}dir store\r\n0 .eye{eye}width store\r\n'\r\n"))
        + "Stop\r\n";

    private static void AssertRendered(Window window)
    {
        using var frame = window.CaptureRenderedFrame();
        Assert.NotNull(frame);
        Assert.True(frame.PixelSize.Width >= 400, $"Rendered width was {frame.PixelSize.Width}.");
        Assert.True(frame.PixelSize.Height >= 300, $"Rendered height was {frame.PixelSize.Height}.");
    }

    private static async Task<LegacyValueWindow> WaitForTagDialog(MainWindow window)
    {
        for (var attempts = 0; attempts < 100; attempts++)
        {
            if (window.OwnedWindows.OfType<LegacyValueWindow>().SingleOrDefault(candidate => candidate.IsVisible) is { } dialog)
                return dialog;
            await Task.Delay(20);
        }
        throw new Xunit.Sdk.XunitException("The AutoTag value dialog did not open.");
    }

    private static void CompleteTagDialog(LegacyValueWindow dialog, string input, string action)
    {
        var controls = dialog.GetVisualDescendants().ToArray();
        Assert.Single(controls.OfType<TextBox>()).Text = input;
        Assert.Single(controls.OfType<Button>(), button => Equals(button.Content, action))
            .RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
    }
}
