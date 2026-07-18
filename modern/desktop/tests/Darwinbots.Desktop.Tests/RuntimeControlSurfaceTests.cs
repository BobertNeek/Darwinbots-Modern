using System.Collections.Concurrent;
using System.Reflection;
using Avalonia.Controls;
using Avalonia.Headless;
using Avalonia.Interactivity;
using Avalonia.Threading;
using Darwinbots.Desktop.Controls;
using Darwinbots.Desktop.Core;
using Darwinbots.Desktop.ViewModels;
using Darwinbots.Desktop.Views;
using Xunit;

namespace Darwinbots.Desktop.Tests;

public sealed class RuntimeControlSurfaceTests
{
    [Fact]
    public async Task TransportSpeedDisplayPlacementAndManualSelectionControlsChangeLiveUiState()
    {
        using var headless = HeadlessUnitTestSession.StartNew(typeof(App));
        await headless.Dispatch(() =>
        {
            var window = new MainWindow([], new WorldSetupOptions { TicksPerUpdate = 1 });
            var viewModel = Field<MainWindowViewModel>(window, "_viewModel");
            var timer = Field<DispatcherTimer>(window, "_runTimer");
            var viewport = window.FindControl<WorldViewport>("Viewport")!;

            Invoke(window, "Run_Click");
            Assert.True(timer.IsEnabled);
            Assert.Equal("RUNNING", viewModel.Status);
            Invoke(window, "Pause_Click");
            Assert.False(timer.IsEnabled);
            Assert.Equal("PAUSED", viewModel.Status);

            var speed = window.FindControl<ComboBox>("RuntimeSpeed")!;
            speed.SelectedIndex = 1;
            Assert.Equal(5U, Field<uint>(window, "_ticksPerUpdate"));
            speed.SelectedIndex = 2;
            Assert.Equal(20U, Field<uint>(window, "_ticksPerUpdate"));
            speed.SelectedIndex = 3;
            Assert.True(Field<bool>(window, "_maximumSpeed"));

            Invoke(window, "Turbo_Click");
            Assert.False(viewport.IsVisible);
            Assert.Equal("SHOW WORLD", window.FindControl<Button>("TurboButton")!.Content);
            Assert.Equal("TURBO · RENDERING SUSPENDED", viewModel.Status);
            Invoke(window, "Turbo_Click");
            Assert.True(viewport.IsVisible);
            Assert.Equal("TURBO", window.FindControl<Button>("TurboButton")!.Content);
            Invoke(window, "Pause_Click");

            window.FindControl<CheckBox>("RenderWasteToggle")!.IsChecked = false;
            Assert.False(viewport.RenderWaste);
            Assert.Equal("WASTE RENDERING OFF", viewModel.Status);
            window.FindControl<CheckBox>("ShowVisionToggle")!.IsChecked = false;
            Assert.False(viewport.ShowSelectedVision);

            Invoke(window, "AddObstacle_Click");
            Assert.Equal("Obstacle", Field<object>(window, "_environmentPlacement").ToString());
            Assert.Equal("OBSTACLE · CLICK FIRST CORNER", viewModel.Status);
            Invoke(window, "AddTeleporter_Click");
            Assert.Equal("Teleporter", Field<object>(window, "_environmentPlacement").ToString());
            Assert.Equal("TELEPORTER · CLICK SOURCE", viewModel.Status);

            viewModel.Update(RecordingEngineClient.CreateSnapshot());
            Invoke(window, "Move_Click");
            Assert.Equal(new OrganismKey(3, 2), Field<OrganismKey?>(window, "_moveOrganism"));
            Invoke(window, "Follow_Click");
            Assert.True(Field<bool>(window, "_following"));
            Assert.Equal("FOLLOWING 3:2", viewModel.Status);
            Invoke(window, "Follow_Click");
            Assert.False(Field<bool>(window, "_following"));
            Invoke(window, "AdvanceZerobot_Click");
            Assert.Equal("ZEROBOT PROGRESSION IS NOT ACTIVE IN THIS WORLD", viewModel.Status);
        }, CancellationToken.None);
    }

