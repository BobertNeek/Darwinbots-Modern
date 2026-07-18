using Avalonia.Controls;
using Avalonia.Input;
using Avalonia.Interactivity;
using Avalonia.Threading;
using Darwinbots.Desktop.Core;
using Darwinbots.Desktop.Controls;
using Darwinbots.Desktop.Services;
using Darwinbots.Desktop.ViewModels;
using System.Diagnostics;

namespace Darwinbots.Desktop.Views;

public sealed partial class MainWindow : Window
{
    private void MainWindow_KeyDown(object? sender, KeyEventArgs e)
    {
        var routed = new RoutedEventArgs();
        if (e.Key == Key.F5) Run_Click(sender, routed);
        else if (e.Key == Key.F6) Pause_Click(sender, routed);
        else if (e.Key == Key.F7) Step_Click(sender, routed);
        else if (e.Key == Key.F8) Turbo_Click(sender, routed);
        else if (e.Key == Key.F9)
            RuntimeSpeed.SelectedIndex = (RuntimeSpeed.SelectedIndex + 1) % 4;
        else if (e.KeyModifiers.HasFlag(KeyModifiers.Control))
        {
            switch (e.Key)
            {
                case Key.B: SwitchBackend_Click(sender, routed); break;
                case Key.I: Import_Click(sender, routed); break;
                case Key.S: Save_Click(sender, routed); break;
                case Key.L: Load_Click(sender, routed); break;
                case Key.R: Reset_Click(sender, routed); break;
                case Key.E: EditDna_Click(sender, routed); break;
                case Key.OemComma: LiveAdvanced_Click(sender, routed); break;
                case Key.D1: AddObstacle_Click(sender, routed); break;
                case Key.D2: AddTeleporter_Click(sender, routed); break;
                case Key.Delete: RemoveFeature_Click(sender, routed); break;
                default: return;
            }
        }
        else if (e.KeyModifiers.HasFlag(KeyModifiers.Alt))
        {
            switch (e.Key)
            {
                case Key.M: Move_Click(sender, routed); break;
                case Key.C: Clone_Click(sender, routed); break;
                case Key.R: Reproduce_Click(sender, routed); break;
                case Key.K: Kill_Click(sender, routed); break;
                case Key.F: Follow_Click(sender, routed); break;
                default: return;
            }
        }
        else return;
        e.Handled = true;
    }

    private readonly MainWindowViewModel _viewModel = new();
    private SimulationSession? _session;
    private readonly DispatcherTimer _runTimer;
    private readonly DispatcherTimer _autosaveTimer;
    private readonly IReadOnlyList<string> _startupBots;
    private readonly WorldSetupOptions _setup;
    private readonly IEngineClientFactory _engineFactory;
    private readonly IDesktopStorageService _storage;
    private bool _tickInFlight;
    private OrganismKey? _moveOrganism;
    private bool _turbo;
    private EnvironmentPlacement _environmentPlacement;
    private float[]? _placementOrigin;
    private WorldFeatureSelection? _selectedFeature;
    private readonly ZerobotProgressionController? _zerobotProgression;
    private bool _progressionInFlight;
    private const string EnergyOnlyFeederDna = "cond\nstart\n-2 .shoot store\n50 .shootval store\n314 rnd .aimdx store\nstop";
    private uint _ticksPerUpdate;
    private uint _maximumBatch = 100;
    private bool _maximumSpeed;
    private bool _following;
    private bool _controlsReady;
    private EnvironmentUpdate _liveEnvironment;

    public MainWindow() : this([], new WorldSetupOptions(), NativeEngineClientFactory.Instance, AvaloniaDesktopStorageService.Instance) { }

    public MainWindow(IReadOnlyList<string> arguments) : this(arguments, new WorldSetupOptions(), NativeEngineClientFactory.Instance, AvaloniaDesktopStorageService.Instance) { }

    public MainWindow(IReadOnlyList<string> arguments, WorldSetupOptions setup) :
        this(arguments, setup, NativeEngineClientFactory.Instance, AvaloniaDesktopStorageService.Instance)
    {
    }

