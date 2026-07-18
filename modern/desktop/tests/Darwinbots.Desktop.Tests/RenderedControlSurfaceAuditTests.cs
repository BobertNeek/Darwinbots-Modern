using System.Collections.Concurrent;
using System.Reflection;
using Avalonia;
using Avalonia.Controls;
using Avalonia.Controls.Primitives;
using Avalonia.Headless;
using Avalonia.Input;
using Avalonia.Interactivity;
using Avalonia.Media.Imaging;
using Avalonia.Threading;
using Avalonia.VisualTree;
using Darwinbots.Desktop.Controls;
using Darwinbots.Desktop.Core;
using Darwinbots.Desktop.Services;
using Darwinbots.Desktop.ViewModels;
using Darwinbots.Desktop.Views;
using Xunit;

namespace Darwinbots.Desktop.Tests;

public sealed class RenderedControlSurfaceAuditTests
{
    private static readonly IReadOnlyDictionary<string, string[]> Coverage =
        new Dictionary<string, string[]>(StringComparer.Ordinal)
        {
            [nameof(SetupRenderedControlsDispatchThroughVisualTree)] =
            [
                "setup.file", "setup.file.new", "setup.file.open", "setup.file.recover", "setup.file.exit",
                "setup.view", "setup.view.advanced", "setup.help", "setup.help.about",
                "setup.mode.starter", "setup.mode.zerobot", "setup.mode.zerobot_veg",
                "setup.species.name", "setup.species.vegetable", "setup.species.count", "setup.species.energy",
                "setup.species.mutation", "setup.species.mutation_label", "setup.species.reseed",
                "setup.species.remove", "setup.species.add", "setup.backend", "setup.world.width",
                "setup.world.height", "setup.population_cap", "setup.vegetable_cap", "setup.speed",
                "setup.sustenance", "setup.auto_progression", "setup.advanced_button", "setup.create",
                "setup.recover_button",
            ],
            [nameof(RuntimeRenderedControlsDispatchThroughVisualTree)] =
            [
                "live.run", "live.pause", "live.step", "live.turbo", "live.import", "live.reset",
                "live.speed", "live.backend", "live.save", "live.load", "live.toroidal", "live.render_waste",
                "live.add_obstacle", "live.add_teleporter", "live.remove_feature", "live.live_physics",
                "live.show_vision", "live.metabolism", "live.plant_energy", "live.feeding_body",
                "live.apply_energy", "live.advance_zerobot", "live.viewport.select", "live.viewport.drag",
                "live.dna.edit", "live.move", "live.clone", "live.reproduce", "live.kill", "live.follow",
                "live.expander.world", "live.expander.parameters", "live.expander.environment",
                "live.expander.physics", "live.expander.senses", "live.expander.energy",
                "live.expander.history", "live.expander.statistics", "live.expander.experiments",
            ],
            [nameof(AdvancedSettingsRenderedControlsRoundTripEveryField)] =
            [
                "advanced.metabolism", "advanced.vegetableenergy", "advanced.sunlight",
                "advanced.autospeciation", "advanced.speciationdistance", "advanced.toroidalworld",
                "advanced.maxvelocity", "advanced.movementefficiency", "advanced.gravityx", "advanced.gravityy",
                "advanced.surfacegravity", "advanced.staticfriction", "advanced.kineticfriction",
                "advanced.density", "advanced.viscosity", "advanced.elasticity", "advanced.drag",
                "advanced.brownian", "advanced.shotspeed", "advanced.shotrangemultiplier",
                "advanced.shotdecay", "advanced.energyshotsnodecay", "advanced.wasteshotsnodecay",
                "advanced.startchloroplasts", "advanced.maxplantenergy",
                "advanced.minimumchloroplastequivalents", "advanced.repopulationamount",
                "advanced.repopulationcooldown", "advanced.feedingtobody", "advanced.daytime",
                "advanced.daynightenabled", "advanced.cyclelength", "advanced.cancel", "advanced.apply",
            ],
            [nameof(DnaEditorRenderedControlsDispatchThroughVisualTree)] =
            ["dna.editor", "dna.save", "dna.clone", "dna.apply"],
        };

    [Fact]
    public void InventoryDefinesEveryCurrentInteractiveSurfaceExactlyOnce()
    {
        var ids = Coverage.Values.SelectMany(value => value).ToArray();

        Assert.Equal(109, ids.Length);
        Assert.Equal(ids.Length, ids.Distinct(StringComparer.Ordinal).Count());
        Assert.All(ids, id => Assert.False(string.IsNullOrWhiteSpace(id)));
    }