    [Fact]
    public async Task RuntimeButtonsDispatchTheExpectedEngineCommandsAndDnaTargets()
    {
        var engine = new RecordingEngineClient();
        await using var simulation = new SimulationSession(engine);
        using var headless = HeadlessUnitTestSession.StartNew(typeof(App));
        MainWindow? window = null;

        await headless.Dispatch(() =>
        {
            window = new MainWindow([], new WorldSetupOptions());
            SetField(window, "_session", simulation);
            Field<MainWindowViewModel>(window, "_viewModel").Update(RecordingEngineClient.CreateSnapshot());

            Invoke(window, "Step_Click");
            Invoke(window, "Clone_Click");
            Invoke(window, "Reproduce_Click");
            Invoke(window, "Kill_Click");
            Invoke(window, "SwitchBackend_Click");

            window.FindControl<NumericUpDown>("LiveMetabolism")!.Value = 7;
            window.FindControl<NumericUpDown>("LivePlantEnergy")!.Value = 55;
            window.FindControl<NumericUpDown>("LiveFeedingToBody")!.Value = 0.25m;
            Invoke(window, "ApplyEnergy_Click");
            window.FindControl<CheckBox>("ToroidalWorldToggle")!.IsChecked = false;

            var editor = new DnaEditorWindow(simulation, RecordingEngineClient.Organism);
            editor.FindControl<TextBox>("Editor")!.Text = "start\n10 .up store\nstop";
            Invoke(editor, "Apply_Click");
            Invoke(editor, "Clone_Click");
        }, CancellationToken.None);

        await WaitFor(() =>
            engine.Calls.Any(call => call.StartsWith("Tick:", StringComparison.Ordinal))
            && engine.Calls.Any(call => call.StartsWith("Clone:3:2", StringComparison.Ordinal))
            && engine.Calls.Any(call => call.StartsWith("Reproduce:3:2", StringComparison.Ordinal))
            && engine.Calls.Contains("Remove:3:2")
            && engine.Calls.Contains("Switch:Gpu")
            && engine.EnvironmentUpdates.Count >= 2
            && engine.DnaReplacements.Any(value => value.Slot == 3 && value.Generation == 2)
            && engine.DnaReplacements.Any(value => value.Slot == 40 && value.Generation == 0));

        Assert.Contains(engine.EnvironmentUpdates, update =>
            update.MetabolismCost == 7
            && update.Vegetation.MaxEnergyPerTick == 55
            && Math.Abs(update.Vegetation.FeedingToBody - 0.25f) < 0.001f);
        Assert.Contains(engine.EnvironmentUpdates, update => !update.ToroidalWorld);
    }

    [Fact]
    public async Task ObstaclePlacementDispatchesTheDb2TwoCornerWorkflow()
    {
        var engine = new RecordingEngineClient();
        await using var simulation = new SimulationSession(engine);
        using var headless = HeadlessUnitTestSession.StartNew(typeof(App));

        await headless.Dispatch(() =>
        {
            var window = RuntimeWindow(simulation);
            var viewport = window.FindControl<WorldViewport>("Viewport")!;
            Invoke(window, "AddObstacle_Click");
            Raise<Action<float[]>>(viewport, "WorldClicked")([10f, 20f]);
            Raise<Action<float[]>>(viewport, "WorldClicked")([40f, 60f]);
        }, CancellationToken.None);
        await WaitFor(() => engine.Calls.Contains("AddObstacle:1"));
    }

    [Fact]
    public async Task TeleporterPlacementDispatchesTheDb2SourceDestinationWorkflow()
    {
        var engine = new RecordingEngineClient();
        await using var simulation = new SimulationSession(engine);
        using var headless = HeadlessUnitTestSession.StartNew(typeof(App));

        await headless.Dispatch(() =>
        {
            var window = RuntimeWindow(simulation);
            var viewport = window.FindControl<WorldViewport>("Viewport")!;
            Invoke(window, "AddTeleporter_Click");
            Raise<Action<float[]>>(viewport, "WorldClicked")([100f, 200f]);
            Raise<Action<float[]>>(viewport, "WorldClicked")([300f, 400f]);
        }, CancellationToken.None);
        await WaitFor(() => engine.Calls.Contains("AddTeleporter:1"));
    }

    [Fact]
    public async Task RemoveMoveAndDragDispatchStableOrganismAndFeatureIds()
    {
        var engine = new RecordingEngineClient();
        await using var simulation = new SimulationSession(engine);
        using var headless = HeadlessUnitTestSession.StartNew(typeof(App));

        await headless.Dispatch(() =>
        {
            var window = RuntimeWindow(simulation);
            SetField(window, "_selectedFeature", new WorldFeatureSelection(WorldFeatureKind.Obstacle, 1));
            Invoke(window, "RemoveFeature_Click");
            Invoke(window, "Move_Click");
            var viewport = window.FindControl<WorldViewport>("Viewport")!;
            Raise<Action<float[]>>(viewport, "WorldClicked")([500f, 600f]);
            Raise<Action<uint, float[]>>(viewport, "OrganismDragCompleted")(3, [700f, 800f]);
        }, CancellationToken.None);
        await WaitFor(() =>
            engine.Calls.Contains("RemoveObstacle:1")
            && engine.Calls.Contains("Move:3:2:500:600")
            && engine.Calls.Contains("Move:3:2:700:800"),
            () => string.Join(", ", engine.Calls));
    }

    private static MainWindow RuntimeWindow(SimulationSession simulation)
    {
        var window = new MainWindow([], new WorldSetupOptions());
        SetField(window, "_session", simulation);
        Field<MainWindowViewModel>(window, "_viewModel").Update(RecordingEngineClient.CreateSnapshot());
        return window;
    }