    public MainWindow(
        IReadOnlyList<string> arguments,
        WorldSetupOptions setup,
        IEngineClientFactory engineFactory,
        IDesktopStorageService storage)
    {
        _startupBots = ParseStartupBots(arguments);
        _setup = setup;
        _engineFactory = engineFactory ?? throw new ArgumentNullException(nameof(engineFactory));
        _storage = storage ?? throw new ArgumentNullException(nameof(storage));
        _zerobotProgression = setup.StartingMode is StartingMode.Zerobots or StartingMode.ZerobotsAndVegetables
            ? new ZerobotProgressionController(setup.AutomaticZerobotProgression)
            : null;
        InitializeComponent();
        _ticksPerUpdate = setup.TicksPerUpdate;
        _liveEnvironment = setup.ToEnvironmentUpdate();
        ToroidalWorldToggle.IsChecked = _liveEnvironment.ToroidalWorld;
        LiveMetabolism.Value = _liveEnvironment.MetabolismCost;
        LivePlantEnergy.Value = _liveEnvironment.Vegetation.MaxEnergyPerTick;
        LiveFeedingToBody.Value = (decimal)_liveEnvironment.Vegetation.FeedingToBody;
        WorldParametersText.Text = $"{setup.WorldWidth:N0} × {setup.WorldHeight:N0}\nCapacity {setup.PopulationCapacity:N0}\nVegetables {setup.VegetablePopulationCap:N0}";
        Viewport.ToroidalWorld = _liveEnvironment.ToroidalWorld;
        RuntimeSpeed.SelectedIndex = _ticksPerUpdate switch { <= 1 => 0, <= 5 => 1, <= 20 => 2, _ => 3 };
        _maximumSpeed = DesktopControlRules.IsMaximumSpeed(RuntimeSpeed.SelectedIndex);
        Viewport.OrganismSelected += slot =>
        {
            _viewModel.Select(slot);
            Viewport.SelectSlot(slot);
        };
        Viewport.WorldClicked += async position =>
        {
            if (_session is null) return;
            if (_moveOrganism is { } organism)
            {
                _moveOrganism = null;
                await _session.MoveAsync(organism.Slot, organism.Generation, position);
                _viewModel.Status = "ORGANISM MOVED";
                return;
            }
            if (_environmentPlacement == EnvironmentPlacement.None) return;
            if (_placementOrigin is null)
            {
                _placementOrigin = position;
                _viewModel.Status = _environmentPlacement == EnvironmentPlacement.Obstacle
                    ? "OBSTACLE · CLICK OPPOSITE CORNER"
                    : "TELEPORTER · CLICK DESTINATION";
                return;
            }
            if (_environmentPlacement == EnvironmentPlacement.Obstacle)
            {
                var minimum = new[] { Math.Min(_placementOrigin[0], position[0]), Math.Min(_placementOrigin[1], position[1]) };
                var maximum = new[] { Math.Max(_placementOrigin[0], position[0]), Math.Max(_placementOrigin[1], position[1]) };
                if (maximum[0] - minimum[0] < 10) maximum[0] = minimum[0] + 10;
                if (maximum[1] - minimum[1] < 10) maximum[1] = minimum[1] + 10;
                var id = _session.LatestSnapshot.Obstacles.Select(value => value.Id).DefaultIfEmpty().Max() + 1;
                await _session.AddObstacleAsync(new ObstacleSnapshot(id, minimum, maximum));
                _viewModel.Status = $"OBSTACLE {id} ADDED";
            }
            else
            {
                var id = _session.LatestSnapshot.Teleporters.Select(value => value.Id).DefaultIfEmpty().Max() + 1;
                await _session.AddTeleporterAsync(new TeleporterSnapshot(id, _placementOrigin, 180f, position));
                _viewModel.Status = $"TELEPORTER {id} ADDED";
            }
            _placementOrigin = null;
            _environmentPlacement = EnvironmentPlacement.None;
        };
        Viewport.WorldFeatureSelected += feature =>
        {
            _selectedFeature = feature;
            _viewModel.Status = $"{feature.Kind.ToString().ToUpperInvariant()} {feature.Id} SELECTED";
        };
        Viewport.OrganismDragCompleted += async (slot, position) =>
        {
            if (_session is null) return;
            var selected = _viewModel.SelectedOrganism;
            var organism = selected is not null && selected.Slot == slot
                ? selected
                : _session.LatestSnapshot.Organisms.FirstOrDefault(value => value.Slot == slot);
            if (organism is null) return;
            await _session.MoveAsync(organism.Slot, organism.Generation, position);
            _viewModel.Status = $"ORGANISM {organism.Slot}:{organism.Generation} MOVED";
        };
        DataContext = _viewModel;
        _runTimer = new DispatcherTimer { Interval = TimeSpan.FromMilliseconds(_maximumSpeed ? 1 : 16) };
        _runTimer.Tick += RunTimer_Tick;
        _autosaveTimer = new DispatcherTimer { Interval = TimeSpan.FromMinutes(1) };
        _autosaveTimer.Tick += AutosaveTimer_Tick;
        Opened += Window_Opened;
        Closed += Window_Closed;
        _controlsReady = true;
    }