    [Fact]
    public async Task SetupRenderedControlsDispatchThroughVisualTree()
    {
        var storage = new AuditStorageService
        {
            DnaFiles = [new DesktopTextFile("Audit_Bot.txt", "start\n10 .up store\nstop")],
            PickedSimulationPath = Path.Combine(Path.GetTempPath(), "audit-open.db3s"),
            LatestAutosave = Path.Combine(Path.GetTempPath(), "audit-autosave.db3s"),
        };
        var headless = DesktopTestApplication.Session;
        SetupWindow? window = null;
        var created = new List<WorldSetupOptions>();

        await headless.Dispatch(() =>
        {
            window = new SetupWindow(storage);
            window.WorldCreated += created.Add;
            ShowAndCapture(window, 1280, 820, "setup-initial.png");

            OpenAndCloseMenu(window, "FileMenu");
            OpenAndCloseMenu(window, "ViewMenu");
            OpenAndCloseMenu(window, "HelpMenu");

            Click(window, Named<RadioButton>(window, "ZerobotMode"));
            Assert.Single(window.Species);
            Click(window, Named<RadioButton>(window, "ZerobotVegetableMode"));
            Assert.Equal(2, window.Species.Count);
            Click(window, Named<RadioButton>(window, "StarterMode"));
            Assert.Equal(2, window.Species.Count);

            var firstRow = window.Species[0];
            var rowControls = Named<ItemsControl>(window, "SpeciesList").GetVisualDescendants().ToArray();
            var name = rowControls.OfType<TextBox>().First(control => ReferenceEquals(control.DataContext, firstRow));
            name.Focus();
            name.SelectAll();
            window.KeyTextInput("Audit Alga");
            Assert.Equal("Audit Alga", firstRow.Name);

            var rowChecks = rowControls.OfType<CheckBox>().Where(control => ReferenceEquals(control.DataContext, firstRow)).ToArray();
            var vegetable = rowChecks.Single(control => Equals(control.Content, "VEG"));
            var vegetableBefore = firstRow.Vegetable;
            Click(window, vegetable);
            Assert.Equal(!vegetableBefore, firstRow.Vegetable);
            var reseed = rowChecks.Single(control => control.Content is null);
            var reseedBefore = firstRow.Reseed;
            Click(window, reseed);
            Assert.Equal(!reseedBefore, firstRow.Reseed);

            var rowNumbers = rowControls.OfType<NumericUpDown>().Where(control => ReferenceEquals(control.DataContext, firstRow)).ToArray();
            Assert.Equal(2, rowNumbers.Length);
            AssertKeyboardIncrement(window, rowNumbers[0]);
            AssertKeyboardIncrement(window, rowNumbers[1]);

            var slider = rowControls.OfType<Slider>().First(control => ReferenceEquals(control.DataContext, firstRow));
            DragSlider(window, slider, 0.20);
            var lowMutationRate = firstRow.MutationRate;
            DragSlider(window, slider, 0.80);
            var highMutationRate = firstRow.MutationRate;
            Assert.True(
                highMutationRate > lowMutationRate + 40,
                $"Mutation slider did not traverse its range: low={lowMutationRate:0.##}, high={highMutationRate:0.##}.");
            var mutationLabel = rowControls.OfType<TextBlock>()
                .First(control => ReferenceEquals(control.DataContext, firstRow)
                    && control.Text?.Contains('%', StringComparison.Ordinal) == true);
            Assert.Contains('%', mutationLabel.Text!);

            var remove = rowControls.OfType<Button>().Last(control => control.Tag is SetupSpeciesRow);
            Click(window, remove);
            Assert.Single(window.Species);
            Click(window, Named<Button>(window, "AddDnaButton"));
            Assert.Equal(2, window.Species.Count);
            Assert.Equal("Audit_Bot", window.Species[^1].Name);

            AssertKeyboardIncrement(window, Named<NumericUpDown>(window, "WorldWidth"));
            AssertKeyboardIncrement(window, Named<NumericUpDown>(window, "WorldHeight"));
            AssertKeyboardIncrement(window, Named<NumericUpDown>(window, "PopulationCap"));
            AssertKeyboardIncrement(window, Named<NumericUpDown>(window, "VegetablePopulationCap"));
            AssertComboSelection(window, Named<ComboBox>(window, "Backend"), 1);
            AssertComboSelection(window, Named<ComboBox>(window, "Speed"), 1);

            Click(window, Named<RadioButton>(window, "ZerobotMode"));
            var sustenance = Named<ComboBox>(window, "Sustenance");
            Assert.True(sustenance.IsEnabled);
            AssertComboSelection(window, sustenance, 1);
            var progression = Named<CheckBox>(window, "AutomaticProgression");
            var progressionBefore = progression.IsChecked;
            Click(window, progression);
            Assert.NotEqual(progressionBefore, progression.IsChecked);

            ActivateMenu(Named<MenuItem>(window, "OpenSaveMenuItem"));
            Assert.Equal(storage.PickedSimulationPath, created[^1].LoadSavePath);
            ActivateMenu(Named<MenuItem>(window, "RecoverMenuItem"));
            Assert.Equal(storage.LatestAutosave, created[^1].LoadSavePath);

            Click(window, Named<Button>(window, "AdvancedSettingsButton"));
            var advanced = Assert.Single(window.OwnedWindows.OfType<AdvancedSettingsWindow>());
            Click(advanced, Named<Button>(advanced, "ApplyButton"));
            Assert.Equal("ADVANCED SETTINGS APPLIED", Named<TextBlock>(window, "SetupStatus").Text);

            ActivateMenu(Named<MenuItem>(window, "AdvancedMenuItem"));
            advanced = Assert.Single(window.OwnedWindows.OfType<AdvancedSettingsWindow>());
            Click(advanced, Named<Button>(advanced, "CancelButton"));

            ActivateMenu(Named<MenuItem>(window, "AboutMenuItem"));
            var about = Assert.Single(window.OwnedWindows, value => value.Title == "About Darwinbots Modern");
            Click(about, about.GetVisualDescendants().OfType<Button>().Single());

            Click(window, Named<Button>(window, "CreateWorldButton"));
            Assert.Equal(StartingMode.Zerobots, created[^1].StartingMode);
            Click(window, Named<Button>(window, "RecoverButton"));
            Assert.Equal(storage.LatestAutosave, created[^1].LoadSavePath);

            ActivateMenu(Named<MenuItem>(window, "NewWorldMenuItem"));
            Assert.True(Named<RadioButton>(window, "StarterMode").IsChecked);
            Assert.Equal(3, Named<ComboBox>(window, "Speed").SelectedIndex);
            Capture(window, "setup-exercised.png");
            window.Close();

            var exitWindow = new SetupWindow(storage);
            ShowForInteraction(exitWindow, 1280, 820);
            ActivateMenu(Named<MenuItem>(exitWindow, "ExitMenuItem"));
            Assert.False(exitWindow.IsVisible);
        }, CancellationToken.None);
    }