    private static void Invoke(object target, string method, object? sender = null) =>
        target.GetType().GetMethod(method, BindingFlags.Instance | BindingFlags.NonPublic)!
            .Invoke(target, [sender, new RoutedEventArgs()]);

    private static T Field<T>(object target, string name) =>
        (T)target.GetType().GetField(name, BindingFlags.Instance | BindingFlags.NonPublic)!.GetValue(target)!;

    private static void SetField(object target, string name, object? value) =>
        target.GetType().GetField(name, BindingFlags.Instance | BindingFlags.NonPublic)!.SetValue(target, value);

    private static T Raise<T>(object target, string eventName) where T : Delegate =>
        (T)target.GetType().GetField(eventName, BindingFlags.Instance | BindingFlags.NonPublic)!.GetValue(target)!;

    private static async Task WaitFor(Func<bool> condition, Func<string>? details = null)
    {
        var deadline = DateTime.UtcNow + TimeSpan.FromSeconds(5);
        while (!condition() && DateTime.UtcNow < deadline) await Task.Delay(10);
        Assert.True(condition(), $"Timed out waiting for the serialized UI command batch. Calls: {details?.Invoke()}");
    }
}

internal sealed class RecordingEngineClient : IEngineClient
{
    public static readonly OrganismSnapshot Organism = new(
        3, 2, [100f, 200f], [0f, 0f], 1_000, 10, Species: 1);

    public ConcurrentQueue<string> Calls { get; } = [];
    public ConcurrentQueue<EnvironmentUpdate> EnvironmentUpdates { get; } = [];
    public ConcurrentQueue<(uint Slot, uint Generation, string Dna)> DnaReplacements { get; } = [];
    public string Backend { get; private set; } = "CPU";
    private ulong _tick;

    public static EngineSnapshot CreateSnapshot() => new(0, 1, "CPU", [Organism])
    {
        Species =
        [
            new SpeciesSnapshot("Unassigned", false, 0xff858982, 0, false),
            new SpeciesSnapshot("Animal Minimalis", false, 0xff239ac0, 0, false),
        ],
    };

    public EngineCapabilities Capabilities() => new(1, Backend, true, null);
    public void Tick(uint count = 1) { _tick += count; Calls.Enqueue($"Tick:{count}"); }
    public DnaImportReport ImportDna(string dna, float[]? position = null) { Calls.Enqueue("ImportDna"); return new([]); }
    public DnaImportReport ImportSpecies(SpeciesImport species) { Calls.Enqueue("ImportSpecies"); return new([]); }
    public void Remove(uint slot, uint generation) => Calls.Enqueue($"Remove:{slot}:{generation}");
    public void Move(uint slot, uint generation, float[] position) => Calls.Enqueue($"Move:{slot}:{generation}:{position[0]:0}:{position[1]:0}");
    public OrganismKey CloneOrganism(uint slot, uint generation, float[] position)
    {
        Calls.Enqueue($"Clone:{slot}:{generation}");
        return new OrganismKey(40, 0);
    }
    public void ReplaceDna(uint slot, uint generation, string dna)
    {
        Calls.Enqueue($"ReplaceDna:{slot}:{generation}");
        DnaReplacements.Enqueue((slot, generation, dna));
    }
    public string ExportDna(uint slot, uint generation) => "start\nstop";
    public OrganismKey Reproduce(uint firstSlot, uint firstGeneration, uint? secondSlot, uint? secondGeneration, float[] position)
    {
        Calls.Enqueue($"Reproduce:{firstSlot}:{firstGeneration}:{secondSlot?.ToString() ?? "none"}");
        return new OrganismKey(41, 0);
    }
    public void SwitchBackend(string backend) { Backend = backend.ToUpperInvariant(); Calls.Enqueue($"Switch:{backend}"); }
    public void AddObstacle(ObstacleSnapshot obstacle) => Calls.Enqueue($"AddObstacle:{obstacle.Id}");
    public void RemoveObstacle(uint id) => Calls.Enqueue($"RemoveObstacle:{id}");
    public void AddTeleporter(TeleporterSnapshot teleporter) => Calls.Enqueue($"AddTeleporter:{teleporter.Id}");
    public void RemoveTeleporter(uint id) => Calls.Enqueue($"RemoveTeleporter:{id}");
    public void SetBrownianMotion(float value) => Calls.Enqueue($"Brownian:{value}");
    public void UpdateEnvironment(EnvironmentUpdate update) { EnvironmentUpdates.Enqueue(update); Calls.Enqueue("Environment"); }
    public byte[] Save() => BitConverter.GetBytes(_tick);
    public void Load(byte[] save) => _tick = BitConverter.ToUInt64(save);
    public EngineSnapshot Snapshot()
    {
        var snapshot = CreateSnapshot();
        return new EngineSnapshot(_tick, 1, Backend, [Organism]) { Species = snapshot.Species };
    }
    public void Dispose() { }
}