    private async void Window_Opened(object? sender, EventArgs e)
    {
        try
        {
            _session = new SimulationSession(_engineFactory.Create(_setup));
            _session.SnapshotPublished += snapshot => Dispatcher.UIThread.Post(() =>
            {
                _viewModel.Update(snapshot);
                Viewport.SetSnapshot(snapshot);
                Viewport.SelectSlot(_viewModel.SelectedSlot);
                PopulationChart.SetSnapshot(snapshot);
                EnergyChart.SetSnapshot(snapshot);
                if (_zerobotProgression is not null && !_progressionInFlight
                    && _zerobotProgression.Observe(snapshot) is { } transition)
                    _ = ApplyZerobotTransitionAsync(transition);
            });
            _viewModel.Status = "ENGINE READY — IMPORT A LEGACY BOT";
            var compatibilityWarnings = new List<string>();
            if (_setup.LoadSavePath is { } savePath)
                await _session.LoadAsync(await File.ReadAllBytesAsync(savePath));
            else
                foreach (var species in _setup.Species)
                    compatibilityWarnings.AddRange((await _session.ImportSpeciesAsync(species)).CompatibilityWarnings);
            foreach (var bot in _startupBots)
            {
                compatibilityWarnings.AddRange((await _session.ImportDnaAsync(await File.ReadAllTextAsync(bot), [8000f, 6000f])).CompatibilityWarnings);
            }
            if (compatibilityWarnings.Count > 0) _viewModel.Status = $"DNA COMPATIBILITY · {string.Join(" · ", compatibilityWarnings.Distinct())}";
            else if (_startupBots.Count > 0) _viewModel.Status = $"IMPORTED {_startupBots.Count} STARTUP BOT(S)";
            else if (_setup.Species.Count > 0) _viewModel.Status = $"WORLD READY · {_setup.Species.Count} SPECIES";
            else if (_setup.LoadSavePath is not null) _viewModel.Status = "AUTOSAVE RECOVERED";
            _autosaveTimer.Start();
        }
        catch (Exception error)
        {
            _viewModel.Status = $"ENGINE ERROR — {error.Message}";
        }
    }

    private static IReadOnlyList<string> ParseStartupBots(IReadOnlyList<string> arguments)
    {
        var bots = new List<string>();
        for (var index = 0; index < arguments.Count; index++)
        {
            if (arguments[index] == "--bot" && index + 1 < arguments.Count)
                bots.Add(Path.GetFullPath(arguments[++index]));
        }
        return bots;
    }

    private async void Step_Click(object? sender, RoutedEventArgs e)
    {
        if (_session is null) return;
        await _session.StepAsync();
    }

    private async void Kill_Click(object? sender, RoutedEventArgs e)
    {
        if (_session is null || _viewModel.SelectedOrganism is not { } organism) return;
        await _session.RemoveAsync(organism.Slot, organism.Generation);
        Viewport.SelectSlot(null);
    }