    [Fact]
    public async Task AdvancedSettingsRenderedControlsRoundTripEveryField()
    {
        var headless = DesktopTestApplication.Session;
        await headless.Dispatch(() =>
        {
            var window = new AdvancedSettingsWindow(EnvironmentUpdate.Default);
            ShowAndCapture(window, 860, 760, "advanced-initial.png");

            string[] numericNames =
            [
                "Metabolism", "VegetableEnergy", "Sunlight", "SpeciationDistance", "MaxVelocity",
                "MovementEfficiency", "GravityX", "GravityY", "SurfaceGravity", "StaticFriction",
                "KineticFriction", "Density", "Viscosity", "Elasticity", "Drag", "Brownian",
                "ShotSpeed", "ShotRangeMultiplier", "ShotDecay", "StartChloroplasts", "MaxPlantEnergy",
                "MinimumChloroplastEquivalents", "RepopulationAmount", "RepopulationCooldown",
                "FeedingToBody", "CycleLength",
            ];
            foreach (var name in numericNames)
                AssertKeyboardIncrement(window, Named<NumericUpDown>(window, name), resetToMinimum: true);

            string[] checkNames =
            [
                "AutoSpeciation", "ToroidalWorld", "EnergyShotsNoDecay", "WasteShotsNoDecay",
                "Daytime", "DayNightEnabled",
            ];
            foreach (var name in checkNames)
            {
                var check = Named<CheckBox>(window, name);
                var before = check.IsChecked;
                Click(window, check);
                Assert.True(before != check.IsChecked, $"{name} did not toggle from {before}.");
            }

            var update = window.Update;
            Assert.NotEqual(EnvironmentUpdate.Default, update);
            Click(window, Named<Button>(window, "ApplyButton"));
            Assert.True(window.Accepted);
            Assert.False(window.IsVisible);

            var cancel = new AdvancedSettingsWindow(EnvironmentUpdate.Default);
            ShowForInteraction(cancel, 860, 760);
            Click(cancel, Named<Button>(cancel, "CancelButton"));
            Assert.False(cancel.Accepted);
            Assert.False(cancel.IsVisible);
        }, CancellationToken.None);
    }

    [Fact]
    public async Task DnaEditorRenderedControlsDispatchThroughVisualTree()
    {
        var engine = new AuditEngineClient();
        await using var simulation = new SimulationSession(engine);
        var storage = new AuditStorageService();
        var headless = DesktopTestApplication.Session;
        await headless.Dispatch(async () =>
        {
            var window = new DnaEditorWindow(simulation, engine.Organism, storage);
            ShowAndCapture(window, 760, 620, "dna-initial.png");
            await WaitFor(() => engine.Calls.Contains("ExportDna"));

            var editor = Named<TextBox>(window, "Editor");
            editor.Focus();
            editor.SelectAll();
            window.KeyTextInput("start\n20 .up store\nstop");
            Click(window, Named<Button>(window, "ApplyDnaButton"));
            await WaitFor(() => engine.Calls.Contains("ReplaceDna:3:2"));
            Click(window, Named<Button>(window, "ApplyCloneButton"));
            await WaitFor(() =>
                engine.Calls.Contains("Clone:3:2")
                && engine.Calls.Contains("ReplaceDna:40:0"));
            Click(window, Named<Button>(window, "SaveBotButton"));
            await WaitFor(() => storage.SavedDna is not null);
            Capture(window, "dna-exercised.png");

            Assert.Contains("20 .up store", storage.SavedDna!, StringComparison.Ordinal);
            window.Close();
            return 0;
        }, CancellationToken.None);

    }