    private void Move_Click(object? sender, RoutedEventArgs e)
    {
        if (_viewModel.SelectedOrganism is not { } organism) return;
        _moveOrganism = new OrganismKey(organism.Slot, organism.Generation);
        _viewModel.Status = "CLICK AN EMPTY WORLD POSITION";
    }

    private async void Clone_Click(object? sender, RoutedEventArgs e)
    {
        if (_session is null || _viewModel.SelectedOrganism is not { } organism) return;
        await _session.CloneAsync(organism.Slot, organism.Generation, [organism.Position[0] + 24f, organism.Position[1] + 24f]);
        _viewModel.Status = "ORGANISM CLONED";
    }

    private async void Reproduce_Click(object? sender, RoutedEventArgs e)
    {
        if (_session is null || _viewModel.SelectedOrganism is not { } organism) return;
        await _session.ReproduceAsync(
            organism.Slot,
            organism.Generation,
            null,
            null,
            [organism.Position[0] + 20f, organism.Position[1] + 20f]);
        _viewModel.Status = "MANUAL REPRODUCTION COMPLETE · 50% SPLIT";
    }

    private async void EditDna_Click(object? sender, RoutedEventArgs e)
    {
        if (_session is null || _viewModel.SelectedOrganism is not { } organism) return;
        await new DnaEditorWindow(_session, organism, _storage).ShowDialog(this);
    }

    private void Run_Click(object? sender, RoutedEventArgs e)
    {
        _runTimer.Start();
        _viewModel.Status = "RUNNING";
    }

    private void Pause_Click(object? sender, RoutedEventArgs e)
    {
        _runTimer.Stop();
        _viewModel.Status = "PAUSED";
    }

    private void Reset_Click(object? sender, RoutedEventArgs e)
    {
        _runTimer.Stop();
        var replacement = new MainWindow(_startupBots, _setup, _engineFactory, _storage);
        replacement.Show();
        Close();
    }

    private void Speed_Changed(object? sender, SelectionChangedEventArgs e)
    {
        if (!_controlsReady) return;
        _ticksPerUpdate = DesktopControlRules.TicksForSelection(RuntimeSpeed.SelectedIndex);
        _maximumSpeed = DesktopControlRules.IsMaximumSpeed(RuntimeSpeed.SelectedIndex);
        if (_maximumSpeed) _maximumBatch = _ticksPerUpdate;
        _runTimer.Interval = TimeSpan.FromMilliseconds(_turbo || _maximumSpeed ? 1 : 16);
        _viewModel.Status = _maximumSpeed ? "THROTTLE · MAXIMUM" : $"THROTTLE · {_ticksPerUpdate} TICKS/FRAME";
    }

    private void Follow_Click(object? sender, RoutedEventArgs e)
    {
        if (_viewModel.SelectedOrganism is not { } organism) return;
        _following = !_following;
        Viewport.FollowSlot(_following ? organism.Slot : null);
        _viewModel.Status = _following ? $"FOLLOWING {organism.Slot}:{organism.Generation}" : "FOLLOW OFF";
    }

    private void Turbo_Click(object? sender, RoutedEventArgs e)
    {
        _turbo = !_turbo;
        _runTimer.Interval = TimeSpan.FromMilliseconds(_turbo || _maximumSpeed ? 1 : 16);
        Viewport.IsVisible = !_turbo;
        TurboButton.Content = _turbo ? "SHOW WORLD" : "TURBO";
        _viewModel.Status = _turbo ? "TURBO · RENDERING SUSPENDED" : "VIEWPORT RESTORED";
        if (_turbo) _runTimer.Start();
    }

    private async void SwitchBackend_Click(object? sender, RoutedEventArgs e)
    {
        if (_session is null) return;
        var target = _viewModel.Backend.Equals("GPU", StringComparison.OrdinalIgnoreCase) ? "Cpu" : "Gpu";
        try
        {
            await _session.SwitchBackendAsync(target);
            _viewModel.Status = $"BACKEND SWITCHED TO {_session.LatestSnapshot.Backend}";
        }
        catch (Exception error) { _viewModel.Status = $"BACKEND SWITCH FAILED · {error.Message}"; }
    }