    [Fact]
    public async Task RuntimeRenderedControlsDispatchThroughVisualTree()
    {
        var factory = new AuditEngineClientFactory();
        var storage = new AuditStorageService
        {
            DnaFiles = [new DesktopTextFile("Imported_Hunter.txt", "start\n30 .up store\nstop")],
            OpenedSimulation = [9, 8, 7, 6],
        };
        var setup = new WorldSetupOptions
        {
            TicksPerUpdate = 1,
            Species = [new SpeciesImport("Audit Species", "start\nstop", false, 0xff239ac0, 1, 1_000, 0, false, 1)],
        };
        var headless = DesktopTestApplication.Session;
        await headless.Dispatch(async () =>
        {
            var window = new MainWindow([], setup, factory, storage);
            ShowAndCapture(window, 1500, 900, "runtime-initial.png");
            await WaitFor(
                () => factory.Clients.TryPeek(out var client) && client.Calls.Contains("ImportSpecies:Audit Species"),
                () => factory.Clients.TryPeek(out var client) ? string.Join(", ", client.Calls) : "Engine client was not created.");
            var engine = factory.Clients.First();

            var viewModel = Field<MainWindowViewModel>(window, "_viewModel");
            Assert.Equal(3U, viewModel.SelectedSlot);

            Click(window, Named<Button>(window, "RunButton"));
            Assert.True(Field<DispatcherTimer>(window, "_runTimer").IsEnabled);
            Click(window, Named<Button>(window, "PauseButton"));
            Assert.False(Field<DispatcherTimer>(window, "_runTimer").IsEnabled);
            Click(window, Named<Button>(window, "StepButton"));

            Click(window, Named<Button>(window, "TurboButton"));
            Assert.False(Named<WorldViewport>(window, "Viewport").IsVisible);
            Click(window, Named<Button>(window, "TurboButton"));
            Assert.True(Named<WorldViewport>(window, "Viewport").IsVisible);
            Click(window, Named<Button>(window, "PauseButton"));

            AssertComboSelection(window, Named<ComboBox>(window, "RuntimeSpeed"), 2);
            Click(window, Named<Button>(window, "BackendButton"));
            Click(window, Named<Button>(window, "ImportButton"));
            Click(window, Named<Button>(window, "SaveButton"));
            Click(window, Named<Button>(window, "LoadButton"));

            ToggleAndAssert(window, Named<CheckBox>(window, "ToroidalWorldToggle"));
            ToggleAndAssert(window, Named<CheckBox>(window, "RenderWasteToggle"));

            ClickExpander(window, Named<Expander>(window, "ParametersExpander"));
            ClickExpander(window, Named<Expander>(window, "PhysicsExpander"));
            Click(window, Named<Button>(window, "LiveAdvancedButton"));
            var advanced = Assert.Single(window.OwnedWindows.OfType<AdvancedSettingsWindow>());
            Named<NumericUpDown>(advanced, "Brownian").Value = 4;
            Click(advanced, Named<Button>(advanced, "ApplyButton"));

            ClickExpander(window, Named<Expander>(window, "SensesExpander"));
            ToggleAndAssert(window, Named<CheckBox>(window, "ShowVisionToggle"));
            ClickExpander(window, Named<Expander>(window, "EnergyExpander"));
            AssertKeyboardIncrement(window, Named<NumericUpDown>(window, "LiveMetabolism"));
            AssertKeyboardIncrement(window, Named<NumericUpDown>(window, "LivePlantEnergy"));
            AssertKeyboardIncrement(window, Named<NumericUpDown>(window, "LiveFeedingToBody"));
            Click(window, Named<Button>(window, "ApplyEnergyButton"));

            ClickExpander(window, Named<Expander>(window, "HistoryExpander"));
            ClickExpander(window, Named<Expander>(window, "StatisticsExpander"));
            ClickExpander(window, Named<Expander>(window, "ExperimentsExpander"));
            Click(window, Named<Button>(window, "AdvanceZerobotButton"));

            var viewport = Named<WorldViewport>(window, "Viewport");
            ClickAt(window, viewport, 0.5, 0.5);
            Assert.Equal(3U, viewModel.SelectedSlot);

            Click(window, Named<Button>(window, "MoveButton"));
            ClickAt(window, viewport, 0.82, 0.78);
            DragAt(window, viewport, 0.5, 0.5, 0.57, 0.58);

            Click(window, Named<Button>(window, "AddObstacleButton"));
            ClickAt(window, viewport, 0.30, 0.30);
            ClickAt(window, viewport, 0.40, 0.40);
            Click(window, Named<Button>(window, "AddTeleporterButton"));
            ClickAt(window, viewport, 0.60, 0.60);
            ClickAt(window, viewport, 0.70, 0.70);

            ClickAt(window, viewport, 0.09375, 0.125);
            Click(window, Named<Button>(window, "RemoveFeatureButton"));

            Click(window, Named<Button>(window, "CloneButton"));
            Click(window, Named<Button>(window, "ReproduceButton"));
            Click(window, Named<Button>(window, "FollowButton"));
            Click(window, Named<Button>(window, "FollowButton"));

            Click(window, Named<Button>(window, "EditDnaButton"));
            var editor = Assert.Single(window.OwnedWindows.OfType<DnaEditorWindow>());
            editor.Close();

            Click(window, Named<Button>(window, "KillButton"));
            ClickExpander(window, Named<Expander>(window, "EnvironmentExpander"));
            ClickExpander(window, Named<Expander>(window, "WorldExpander"));

            await WaitFor(() =>
                engine.Calls.Any(call => call.StartsWith("Tick:", StringComparison.Ordinal))
                && engine.Calls.Contains("Switch:Gpu")
                && engine.Calls.Contains("ImportSpecies:Imported_Hunter")
                && engine.Calls.Contains("Save")
                && engine.Calls.Contains("Load")
                && engine.Calls.Any(call => call.StartsWith("Move:3:2", StringComparison.Ordinal))
                && engine.Calls.Contains("AddObstacle")
                && engine.Calls.Contains("AddTeleporter")
                && engine.Calls.Contains("RemoveObstacle:7")
                && engine.Calls.Contains("Clone:3:2")
                && engine.Calls.Contains("Reproduce:3:2")
                && engine.Calls.Contains("Remove:3:2")
                && engine.EnvironmentUpdates.Count >= 3
                && storage.SavedSimulation is not null,
                () => string.Join(", ", engine.Calls));
            Capture(window, "runtime-exercised.png");

            Click(window, Named<Button>(window, "ResetButton"));
            await WaitFor(() => factory.Clients.Count >= 2);
            return 0;
        }, CancellationToken.None);
    }

    private static void ShowAndCapture(Window window, double width, double height, string evidenceName)
    {
        ShowForInteraction(window, width, height);
        Capture(window, evidenceName);
    }

    private static void ShowForInteraction(Window window, double width, double height)
    {
        window.WindowState = WindowState.Normal;
        window.Width = width;
        window.Height = height;
        window.Show();
        AvaloniaHeadlessPlatform.ForceRenderTimerTick();
    }

    private static void Capture(Window window, string evidenceName)
    {
        AvaloniaHeadlessPlatform.ForceRenderTimerTick();
        using var frame = window.CaptureRenderedFrame();
        Assert.NotNull(frame);
        Assert.True(frame.PixelSize.Width >= 400, $"Rendered width was {frame.PixelSize.Width}.");
        Assert.True(frame.PixelSize.Height >= 300, $"Rendered height was {frame.PixelSize.Height}.");
        var directory = Environment.GetEnvironmentVariable("DARWINBOTS_GUI_AUDIT_DIR");
        if (string.IsNullOrWhiteSpace(directory))
            directory = Path.Combine(Path.GetTempPath(), "darwinbots-modern-control-audit");
        Directory.CreateDirectory(directory);
        using var stream = File.Create(Path.Combine(directory, evidenceName));
        frame.Save(stream, new PngBitmapEncoderOptions());
    }

    private static T Named<T>(Control root, string name) where T : Control =>
        root.FindControl<T>(name) ?? throw new Xunit.Sdk.XunitException($"Missing {typeof(T).Name} named {name}.");

    private static void Click(Window window, Control control)
    {
        StabilizeForInput(window, control);
        var local = new Point(Math.Max(1, control.Bounds.Width / 2), Math.Max(1, control.Bounds.Height / 2));
        var point = control.TranslatePoint(local, window);
        Assert.True(point.HasValue, $"{control.Name ?? control.GetType().Name} was not attached to the rendered window.");
        AssertPointInsideWindow(window, point.Value, control);
        window.MouseDown(point.Value, MouseButton.Left, RawInputModifiers.None);
        window.MouseUp(point.Value, MouseButton.Left, RawInputModifiers.None);
        AvaloniaHeadlessPlatform.ForceRenderTimerTick();
    }

    private static void ClickAt(Window window, Control control, double fractionX, double fractionY)
    {
        StabilizeForInput(window, control);
        var local = new Point(control.Bounds.Width * fractionX, control.Bounds.Height * fractionY);
        var point = control.TranslatePoint(local, window);
        Assert.True(point.HasValue);
        AssertPointInsideWindow(window, point.Value, control);
        window.MouseDown(point.Value, MouseButton.Left, RawInputModifiers.None);
        window.MouseUp(point.Value, MouseButton.Left, RawInputModifiers.None);
    }

    private static void DragAt(
        Window window,
        Control control,
        double startFractionX,
        double startFractionY,
        double endFractionX,
        double endFractionY)
    {
        StabilizeForInput(window, control);
        var start = control.TranslatePoint(
            new Point(control.Bounds.Width * startFractionX, control.Bounds.Height * startFractionY), window);
        var end = control.TranslatePoint(
            new Point(control.Bounds.Width * endFractionX, control.Bounds.Height * endFractionY), window);
        Assert.True(start.HasValue && end.HasValue);
        AssertPointInsideWindow(window, start.Value, control);
        AssertPointInsideWindow(window, end.Value, control);
        window.MouseDown(start.Value, MouseButton.Left, RawInputModifiers.None);
        window.MouseMove(end.Value, RawInputModifiers.LeftMouseButton);
        window.MouseUp(end.Value, MouseButton.Left, RawInputModifiers.None);
    }

    private static void DragSlider(Window window, Slider slider, double fraction)
    {
        StabilizeForInput(window, slider);
        var start = slider.TranslatePoint(new Point(2, slider.Bounds.Height / 2), window);
        var end = slider.TranslatePoint(new Point(slider.Bounds.Width * fraction, slider.Bounds.Height / 2), window);
        Assert.True(start.HasValue && end.HasValue);
        AssertPointInsideWindow(window, start.Value, slider);
        AssertPointInsideWindow(window, end.Value, slider);
        window.MouseDown(start.Value, MouseButton.Left, RawInputModifiers.None);
        window.MouseMove(end.Value, RawInputModifiers.LeftMouseButton);
        window.MouseUp(end.Value, MouseButton.Left, RawInputModifiers.None);
    }

    private static void ClickExpander(Window window, Expander expander)
    {
        var header = expander.GetVisualDescendants().OfType<ToggleButton>().First();
        StabilizeForInput(window, header);
        var before = expander.IsExpanded;
        var point = header.TranslatePoint(
            new Point(Math.Max(1, header.Bounds.Width / 2), Math.Max(1, header.Bounds.Height / 2)),
            window);
        Assert.True(point.HasValue);
        AssertPointInsideWindow(window, point.Value, header);
        window.MouseDown(point.Value, MouseButton.Left, RawInputModifiers.None);
        window.MouseUp(point.Value, MouseButton.Left, RawInputModifiers.None);
        Assert.NotEqual(before, expander.IsExpanded);
    }