    private void AddObstacle_Click(object? sender, RoutedEventArgs e)
    {
        _environmentPlacement = EnvironmentPlacement.Obstacle;
        _placementOrigin = null;
        _viewModel.Status = "OBSTACLE · CLICK FIRST CORNER";
    }

    private void AddTeleporter_Click(object? sender, RoutedEventArgs e)
    {
        _environmentPlacement = EnvironmentPlacement.Teleporter;
        _placementOrigin = null;
        _viewModel.Status = "TELEPORTER · CLICK SOURCE";
    }

    private async void RemoveFeature_Click(object? sender, RoutedEventArgs e)
    {
        if (_session is null || _selectedFeature is not { } feature) return;
        if (feature.Kind == WorldFeatureKind.Obstacle) await _session.RemoveObstacleAsync(feature.Id);
        else await _session.RemoveTeleporterAsync(feature.Id);
        _selectedFeature = null;
        Viewport.SelectFeature(null);
        _viewModel.Status = $"{feature.Kind.ToString().ToUpperInvariant()} {feature.Id} REMOVED";
    }

    private async void AdvanceZerobot_Click(object? sender, RoutedEventArgs e)
    {
        if (_zerobotProgression is null)
        {
            _viewModel.Status = "ZEROBOT PROGRESSION IS NOT ACTIVE IN THIS WORLD";
            return;
        }
        if (_zerobotProgression.AdvanceManually() is { } transition)
            await ApplyZerobotTransitionAsync(transition);
        else
            _viewModel.Status = "ZEROBOT PROGRESSION COMPLETE";
    }

    private async void LiveAdvanced_Click(object? sender, RoutedEventArgs e)
    {
        if (_session is null) return;
        var dialog = new AdvancedSettingsWindow(_liveEnvironment);
        await dialog.ShowDialog(this);
        if (!dialog.Accepted) return;
        var update = dialog.Update;
        await _session.UpdateEnvironmentAsync(update);
        _liveEnvironment = update;
        _viewModel.Status = "LIVE ENVIRONMENT SETTINGS APPLIED";
    }

    private async void ToroidalWorld_Changed(object? sender, RoutedEventArgs e)
    {
        if (!_controlsReady || _session is null) return;
        var update = _liveEnvironment with { ToroidalWorld = ToroidalWorldToggle.IsChecked == true };
        await _session.UpdateEnvironmentAsync(update);
        _liveEnvironment = update;
        Viewport.ToroidalWorld = update.ToroidalWorld;
        _viewModel.Status = update.ToroidalWorld ? "TOROIDAL WORLD ON" : "TOROIDAL WORLD OFF";
    }

    private void RenderWaste_Changed(object? sender, RoutedEventArgs e)
    {
        if (!_controlsReady) return;
        Viewport.RenderWaste = RenderWasteToggle.IsChecked == true;
        _viewModel.Status = Viewport.RenderWaste ? "WASTE RENDERING ON" : "WASTE RENDERING OFF";
    }

    private void ShowVision_Changed(object? sender, RoutedEventArgs e)
    {
        if (!_controlsReady) return;
        Viewport.ShowSelectedVision = ShowVisionToggle.IsChecked == true;
    }

    private async void ApplyEnergy_Click(object? sender, RoutedEventArgs e)
    {
        if (_session is null) return;
        var vegetation = _liveEnvironment.Vegetation with
        {
            MaxEnergyPerTick = (int)(LivePlantEnergy.Value ?? 100),
            FeedingToBody = (float)(LiveFeedingToBody.Value ?? 0.75m),
        };
        var update = _liveEnvironment with
        {
            MetabolismCost = (int)(LiveMetabolism.Value ?? 0),
            Vegetation = vegetation,
        };
        await _session.UpdateEnvironmentAsync(update);
        _liveEnvironment = update;
        _viewModel.Status = $"ENERGY APPLIED · METABOLISM {update.MetabolismCost} · PLANTS {vegetation.MaxEnergyPerTick}";
    }