    private static void OpenAndCloseMenu(Window window, string name)
    {
        var menu = Named<MenuItem>(window, name);
        Click(window, menu);
        Assert.True(menu.IsSubMenuOpen);
        menu.IsSubMenuOpen = false;
    }

    private static void ActivateMenu(MenuItem item) =>
        item.RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));

    private static void ToggleAndAssert(Window window, CheckBox check)
    {
        var before = check.IsChecked;
        Click(window, check);
        Assert.NotEqual(before, check.IsChecked);
    }

    private static void AssertKeyboardIncrement(Window window, NumericUpDown numeric, bool resetToMinimum = false)
    {
        StabilizeForInput(window, numeric);
        if (resetToMinimum) numeric.Value = numeric.Minimum;
        var before = numeric.Value;
        numeric.Focus();
        window.KeyPress(Key.Up, RawInputModifiers.None, PhysicalKey.ArrowUp, null);
        window.KeyRelease(Key.Up, RawInputModifiers.None, PhysicalKey.ArrowUp, null);
        Assert.True(numeric.Value > before, $"{numeric.Name} did not increment from {before}.");
    }

    private static void AssertComboSelection(Window window, ComboBox combo, int targetIndex)
    {
        StabilizeForInput(window, combo);
        combo.SelectedIndex = Math.Max(0, targetIndex - 1);
        combo.Focus();
        window.KeyPress(Key.Down, RawInputModifiers.None, PhysicalKey.ArrowDown, null);
        window.KeyRelease(Key.Down, RawInputModifiers.None, PhysicalKey.ArrowDown, null);
        Assert.Equal(targetIndex, combo.SelectedIndex);
    }

    private static void StabilizeForInput(Window window, Control control)
    {
        control.BringIntoView();
        Dispatcher.UIThread.RunJobs();
        AvaloniaHeadlessPlatform.ForceRenderTimerTick();
        Dispatcher.UIThread.RunJobs();
        Assert.True(window.IsVisible, $"{window.Title} was not visible while targeting {control.Name ?? control.GetType().Name}.");
    }

    private static void AssertPointInsideWindow(Window window, Point point, Control control)
    {
        Assert.InRange(point.X, 0, window.ClientSize.Width);
        Assert.InRange(point.Y, 0, window.ClientSize.Height);
    }

    private static T Field<T>(object target, string name) =>
        (T)target.GetType().GetField(name, BindingFlags.Instance | BindingFlags.NonPublic)!.GetValue(target)!;

    private static async Task WaitFor(Func<bool> condition, Func<string>? details = null)
    {
        var deadline = DateTime.UtcNow + TimeSpan.FromSeconds(8);
        while (!condition() && DateTime.UtcNow < deadline) await Task.Delay(10);
        Assert.True(condition(), $"Timed out waiting for rendered control dispatch. {details?.Invoke()}");
    }
}

internal sealed class AuditStorageService : IDesktopStorageService
{
    public IReadOnlyList<DesktopTextFile> DnaFiles { get; init; } = [];
    public string? PickedSimulationPath { get; init; }
    public byte[]? OpenedSimulation { get; init; }
    public string? LatestAutosave { get; init; }
    public byte[]? SavedSimulation { get; private set; }
    public string? SavedDna { get; private set; }
    public byte[]? Autosave { get; private set; }

    public Task<IReadOnlyList<DesktopTextFile>> OpenDnaFilesAsync(TopLevel owner) => Task.FromResult(DnaFiles);
    public Task<string?> PickSimulationPathAsync(TopLevel owner) => Task.FromResult(PickedSimulationPath);
    public Task<byte[]?> OpenSimulationAsync(TopLevel owner) => Task.FromResult(OpenedSimulation);

    public Task<bool> SaveSimulationAsync(TopLevel owner, ReadOnlyMemory<byte> save)
    {
        SavedSimulation = save.ToArray();
        return Task.FromResult(true);
    }

    public Task<bool> SaveDnaAsync(TopLevel owner, string suggestedFileName, string dna)
    {
        SavedDna = dna;
        return Task.FromResult(true);
    }

    public string? FindLatestAutosave() => LatestAutosave;

    public Task SaveAutosaveAsync(ReadOnlyMemory<byte> save)
    {
        Autosave = save.ToArray();
        return Task.CompletedTask;
    }
}

internal sealed class AuditEngineClientFactory : IEngineClientFactory
{
    public ConcurrentQueue<AuditEngineClient> Clients { get; } = [];

    public IEngineClient Create(WorldSetupOptions setup)
    {
        var client = new AuditEngineClient();
        Clients.Enqueue(client);
        return client;
    }
}

internal sealed class AuditEngineClient : IEngineClient
{
    private ulong _tick;
    private string _backend = "CPU";
    private EngineSnapshot _snapshot;

    public AuditEngineClient()
    {
        Organism = new OrganismSnapshot(3, 2, [8_000f, 6_000f], [0f, 0f], 1_000, 10, Species: 1);
        _snapshot = new EngineSnapshot(0, 1, _backend, [Organism])
        {
            Species =
            [
                new SpeciesSnapshot("Unassigned", false, 0xff858982, 0, false),
                new SpeciesSnapshot("Audit Species", false, 0xff239ac0, 0, false),
            ],
            RenderInstances =
            [
                new RenderInstanceSnapshot(3, [8_000f, 6_000f], 12f, 0xff239ac0) { Generation = 2 },
            ],
            Obstacles = [new ObstacleSnapshot(7, [1_000f, 1_000f], [2_000f, 2_000f])],
            Teleporters = [new TeleporterSnapshot(8, [14_000f, 10_000f], 180f, [2_000f, 2_000f])],
        };
    }

    public OrganismSnapshot Organism { get; }
    public ConcurrentQueue<string> Calls { get; } = [];
    public ConcurrentQueue<EnvironmentUpdate> EnvironmentUpdates { get; } = [];
    public string Backend => _backend;

    public EngineCapabilities Capabilities() => new(1, _backend, true, null);
    public void Tick(uint count = 1) { _tick += count; Calls.Enqueue($"Tick:{count}"); }
    public DnaImportReport ImportDna(string dna, float[]? position = null) { Calls.Enqueue("ImportDna"); return DnaImportReport.Compatible; }
    public DnaImportReport ImportSpecies(SpeciesImport species) { Calls.Enqueue($"ImportSpecies:{species.Name}"); return DnaImportReport.Compatible; }
    public void Remove(uint slot, uint generation) => Calls.Enqueue($"Remove:{slot}:{generation}");
    public void Move(uint slot, uint generation, float[] position) => Calls.Enqueue($"Move:{slot}:{generation}:{position[0]:0}:{position[1]:0}");
    public OrganismKey CloneOrganism(uint slot, uint generation, float[] position) { Calls.Enqueue($"Clone:{slot}:{generation}"); return new OrganismKey(40, 0); }
    public void ReplaceDna(uint slot, uint generation, string dna) => Calls.Enqueue($"ReplaceDna:{slot}:{generation}");
    public string ExportDna(uint slot, uint generation) { Calls.Enqueue("ExportDna"); return "start\nstop"; }
    public OrganismKey Reproduce(uint firstSlot, uint firstGeneration, uint? secondSlot, uint? secondGeneration, float[] position)
    {
        Calls.Enqueue($"Reproduce:{firstSlot}:{firstGeneration}");
        return new OrganismKey(41, 0);
    }
    public void SwitchBackend(string backend) { _backend = backend.ToUpperInvariant(); Calls.Enqueue($"Switch:{backend}"); }
    public void AddObstacle(ObstacleSnapshot obstacle) { Calls.Enqueue("AddObstacle"); _snapshot = _snapshot with { Obstacles = [.. _snapshot.Obstacles, obstacle] }; }
    public void RemoveObstacle(uint id) { Calls.Enqueue($"RemoveObstacle:{id}"); _snapshot = _snapshot with { Obstacles = _snapshot.Obstacles.Where(value => value.Id != id).ToArray() }; }
    public void AddTeleporter(TeleporterSnapshot teleporter) { Calls.Enqueue("AddTeleporter"); _snapshot = _snapshot with { Teleporters = [.. _snapshot.Teleporters, teleporter] }; }
    public void RemoveTeleporter(uint id) { Calls.Enqueue($"RemoveTeleporter:{id}"); _snapshot = _snapshot with { Teleporters = _snapshot.Teleporters.Where(value => value.Id != id).ToArray() }; }
    public void SetBrownianMotion(float value) => Calls.Enqueue($"Brownian:{value}");
    public void UpdateEnvironment(EnvironmentUpdate update) { EnvironmentUpdates.Enqueue(update); Calls.Enqueue("Environment"); }
    public byte[] Save() { Calls.Enqueue("Save"); return [1, 2, 3, 4]; }
    public void Load(byte[] save) => Calls.Enqueue("Load");
    public EngineSnapshot Snapshot() => _snapshot with { Tick = _tick, Backend = _backend };
    public void Dispose() => Calls.Enqueue("Dispose");
}