    private async Task ApplyZerobotTransitionAsync(ZerobotProgressionTransition transition)
    {
        if (_session is null || _progressionInFlight) return;
        _progressionInFlight = true;
        try
        {
            var snapshot = _session.LatestSnapshot;
            var feederSpecies = snapshot.Species.Select((species, index) => (species, index))
                .Where(value => value.species.Name.Contains("Feeder", StringComparison.OrdinalIgnoreCase))
                .Select(value => (uint)value.index).ToHashSet();
            var feeders = snapshot.Organisms.Where(organism => feederSpecies.Contains(organism.Species)).ToArray();
            switch (transition.Action)
            {
                case ZerobotProgressionAction.SwitchToEnergyOnlyFeeder:
                    foreach (var feeder in feeders)
                        await _session.ReplaceDnaAsync(feeder.Slot, feeder.Generation, EnergyOnlyFeederDna);
                    break;
                case ZerobotProgressionAction.RemoveFeederAssistance:
                    foreach (var feeder in feeders)
                        await _session.RemoveAsync(feeder.Slot, feeder.Generation);
                    break;
                case ZerobotProgressionAction.DisableBrownianMotion:
                    await _session.SetBrownianMotionAsync(0f);
                    _liveEnvironment = _liveEnvironment with { BrownianMotion = 0f };
                    break;
            }
            _viewModel.Status = $"ZEROBOT · {transition.Message.ToUpperInvariant()}";
        }
        catch (Exception error)
        {
            _viewModel.Status = $"ZEROBOT PROGRESSION FAILED · {error.Message}";
        }
        finally { _progressionInFlight = false; }
    }

    private async void Import_Click(object? sender, RoutedEventArgs e)
    {
        if (_session is null) return;
        foreach (var file in await _storage.OpenDnaFilesAsync(this))
        {
            var report = await _session.ImportSpeciesAsync(new SpeciesImport(
                Path.GetFileNameWithoutExtension(file.Name),
                file.Content,
                false,
                0xffd07a2d,
                1,
                1_000,
                0,
                false,
                1f));
            _viewModel.Status = report.CompatibilityWarnings.Count == 0
                ? $"IMPORTED {file.Name}"
                : $"DNA COMPATIBILITY · {string.Join(" · ", report.CompatibilityWarnings)}";
        }
    }

    private async void Save_Click(object? sender, RoutedEventArgs e)
    {
        if (_session is null) return;
        if (await _storage.SaveSimulationAsync(this, await _session.SaveAsync()))
            _viewModel.Status = "SAVED";
    }

    private async void Load_Click(object? sender, RoutedEventArgs e)
    {
        if (_session is null) return;
        if (await _storage.OpenSimulationAsync(this) is not { } save) return;
        await _session.LoadAsync(save);
        _viewModel.Status = "LOADED";
    }

    private async void RunTimer_Tick(object? sender, EventArgs e)
    {
        if (_session is null || _tickInFlight) return;
        _tickInFlight = true;
        var adaptive = _turbo || _maximumSpeed;
        var batch = adaptive ? _maximumBatch : _ticksPerUpdate;
        var started = Stopwatch.GetTimestamp();
        try
        {
            await _session.StepAsync(batch);
            if (adaptive)
                _maximumBatch = DesktopControlRules.AdaptMaximumBatch(batch, Stopwatch.GetElapsedTime(started), _turbo);
        }
        finally { _tickInFlight = false; }
    }

    private async void AutosaveTimer_Tick(object? sender, EventArgs e)
    {
        if (_session is null) return;
        try
        {
            await _storage.SaveAutosaveAsync(await _session.SaveAsync());
            _viewModel.Status = "AUTOSAVED";
        }
        catch (Exception error) { _viewModel.Status = $"AUTOSAVE ERROR · {error.Message}"; }
    }

    private async void Window_Closed(object? sender, EventArgs e)
    {
        _runTimer.Stop();
        _autosaveTimer.Stop();
        if (_session is not null) await _session.DisposeAsync();
    }
}

internal enum EnvironmentPlacement { None, Obstacle, Teleporter }
