using Avalonia;
using Avalonia.Controls;
using Avalonia.Input;
using Avalonia.Interactivity;
using Avalonia.Threading;
using Darwinbots.Desktop.Core;
using Darwinbots.Desktop.Core.LegacyMenus;
using Darwinbots.Desktop.Controls;
using Darwinbots.Desktop.Services;
using Darwinbots.Desktop.ViewModels;
using System.Collections.ObjectModel;
using System.Diagnostics;

namespace Darwinbots.Desktop.Views;

public sealed partial class MainWindow : Window
{
    private const string TrayPngBase64 = "iVBORw0KGgoAAAANSUhEUgAAABAAAAAQCAYAAAAf8/9hAAAAAXNSR0IArs4c6QAAAARnQU1BAACxjwv8YQUAAAAJcEhZcwAADsMAAA7DAcdvqGQAAAAlSURBVDhPY1BQV/1PCQYb4FNrQBYeNWDUABAeNYCaBpCPVf8DAN1j1wSVGu34AAAAAElFTkSuQmCC";

    private void MainWindow_KeyDown(object? sender, KeyEventArgs e)
    {
        var routed = new RoutedEventArgs();
        if (e.Key == Key.F6) Pause_Click(sender, routed);
        else if (e.Key == Key.F7) Step_Click(sender, routed);
        else if (e.Key == Key.F9)
            RuntimeSpeed.SelectedIndex = (RuntimeSpeed.SelectedIndex + 1) % 4;
        else if (e.KeyModifiers.HasFlag(KeyModifiers.Control))
        {
            switch (e.Key)
            {
                case Key.L: Load_Click(sender, routed); break;
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
    private readonly ObservableCollection<SpeciesImport> _insertionSpecies = [];
    private GlobalSettings _globalSettings;
    private readonly IEngineClientFactory _engineFactory;
    private readonly IDesktopStorageService _storage;
    private bool _tickInFlight;
    private OrganismKey? _moveOrganism;
    private WorldInsertionController? _insertion;
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
    private readonly LegacyCommandRouter _legacyCommands;
    private readonly OfflineDiagnosticRecovery _diagnostics = new();
    private bool _safemodeBackup = true;
    private bool _hasBackgroundImage;
    private bool _deadRecordingEnabled;
    private bool _excludeRepopulatingDead;
    private int _recordedCorpseCount;
    private bool _automaticTagByName;
    private bool _playerBotEnabled;
    private float _playerBotThrust = 30f;
    private bool _rgbMemoryMonitorEnabled;
    private string _rgbMemoryChannels = ".nrg, .body, .waste";
    private const string RgbMonitorToolKey = "rgb-memory-monitor";
    private readonly PersistentToolWindowRegistry _toolWindows;
    private readonly Stopwatch _runtimeClock = new();
    private double _activeCostMultiplier;
    private TrayIcon? _trayIcon;
    private NativeMenuItem? _trayIgnoreErrorsItem;

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
        _globalSettings = setup.GlobalSettings;
        _engineFactory = engineFactory ?? throw new ArgumentNullException(nameof(engineFactory));
        _storage = storage ?? throw new ArgumentNullException(nameof(storage));
        _zerobotProgression = setup.StartingMode is StartingMode.Zerobots or StartingMode.ZerobotsAndVegetables
            ? new ZerobotProgressionController(setup.AutomaticZerobotProgression)
            : null;
        InitializeComponent();
        _toolWindows = new PersistentToolWindowRegistry(this);
        _activeCostMultiplier = setup.GlobalSettings.Costs.GlobalMultiplier;
        foreach (var species in setup.Species)
            _insertionSpecies.Add(species);
        InsertionSpeciesSelector.ItemsSource = _insertionSpecies;
        if (_insertionSpecies.Count > 0)
            InsertionSpeciesSelector.SelectedIndex = 0;
        InitializeTray();
        KeyDown += PlayerBot_KeyDown;
        _legacyCommands = CreateLegacyCommandRouter();
        HistoricMenuBar.CommandInvoked += HistoricMenuBar_CommandInvoked;
        _ticksPerUpdate = setup.TicksPerUpdate;
        _liveEnvironment = setup.ToEnvironmentUpdate();
        LocalModeKindSelector.SelectedIndex = 0;
        LocalModePreset.SelectedIndex = 0;
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
            RefreshLegacyMenuStates();
        };
        Viewport.WorldClicked += async position =>
        {
            if (_session is null) return;
            if (_insertion?.IsActive == true)
            {
                if (_insertion.SelectedSpecies is not { } species) return;
                try
                {
                    await _insertion.PlaceAsync(position);
                    _viewModel.Status = $"INSERTED {species.Name.ToUpperInvariant()} · {position[0]:0}, {position[1]:0}";
                }
                catch (Exception error)
                {
                    _viewModel.Status = $"INSERTION FAILED · {error.Message}";
                }
                return;
            }
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
            RefreshFeatureEditorButtons();
        };
        RefreshFeatureEditorButtons();
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
        RefreshLegacyMenuStates();
    }

    private void InitializeTray()
    {
        if (Application.Current is not { } application)
            return;

        try
        {
            var trayIcon = new TrayIcon
            {
                Icon = CreateTrayIcon(),
                ToolTipText = "Darwinbots Modern",
                IsVisible = !_globalSettings.Main.HideTray,
            };
            var menu = new NativeMenu();
            var restore = new NativeMenuItem("Restore Darwinbots");
            restore.Click += (_, _) => RestoreFromTray();
            _trayIgnoreErrorsItem = new NativeMenuItem("Ignore errors")
            {
                ToggleType = MenuItemToggleType.CheckBox,
                IsChecked = _diagnostics.IgnoreErrors,
            };
            _trayIgnoreErrorsItem.Click += (_, _) => SetIgnoreErrors(!_diagnostics.IgnoreErrors);
            var exit = new NativeMenuItem("Exit");
            exit.Click += (_, _) => Close();
            menu.Add(restore);
            menu.Add(_trayIgnoreErrorsItem);
            menu.Add(exit);
            trayIcon.Menu = menu;
            trayIcon.Clicked += (_, _) => RestoreFromTray();

            var icons = new TrayIcons();
            icons.Add(trayIcon);
            TrayIcon.SetIcons(application, icons);
            _trayIcon = trayIcon;
        }
        catch
        {
            // Tray support is optional on headless and desktop environments without a tray host.
            _trayIcon = null;
        }
    }

    private static WindowIcon CreateTrayIcon()
    {
        using var stream = new MemoryStream(Convert.FromBase64String(TrayPngBase64));
        return new WindowIcon(stream);
    }

    private void SetIgnoreErrors(bool enabled)
    {
        _diagnostics.SetIgnoreErrors(enabled);
        if (IgnoreErrorsToggle.IsChecked != enabled)
            IgnoreErrorsToggle.IsChecked = enabled;
        if (_trayIgnoreErrorsItem is not null)
            _trayIgnoreErrorsItem.IsChecked = enabled;
        RefreshLegacyMenuStates();
    }

    private LegacyCommandResult ToggleIgnoreErrors()
    {
        var enabled = !_diagnostics.IgnoreErrors;
        SetIgnoreErrors(enabled);
        return LegacyCommandResult.Success(enabled ? "IGNORE ERRORS ON" : "IGNORE ERRORS OFF");
    }

    private void IgnoreErrors_Changed(object? sender, RoutedEventArgs e)
    {
        if (_controlsReady)
            SetIgnoreErrors(IgnoreErrorsToggle.IsChecked == true);
    }

    private void HideToTray_Click(object? sender, RoutedEventArgs e)
    {
        if (_globalSettings.Main.HideTray)
        {
            _viewModel.Status = "TRAY HIDDEN IN GLOBAL SETTINGS";
            return;
        }
        if (_trayIcon is null)
        {
            _viewModel.Status = "TRAY UNAVAILABLE ON THIS DESKTOP";
            return;
        }

        _diagnostics.HideToTray();
        _trayIcon.IsVisible = true;
        Hide();
    }

    private void RestoreFromTray()
    {
        Dispatcher.UIThread.Post(() =>
        {
            _diagnostics.RestoreFromTray();
            Show();
            WindowState = WindowState.Normal;
            Activate();
            _viewModel.Status = "RESTORED FROM TRAY";
        });
    }

    private LegacyCommandRouter CreateLegacyCommandRouter()
    {
        const string pending = "This historical command is not implemented in the modern runtime yet.";
        const string offline = "Online play is outside the modern compatibility scope.";
        var handlers = LegacyMenuCatalog.FlattenLeaves().ToDictionary(
            item => item.Command!.Value,
            _ => (ILegacyCommandHandler)DelegateLegacyCommandHandler.Disabled(pending));

        handlers[LegacyCommandId.FilePauseSimulation] = Handler(
            () => LegacyCommandState.Available,
            _ => Task.FromResult(PauseSimulation()));
        handlers[LegacyCommandId.FileIgnoreErrors] = Handler(
            () => new LegacyCommandState(true, _diagnostics.IgnoreErrors),
            _ => Task.FromResult(ToggleIgnoreErrors()));
        handlers[LegacyCommandId.FileNewSimulation] = Handler(
            () => LegacyCommandState.Available,
            _ => Task.FromResult(OpenNewSimulationSetup()));
        handlers[LegacyCommandId.FileLoadSimulation] = Handler(
            EngineState,
            async _ => await LoadSimulationCommandAsync());
        handlers[LegacyCommandId.FileSaveSimulation] = Handler(
            EngineState,
            async _ => await SaveSimulationCommandAsync("SAVED", withoutMutations: false));
        handlers[LegacyCommandId.FileSaveWithoutMutations] = Handler(
            EngineState,
            async _ => await SaveSimulationCommandAsync("SAVED WITHOUT MUTATION SETTINGS", withoutMutations: true));
        handlers[LegacyCommandId.FileQuit] = Handler(
            () => LegacyCommandState.Available,
            _ => Task.FromResult(QuitApplication()));
        handlers[LegacyCommandId.EditInsertOrganism] = Handler(
            EngineState,
            async _ => await ImportOrganismsCommandAsync());
        handlers[LegacyCommandId.EditRestartAndLeagues] = Handler(
            () => new LegacyCommandState(true, false),
            _ => Task.FromResult(OpenLeagueTab()));
        handlers[LegacyCommandId.EditDisallowFixing] = Handler(
            () => new LegacyCommandState(_session is not null, _session?.LatestSnapshot.RobotFixingAllowed == false, _session is null ? "Start a simulation first." : null),
            async _ => await ToggleLegacyRestrictionAsync(true));
        handlers[LegacyCommandId.EditDisableAsexualReproduction] = Handler(
            () => new LegacyCommandState(_session is not null, _session?.LatestSnapshot.AsexualReproductionAllowed == false, _session is null ? "Start a simulation first." : null),
            async _ => await ToggleLegacyRestrictionAsync(false));
        handlers[LegacyCommandId.EditAutomaticallyTagByName] = Handler(
            () => new LegacyCommandState(true, _automaticTagByName),
            _ => Task.FromResult(ToggleAutomaticTagging()));
        handlers[LegacyCommandId.EditEnableAutomaticForking] = Handler(
            () => new LegacyCommandState(_session is not null, _liveEnvironment.AutoSpeciation, _session is null ? "Start a simulation first." : null),
            async _ => await ToggleAutomaticForkingAsync());
        handlers[LegacyCommandId.EditPlayerBotSettings] = Handler(
            SelectedRobotState,
            async _ => await OpenPlayerBotSettingsAsync());
        handlers[LegacyCommandId.EditEnablePlayerBot] = Handler(
            () => SelectedRobotState() with { Checked = _playerBotEnabled },
            _ => Task.FromResult(TogglePlayerBot()));
        handlers[LegacyCommandId.EditEyeDesigner] = Handler(
            SelectedRobotState,
            async _ => await OpenEyeDesignerAsync());
        handlers[LegacyCommandId.EditRestrictionOverwrites] = Handler(
            EngineState,
            async _ => await ShowRestrictionOverridesAsync());
        foreach (var command in new[]
                 {
                     LegacyCommandId.EditSpecies,
                     LegacyCommandId.EditGeneralSettings,
                     LegacyCommandId.EditPhysicsAndCosts,
                     LegacyCommandId.EditGlobalSettings,
                     LegacyCommandId.EditGlobalMutationRates,
                     LegacyCommandId.RobotMutationRates,
                 })
            handlers[command] = Handler(EngineState, async _ => await OpenAdvancedSettingsCommandAsync());
        handlers[LegacyCommandId.EditGlobalSettings] = Handler(
            () => LegacyCommandState.Available,
            async _ => await OpenGlobalSettingsCommandAsync());
        handlers[LegacyCommandId.ViewShowVisionGrid] = Handler(
            () => new LegacyCommandState(true, Viewport.ShowSelectedVision),
            _ => Task.FromResult(ToggleVisionGrid()));
        handlers[LegacyCommandId.ViewDisplayMovementVectors] = ViewportToggle(
            () => Viewport.ShowMovementVectors,
            value => Viewport.ShowMovementVectors = value,
            "MOVEMENT VECTORS");
        handlers[LegacyCommandId.ViewDisplayShotImpacts] = ViewportToggle(
            () => Viewport.ShowShotImpacts,
            value => Viewport.ShowShotImpacts = value,
            "SHOT IMPACTS");
        handlers[LegacyCommandId.ViewDisplayResourceGuages] = ViewportToggle(
            () => Viewport.ShowResourceGauges,
            value => Viewport.ShowResourceGauges = value,
            "RESOURCE GAUGES");
        handlers[LegacyCommandId.ViewRgbMemoryMonitorSettings] = Handler(
            SelectedRobotState,
            async _ => await OpenRgbMonitorSettingsAsync());
        handlers[LegacyCommandId.ViewRgbMemoryMonitor] = Handler(
            () => SelectedRobotState() with { Checked = _rgbMemoryMonitorEnabled },
            _ => Task.FromResult(ToggleRgbMemoryMonitor()));
        handlers[LegacyCommandId.ViewImportBackground] = Handler(
            () => new LegacyCommandState(true, false),
            async _ => await ImportBackgroundAsync());
        handlers[LegacyCommandId.ViewRemoveBackground] = Handler(
            () => new LegacyCommandState(_hasBackgroundImage, false, _hasBackgroundImage ? null : "No background picture is loaded."),
            _ => Task.FromResult(RemoveBackground()));
        handlers[LegacyCommandId.ObjectsNewTeleporter] = Handler(
            () => LegacyCommandState.Available,
            _ => Task.FromResult(BeginEnvironmentPlacement(EnvironmentPlacement.Teleporter)));
        handlers[LegacyCommandId.ObjectsDeleteTeleporter] = Handler(
            () => FeatureState(WorldFeatureKind.Teleporter),
            async _ => await RemoveSelectedFeatureCommandAsync());
        handlers[LegacyCommandId.ObjectsDeleteAllTeleporters] = Handler(
            EngineState,
            async _ => await DeleteAllTeleportersCommandAsync());
        handlers[LegacyCommandId.ObjectsHilightTeleporters] = ViewportToggle(
            () => Viewport.HighlightTeleporters,
            value => Viewport.HighlightTeleporters = value,
            "TELEPORTER HIGHLIGHT");
        handlers[LegacyCommandId.ObjectsDontDecayNrgShots] = Handler(
            () => new LegacyCommandState(_session is not null, _liveEnvironment.Shots.EnergyShotsDoNotDecay),
            async _ => await ToggleShotDecayAsync(true));
        handlers[LegacyCommandId.ObjectsDontDecayWasteShots] = Handler(
            () => new LegacyCommandState(_session is not null, _liveEnvironment.Shots.WasteShotsDoNotDecay),
            async _ => await ToggleShotDecayAsync(false));
        handlers[LegacyCommandId.ObjectsDisableTies] = Handler(
            () => new LegacyCommandState(_session is not null, _session?.LatestSnapshot.TiesEnabled == false),
            async _ => await ToggleTiesAsync());
        handlers[LegacyCommandId.ObjectsNewShape] = Handler(
            () => LegacyCommandState.Available,
            _ => Task.FromResult(BeginEnvironmentPlacement(EnvironmentPlacement.Obstacle)));
        handlers[LegacyCommandId.ObjectsDeleteShape] = Handler(
            () => FeatureState(WorldFeatureKind.Obstacle),
            async _ => await RemoveSelectedFeatureCommandAsync());
        handlers[LegacyCommandId.ObjectsDeleteAllShapes] = Handler(
            EngineState,
            async _ => await DeleteAllObstaclesCommandAsync());
        handlers[LegacyCommandId.ObjectsAddTenRandomShapes] = Handler(
            EngineState,
            async _ => await AddRandomObstaclesCommandAsync(10));
        handlers[LegacyCommandId.ObjectsDeleteTenRandomShapes] = Handler(
            EngineState,
            async _ => await DeleteRandomObstaclesCommandAsync(10));
        handlers[LegacyCommandId.ObjectsHorizontalMaze] = MazeHandler(MazePattern.Horizontal);
        handlers[LegacyCommandId.ObjectsVerticalMaze] = MazeHandler(MazePattern.Vertical);
        handlers[LegacyCommandId.ObjectsSprial] = MazeHandler(MazePattern.Spiral);
        handlers[LegacyCommandId.ObjectsCheckerboard] = MazeHandler(MazePattern.Checkerboard);
        handlers[LegacyCommandId.ObjectsPolarIce] = MazeHandler(MazePattern.PolarIce);
        handlers[LegacyCommandId.ObjectsTrashCompactor] = MazeHandler(MazePattern.TrashCompactor);
        handlers[LegacyCommandId.RobotKill] = Handler(
            SelectedRobotState,
            async _ => await KillSelectedAsync());
        handlers[LegacyCommandId.RobotKillEntireOrganism] = Handler(
            SelectedRobotState,
            async _ => await KillSelectedOrganismAsync());
        handlers[LegacyCommandId.RobotShowInfo] = Handler(
            SelectedRobotState,
            async _ => await ShowSelectedRobotInfoAsync());
        handlers[LegacyCommandId.RobotShowPhilogeny] = Handler(
            SelectedRobotState,
            async _ => await ShowSelectedPhylogenyAsync());
        handlers[LegacyCommandId.RobotChangeColor] = Handler(
            SelectedRobotState,
            async _ => await ChangeSelectedSpeciesColorAsync());
        handlers[LegacyCommandId.RobotOpenConsole] = Handler(
            SelectedRobotState,
            async _ => await ShowSelectedRobotConsoleAsync());
        handlers[LegacyCommandId.RobotViewGeneActivations] = Handler(
            SelectedRobotState,
            async _ => await ShowSelectedGeneActivationsAsync());
        handlers[LegacyCommandId.RobotSaveDna] = Handler(
            SelectedRobotState,
            async _ => await SaveSelectedDnaCommandAsync());
        handlers[LegacyCommandId.RobotMakeNewSpecies] = Handler(
            SelectedRobotState,
            async _ => await MakeSelectedNewSpeciesAsync());
        handlers[LegacyCommandId.RobotSelectEntireOrganism] = Handler(
            SelectedRobotState,
            _ => Task.FromResult(SelectEntireOrganism()));
        handlers[LegacyCommandId.RobotSaveEntireOrganism] = Handler(
            SelectedRobotState,
            async _ => await SaveEntireOrganismAsync());
        handlers[LegacyCommandId.RobotFindBest] = Handler(
            EngineState,
            _ => Task.FromResult(FindBestRobot()));
        handlers[LegacyCommandId.RecordingSafemodeBackup] = Handler(
            () => new LegacyCommandState(true, _safemodeBackup),
            _ => Task.FromResult(ToggleSafemodeBackup()));
        handlers[LegacyCommandId.RecordingRecoverLatestAutosave] = Handler(
            () => new LegacyCommandState(
                _session is not null && RecoveryArtifactPath() is not null,
                false,
                _session is null ? "Start a simulation first." : "No autosave recovery artifact is available."),
            async _ => await RecoverLatestAutosaveAsync());
        handlers[LegacyCommandId.RecordingSnapshotLiving] = Handler(
            EngineState,
            async _ => await SaveLivingSnapshotAsync());
        handlers[LegacyCommandId.RecordingDeadEnable] = Handler(
            () => new LegacyCommandState(_session is not null, _deadRecordingEnabled, _session is null ? "Start a simulation first." : null),
            _ => Task.FromResult(ToggleDeadRecording()));
        handlers[LegacyCommandId.RecordingDeadExcludeRepopulating] = Handler(
            () => new LegacyCommandState(_session is not null, _excludeRepopulatingDead, _session is null ? "Start a simulation first." : null),
            _ => Task.FromResult(ToggleDeadRecordingFilter()));
        handlers[LegacyCommandId.EGridEnable] = ViewportToggle(
            () => Viewport.EnergyGridEnabled,
            value => Viewport.EnergyGridEnabled = value,
            "E-GRID");
        handlers[LegacyCommandId.EGridLarge] = EnergyGridSizeHandler(EnergyGridSize.Large);
        handlers[LegacyCommandId.EGridMedium] = EnergyGridSizeHandler(EnergyGridSize.Medium);
        handlers[LegacyCommandId.EGridSmall] = EnergyGridSizeHandler(EnergyGridSize.Small);
        handlers[LegacyCommandId.InternetEnabled] = DelegateLegacyCommandHandler.Disabled(offline);
        handlers[LegacyCommandId.InternetTeleporter] = DelegateLegacyCommandHandler.Disabled(offline);
        handlers[LegacyCommandId.EditInternetOptions] = DelegateLegacyCommandHandler.Disabled(offline);
        handlers[LegacyCommandId.HelpDna] = Handler(() => new LegacyCommandState(true, false), _ => ShowDocumentAsync(
            "DNA Help", "Legacy DNA quick reference",
            "Darwinbots DNA is plain text. Conditions appear between cond and start; actions appear between start and stop. Values are pushed onto the stack, sysvars are read with * and written with store.\n\nExample:\ncond\n*.eye5 0 >\nstart\n30 .up store\nstop\nend\n\nApostrophes begin comments. Imported bot files keep the historical .txt format and sysvar names."));
        handlers[LegacyCommandId.HelpRobotTagInformation] = Handler(() => new LegacyCommandState(true, false), _ => ShowDocumentAsync(
            "Robot Tag Information", "Species and lineage identity",
            "A robot retains a stable slot:generation identity while alive. Species name identifies its imported DNA family. Lineage color drifts only when automatic speciation creates a descendant lineage, allowing related organisms to be recognized visually.\n\nAutotroph species use green hues. Heterotroph species use the remaining palette."));
        handlers[LegacyCommandId.HelpSurvivalInfo] = Handler(() => new LegacyCommandState(true, false, Visible: false), _ => ShowDocumentAsync(
            "Survival Info", "Reading the ecosystem",
            "Energy keeps a bot alive and powers movement, shots, and reproduction. Body is a durable energy reserve and influences rendered size. Vegetables gain energy from chloroplasts and sunlight; animals must feed or exchange energy.\n\nPopulation, births, deaths, energy, mutation, shots, and feeding totals are recorded below the world view."));
        handlers[LegacyCommandId.HelpAbout] = Handler(() => new LegacyCommandState(true, false), _ => ShowDocumentAsync(
            "About Darwinbots Modern", "Behavioral modernization",
            "Darwinbots Modern is a cross-platform continuation of Darwinbots 2. It preserves legacy text DNA and sysvars while running a Rust simulation engine behind an Avalonia desktop interface. CPU and GPU backends share the same simulation contract.\n\nThe VB6 Darwinbots 2 runtime remains the behavioral authority for compatibility decisions."));

        return new LegacyCommandRouter(handlers);
    }

    private LegacyCommandState EngineState() => _session is null
        ? new LegacyCommandState(false, false, "The simulation engine is not ready.")
        : LegacyCommandState.Available;

    private LegacyCommandState SelectedRobotState() => _viewModel.SelectedOrganism is null
        ? new LegacyCommandState(false, false, "Select a robot first.")
        : LegacyCommandState.Available;

    private LegacyCommandState FeatureState(WorldFeatureKind kind) =>
        _selectedFeature is { Kind: var selectedKind } && selectedKind == kind
            ? LegacyCommandState.Available
            : new LegacyCommandState(false, false, $"Select a {kind.ToString().ToLowerInvariant()} first.");

    private static DelegateLegacyCommandHandler Handler(
        Func<LegacyCommandState> state,
        Func<CancellationToken, Task<LegacyCommandResult>> execute) => new(state, execute);

    private DelegateLegacyCommandHandler ViewportToggle(
        Func<bool> current,
        Action<bool> set,
        string label) => Handler(
        () => new LegacyCommandState(true, current()),
        _ =>
        {
            var enabled = !current();
            set(enabled);
            return Task.FromResult(LegacyCommandResult.Success($"{label} {(enabled ? "ON" : "OFF")}"));
        });

    private DelegateLegacyCommandHandler EnergyGridSizeHandler(EnergyGridSize size) => Handler(
        () => new LegacyCommandState(true, Viewport.EnergyGridSize == size),
        _ =>
        {
            Viewport.EnergyGridSize = size;
            return Task.FromResult(LegacyCommandResult.Success($"E-GRID SIZE · {size.ToString().ToUpperInvariant()}"));
        });

    private DelegateLegacyCommandHandler MazeHandler(MazePattern pattern) => Handler(
        EngineState,
        async _ => await AddMazeCommandAsync(pattern));

    private async void HistoricMenuBar_CommandInvoked(object? sender, LegacyCommandInvokedEventArgs e)
    {
        var result = await _legacyCommands.ExecuteAsync(e.Command);
        _viewModel.Status = result.Status;
        RefreshLegacyMenuStates();
    }

    private void RefreshLegacyMenuStates() =>
        HistoricMenuBar.RefreshStates(_legacyCommands.GetState);

    private LegacyCommandResult PauseSimulation()
    {
        _runTimer.Stop();
        return LegacyCommandResult.Success("PAUSED");
    }

    private LegacyCommandResult ToggleVisionGrid()
    {
        Viewport.ShowSelectedVision = !Viewport.ShowSelectedVision;
        ShowVisionToggle.IsChecked = Viewport.ShowSelectedVision;
        return LegacyCommandResult.Success(Viewport.ShowSelectedVision ? "VISION GRID ON" : "VISION GRID OFF");
    }

    private LegacyCommandResult BeginEnvironmentPlacement(EnvironmentPlacement placement)
    {
        _environmentPlacement = placement;
        _placementOrigin = null;
        return LegacyCommandResult.Success(placement == EnvironmentPlacement.Obstacle
            ? "OBSTACLE · CLICK FIRST CORNER"
            : "TELEPORTER · CLICK SOURCE");
    }

    private async Task<LegacyCommandResult> KillSelectedAsync()
    {
        if (_session is null) return LegacyCommandResult.Failure("ENGINE IS NOT READY");
        if (_viewModel.SelectedOrganism is not { } organism)
            return LegacyCommandResult.Failure("SELECT A ROBOT FIRST");
        await _session.RemoveAsync(organism.Slot, organism.Generation);
        Viewport.SelectSlot(null);
        return LegacyCommandResult.Success("ROBOT KILLED");
    }

    private async Task<LegacyCommandResult> KillSelectedOrganismAsync()
    {
        if (_session is null) return LegacyCommandResult.Failure("ENGINE IS NOT READY");
        var group = SelectedOrganismGroup();
        if (group.Count == 0) return LegacyCommandResult.Failure("SELECT A ROBOT FIRST");
        OrganismKey[] keys = [.. group.Select(value => new OrganismKey(value.Slot, value.Generation))];
        await _session.RemoveManyAsync(keys);
        _viewModel.ClearSelection();
        Viewport.SelectSlot(null);
        return LegacyCommandResult.Success($"ORGANISM KILLED · {keys.Length:N0} BOT(S)");
    }

    private LegacyCommandResult OpenNewSimulationSetup()
    {
        new SetupWindow(_storage).Show();
        Close();
        return LegacyCommandResult.Success("NEW SIMULATION SETUP");
    }

    private LegacyCommandResult OpenLeagueTab()
    {
        LeagueExpander.IsExpanded = true;
        LeagueExpander.BringIntoView();
        return LegacyCommandResult.Success("LEAGUE TAB OPEN");
    }

    private LegacyCommandResult QuitApplication()
    {
        Close();
        return LegacyCommandResult.Success("QUIT");
    }

    private async Task<LegacyCommandResult> SaveSimulationCommandAsync(string status, bool withoutMutations)
    {
        if (_session is null) return LegacyCommandResult.Failure("ENGINE IS NOT READY");
        var bytes = withoutMutations
            ? await _session.SaveWithoutMutationsAsync()
            : await _session.SaveAsync();
        return await _storage.SaveSimulationAsync(this, bytes)
            ? LegacyCommandResult.Success(status)
            : LegacyCommandResult.Cancel();
    }

    private async Task<LegacyCommandResult> LoadSimulationCommandAsync()
    {
        if (_session is null) return LegacyCommandResult.Failure("ENGINE IS NOT READY");
        if (await _storage.OpenSimulationAsync(this) is not { } save)
            return LegacyCommandResult.Cancel();
        await _session.LoadAsync(save);
        return LegacyCommandResult.Success("LOADED");
    }

    private async Task<LegacyCommandResult> ImportOrganismsCommandAsync()
    {
        if (_session is null) return LegacyCommandResult.Failure("ENGINE IS NOT READY");
        var files = await _storage.OpenDnaFilesAsync(this);
        if (files.Count == 0) return LegacyCommandResult.Cancel();
        var warnings = new List<string>();
        foreach (var file in files)
        {
            var species = new SpeciesImport(
                Path.GetFileNameWithoutExtension(file.Name), file.Content, false, 0xffd07a2d,
                1, 1_000, 0, false, 1f);
            var report = await _session.ImportSpeciesAsync(species);
            RegisterInsertionSpecies(species);
            warnings.AddRange(report.CompatibilityWarnings);
        }
        return warnings.Count == 0
            ? LegacyCommandResult.Success($"IMPORTED {files.Count} BOT FILE(S)")
            : LegacyCommandResult.Success($"DNA COMPATIBILITY · {string.Join(" · ", warnings.Distinct())}");
    }

    private async Task<LegacyCommandResult> ToggleLegacyRestrictionAsync(bool fixing)
    {
        if (_session is null) return LegacyCommandResult.Failure("ENGINE IS NOT READY");
        var snapshot = _session.LatestSnapshot;
        var fixingAllowed = fixing ? !snapshot.RobotFixingAllowed : snapshot.RobotFixingAllowed;
        var asexualAllowed = fixing ? snapshot.AsexualReproductionAllowed : !snapshot.AsexualReproductionAllowed;
        await _session.UpdateLegacyRestrictionsAsync(fixingAllowed, asexualAllowed);
        return LegacyCommandResult.Success(fixing
            ? $"ROBOT FIXING · {(fixingAllowed ? "ALLOWED" : "DISALLOWED")}"
            : $"ASEXUAL REPRODUCTION · {(asexualAllowed ? "ALLOWED" : "BLOCKED FOR NON-REPOPULATING SPECIES")}");
    }

    private LegacyCommandResult ToggleAutomaticTagging()
    {
        _automaticTagByName = !_automaticTagByName;
        return LegacyCommandResult.Success($"AUTOMATIC NAME TAGS · {(_automaticTagByName ? "ON" : "OFF")}");
    }

    private async Task<LegacyCommandResult> ToggleAutomaticForkingAsync()
    {
        if (_session is null) return LegacyCommandResult.Failure("ENGINE IS NOT READY");
        _liveEnvironment = _liveEnvironment with { AutoSpeciation = !_liveEnvironment.AutoSpeciation };
        await _session.UpdateEnvironmentAsync(_liveEnvironment);
        return LegacyCommandResult.Success($"AUTOMATIC FORKING · {(_liveEnvironment.AutoSpeciation ? "ON" : "OFF")}");
    }

    private async Task<LegacyCommandResult> OpenPlayerBotSettingsAsync()
    {
        var dialog = new LegacyValueWindow("PlayerBot Settings", "Manual movement step", _playerBotThrust.ToString("0.0", System.Globalization.CultureInfo.InvariantCulture), "World units moved per arrow or WASD key press while PlayerBot mode is enabled.");
        await dialog.ShowDialog(this);
        if (!dialog.Accepted) return LegacyCommandResult.Cancel();
        if (!float.TryParse(dialog.Value, System.Globalization.NumberStyles.Float, System.Globalization.CultureInfo.InvariantCulture, out var thrust) || !float.IsFinite(thrust) || thrust <= 0 || thrust > 2_000)
            return LegacyCommandResult.Failure("PLAYERBOT STEP MUST BE BETWEEN 0 AND 2000");
        _playerBotThrust = thrust;
        return LegacyCommandResult.Success($"PLAYERBOT STEP · {_playerBotThrust:0.0}");
    }

    private LegacyCommandResult TogglePlayerBot()
    {
        _playerBotEnabled = !_playerBotEnabled;
        return LegacyCommandResult.Success($"PLAYERBOT MODE · {(_playerBotEnabled ? "ON · USE ARROWS OR WASD" : "OFF")}");
    }

    private async void PlayerBot_KeyDown(object? sender, Avalonia.Input.KeyEventArgs e)
    {
        if (!_playerBotEnabled || _session is null || _viewModel.SelectedOrganism is not { } organism) return;
        var delta = e.Key switch
        {
            Avalonia.Input.Key.Left or Avalonia.Input.Key.A => new[] { -_playerBotThrust, 0f },
            Avalonia.Input.Key.Right or Avalonia.Input.Key.D => new[] { _playerBotThrust, 0f },
            Avalonia.Input.Key.Up or Avalonia.Input.Key.W => new[] { 0f, -_playerBotThrust },
            Avalonia.Input.Key.Down or Avalonia.Input.Key.S => new[] { 0f, _playerBotThrust },
            _ => null,
        };
        if (delta is null) return;
        var world = _session.LatestSnapshot.WorldSize;
        var position = new[]
        {
            Math.Clamp(organism.Position[0] + delta[0], 0, world[0]),
            Math.Clamp(organism.Position[1] + delta[1], 0, world[1]),
        };
        await _session.MoveAsync(organism.Slot, organism.Generation, position);
        e.Handled = true;
    }

    private async Task<LegacyCommandResult> OpenEyeDesignerAsync()
    {
        var organism = _viewModel.SelectedOrganism!;
        var dialog = new LegacyValueWindow("Eye Designer", "Aim angle", organism.Aim.ToString(System.Globalization.CultureInfo.InvariantCulture), "DB2 angular units: 0 through 1256, where approximately 314 is a quarter turn.");
        await dialog.ShowDialog(this);
        if (!dialog.Accepted) return LegacyCommandResult.Cancel();
        if (!int.TryParse(dialog.Value, out var aim)) return LegacyCommandResult.Failure("AIM MUST BE AN INTEGER");
        await _session!.SetOrganismAimAsync(organism.Slot, organism.Generation, aim);
        return LegacyCommandResult.Success($"EYE AIM · {((aim % 1257) + 1257) % 1257}");
    }

    private Task<LegacyCommandResult> ShowRestrictionOverridesAsync()
    {
        var snapshot = _session!.LatestSnapshot;
        return ShowDocumentAsync("Restriction Overwrites", "Active simulation policy", $"Robot-controlled fixing: {(snapshot.RobotFixingAllowed ? "allowed" : "overwritten off")}\nAsexual reproduction for non-repopulating species: {(snapshot.AsexualReproductionAllowed ? "allowed" : "overwritten off")}\nTie creation: {(snapshot.TiesEnabled ? "allowed" : "overwritten off")}\nAutomatic forking/speciation: {(_liveEnvironment.AutoSpeciation ? "enabled" : "disabled")}\n\nThese overrides apply between simulation phases and do not modify bot DNA.");
    }

    private async Task<LegacyCommandResult> OpenRgbMonitorSettingsAsync()
    {
        var dialog = new LegacyValueWindow("RGB Memory Monitor Settings", "Red, green, blue channels", _rgbMemoryChannels, "Enter three comma-separated DB2 values. The modern monitor currently recognizes .nrg, .body, .waste, .chlr, .shell, .slime, .venom, and .poison.");
        await dialog.ShowDialog(this);
        if (!dialog.Accepted) return LegacyCommandResult.Cancel();
        var channels = dialog.Value.Split(',', StringSplitOptions.RemoveEmptyEntries | StringSplitOptions.TrimEntries);
        if (channels.Length != 3) return LegacyCommandResult.Failure("RGB MONITOR REQUIRES THREE CHANNELS");
        _rgbMemoryChannels = string.Join(", ", channels);
        if (_rgbMemoryMonitorEnabled) CloseRgbMonitor();
        return LegacyCommandResult.Success($"RGB CHANNELS · {_rgbMemoryChannels.ToUpperInvariant()}");
    }

    private LegacyCommandResult ToggleRgbMemoryMonitor()
    {
        if (_rgbMemoryMonitorEnabled)
        {
            CloseRgbMonitor();
            return LegacyCommandResult.Success("RGB MEMORY MONITOR · OFF");
        }
        var organism = _viewModel.SelectedOrganism!;
        var channels = _rgbMemoryChannels.Split(',', StringSplitOptions.TrimEntries);
        int Read(string channel) => channel.ToLowerInvariant() switch
        {
            ".nrg" => organism.Energy, ".body" => organism.Body, ".waste" => organism.Waste,
            ".chlr" => organism.Chloroplasts, ".shell" => organism.Shell, ".slime" => organism.Slime,
            ".venom" => organism.Venom, ".poison" => organism.Poison, _ => 0,
        };
        var section = $"Robot {organism.Slot}:{organism.Generation}";
        var content = $"RED   {channels[0]} = {Read(channels[0]):N0}\nGREEN {channels[1]} = {Read(channels[1]):N0}\nBLUE  {channels[2]} = {Read(channels[2]):N0}\n\nThe three values form a diagnostic color signature for the selected robot.";
        var window = _toolWindows.ShowOrActivate(RgbMonitorToolKey, () =>
        {
            var tool = new LegacyDocumentWindow("RGB Memory Monitor", section, content);
            tool.Closed += (_, _) =>
            {
                _rgbMemoryMonitorEnabled = false;
                RefreshLegacyMenuStates();
            };
            return tool;
        });
        window.SetDocument("RGB Memory Monitor", section, content);
        _rgbMemoryMonitorEnabled = true;
        return LegacyCommandResult.Success("RGB MEMORY MONITOR · ON");
    }

    private void CloseRgbMonitor()
    {
        _rgbMemoryMonitorEnabled = false;
        _toolWindows.Close(RgbMonitorToolKey);
    }

    private async Task<LegacyCommandResult> ImportBackgroundAsync()
    {
        var files = await StorageProvider.OpenFilePickerAsync(new Avalonia.Platform.Storage.FilePickerOpenOptions
        {
            Title = "Import Darwinbots background picture",
            AllowMultiple = false,
            FileTypeFilter =
            [
                new Avalonia.Platform.Storage.FilePickerFileType("Images") { Patterns = ["*.png", "*.jpg", "*.jpeg", "*.bmp"] },
            ],
        });
        if (files.Count == 0) return LegacyCommandResult.Cancel();
        await using var stream = await files[0].OpenReadAsync();
        Viewport.SetBackgroundImage(new Avalonia.Media.Imaging.Bitmap(stream));
        _hasBackgroundImage = true;
        return LegacyCommandResult.Success($"BACKGROUND · {files[0].Name}");
    }

    private LegacyCommandResult RemoveBackground()
    {
        Viewport.SetBackgroundImage(null);
        _hasBackgroundImage = false;
        return LegacyCommandResult.Success("BACKGROUND REMOVED");
    }

    private async Task<LegacyCommandResult> ToggleShotDecayAsync(bool energy)
    {
        if (_session is null) return LegacyCommandResult.Failure("ENGINE IS NOT READY");
        var shots = energy
            ? _liveEnvironment.Shots with { EnergyShotsDoNotDecay = !_liveEnvironment.Shots.EnergyShotsDoNotDecay }
            : _liveEnvironment.Shots with { WasteShotsDoNotDecay = !_liveEnvironment.Shots.WasteShotsDoNotDecay };
        _liveEnvironment = _liveEnvironment with { Shots = shots };
        await _session.UpdateEnvironmentAsync(_liveEnvironment);
        var enabled = energy ? shots.EnergyShotsDoNotDecay : shots.WasteShotsDoNotDecay;
        return LegacyCommandResult.Success($"{(energy ? "ENERGY" : "WASTE")} SHOT DECAY · {(enabled ? "OFF" : "ON")}");
    }

    private async Task<LegacyCommandResult> ToggleTiesAsync()
    {
        if (_session is null) return LegacyCommandResult.Failure("ENGINE IS NOT READY");
        var enabled = !_session.LatestSnapshot.TiesEnabled;
        await _session.SetTiesEnabledAsync(enabled);
        return LegacyCommandResult.Success($"TIES · {(enabled ? "ENABLED" : "DISABLED")}");
    }

    private async Task<LegacyCommandResult> SaveLivingSnapshotAsync()
    {
        if (_session is null) return LegacyCommandResult.Failure("ENGINE IS NOT READY");
        var file = await StorageProvider.SaveFilePickerAsync(new Avalonia.Platform.Storage.FilePickerSaveOptions
        {
            Title = "Save snapshot of the living",
            SuggestedFileName = $"darwinbots-living-{_session.LatestSnapshot.Tick}.json",
            DefaultExtension = "json",
            FileTypeChoices = [new Avalonia.Platform.Storage.FilePickerFileType("Darwinbots living snapshot") { Patterns = ["*.json"] }],
        });
        if (file is null) return LegacyCommandResult.Cancel();
        await using var stream = await file.OpenWriteAsync();
        await System.Text.Json.JsonSerializer.SerializeAsync(stream, new
        {
            version = 1,
            tick = _session.LatestSnapshot.Tick,
            world_size = _session.LatestSnapshot.WorldSize,
            organisms = _session.LatestSnapshot.Organisms,
            species = _session.LatestSnapshot.Species,
            stats = _session.LatestSnapshot.Stats,
        }, new System.Text.Json.JsonSerializerOptions { WriteIndented = true });
        return LegacyCommandResult.Success($"LIVING SNAPSHOT · {file.Name}");
    }

    private LegacyCommandResult ToggleDeadRecording()
    {
        _deadRecordingEnabled = !_deadRecordingEnabled;
        _recordedCorpseCount = _session?.LatestSnapshot.Corpses.Count ?? 0;
        return LegacyCommandResult.Success($"DEAD RECORDING · {(_deadRecordingEnabled ? "ON" : "OFF")}");
    }

    private LegacyCommandResult ToggleDeadRecordingFilter()
    {
        _excludeRepopulatingDead = !_excludeRepopulatingDead;
        return LegacyCommandResult.Success($"REPOPULATING DEAD · {(_excludeRepopulatingDead ? "EXCLUDED" : "INCLUDED")}");
    }

    private void RecordDeadSnapshot(EngineSnapshot snapshot)
    {
        if (!_deadRecordingEnabled)
        {
            _recordedCorpseCount = snapshot.Corpses.Count;
            return;
        }
        if (snapshot.Corpses.Count < _recordedCorpseCount) _recordedCorpseCount = 0;
        if (snapshot.Corpses.Count == _recordedCorpseCount) return;
        try
        {
            var directory = Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData), "Darwinbots Modern", "records");
            Directory.CreateDirectory(directory);
            var path = Path.Combine(directory, "dead.jsonl");
            var lines = snapshot.Corpses.Skip(_recordedCorpseCount)
                .Where(corpse => !_excludeRepopulatingDead || !corpse.Repopulating)
                .Select(corpse => System.Text.Json.JsonSerializer.Serialize(new { version = 1, tick = snapshot.Tick, corpse }))
                .ToArray();
            File.AppendAllLines(path, lines);
            _recordedCorpseCount = snapshot.Corpses.Count;
        }
        catch (Exception error)
        {
            _deadRecordingEnabled = false;
            _viewModel.Status = $"DEAD RECORDING FAILED · {error.Message}";
        }
    }

    private Task<LegacyCommandResult> ShowDocumentAsync(string title, string section, string content)
    {
        var key = $"document:{title}";
        var window = _toolWindows.ShowOrActivate(key, () => new LegacyDocumentWindow(title, section, content));
        window.SetDocument(title, section, content);
        return Task.FromResult(LegacyCommandResult.Success(title.ToUpperInvariant()));
    }

    private async Task<LegacyCommandResult> OpenAdvancedSettingsCommandAsync()
    {
        if (_session is null) return LegacyCommandResult.Failure("ENGINE IS NOT READY");
        var dialog = new AdvancedSettingsWindow(_liveEnvironment, _session.LatestSnapshot);
        await dialog.ShowDialog(this);
        if (!dialog.Accepted) return LegacyCommandResult.Cancel();
        await _session.UpdateEnvironmentAsync(dialog.Update);
        await _session.UpdateMutationSettingsAsync(dialog.MutationUpdate);
        _liveEnvironment = dialog.Update;
        return LegacyCommandResult.Success("LIVE SETTINGS APPLIED");
    }

    private async Task<LegacyCommandResult> OpenGlobalSettingsCommandAsync()
    {
        var dialog = new GlobalSettingsWindow(_globalSettings);
        await dialog.ShowDialog(this);
        if (!dialog.Accepted) return LegacyCommandResult.Cancel();
        _globalSettings = dialog.Settings;
        if (_trayIcon is not null)
            _trayIcon.IsVisible = !_globalSettings.Main.HideTray;
        _viewModel.Status = "GLOBAL SETTINGS SAVED FOR NEXT WORLD";
        return LegacyCommandResult.Success("GLOBAL SETTINGS SAVED");
    }

    private WorldSetupOptions CreateNextWorldSetup() => _setup with
    {
        GlobalSettings = _globalSettings,
    };

    private async Task<LegacyCommandResult> ShowSelectedRobotInfoAsync()
    {
        var organism = _viewModel.SelectedOrganism!;
        var session = _session!;
        Viewport.SelectSlot(organism.Slot);
        ShowRobotDetails(session, organism, RobotDetailsTab.Facts);
        return LegacyCommandResult.Success("ROBOT DETAILS OPEN");
    }

    private RobotDetailsWindow ShowRobotDetails(
        SimulationSession session,
        OrganismSnapshot organism,
        RobotDetailsTab tab)
    {
        var key = $"robot-details:{organism.Slot}:{organism.Generation}";
        var window = _toolWindows.ShowOrActivate(
            key,
            () => new RobotDetailsWindow(
                session,
                new OrganismKey(organism.Slot, organism.Generation),
                tab));
        window.SelectTab(tab);
        window.SetSnapshot(session.LatestSnapshot);
        return window;
    }

    private async Task<LegacyCommandResult> ShowSelectedPhylogenyAsync()
    {
        var organism = _viewModel.SelectedOrganism!;
        var snapshot = _session!.LatestSnapshot;
        var parents = organism.Parents is { Length: > 0 }
            ? string.Join("\n", organism.Parents.Select((parent, index) => parent is { } value ? $"Parent {index + 1}: {value.Slot}:{value.Generation}" : $"Parent {index + 1}: unknown"))
            : "No recorded parents (founder or legacy import).";
        var relatives = snapshot.Organisms.Count(value => value.Phenotype.LineageId == organism.Phenotype.LineageId);
        return await ShowDocumentAsync("Show Philogeny", "Lineage record", $"Robot: {organism.Slot}:{organism.Generation}\nLineage ID: {organism.Phenotype.LineageId}\nLiving lineage members: {relatives:N0}\nAccumulated mutations: {organism.Phenotype.AccumulatedMutations:N0}\n\n{parents}");
    }

    private async Task<LegacyCommandResult> ChangeSelectedSpeciesColorAsync()
    {
        var organism = _viewModel.SelectedOrganism!;
        var species = _session!.LatestSnapshot.Species.ElementAtOrDefault((int)organism.Species);
        var dialog = new LegacyValueWindow("Change Color", "Species color", $"#{(species?.Color ?? organism.Phenotype.Color) & 0x00ffffff:X6}", "Enter a six-digit RGB hexadecimal color, for example #239AC0.");
        await dialog.ShowDialog(this);
        if (!dialog.Accepted) return LegacyCommandResult.Cancel();
        var text = dialog.Value.TrimStart('#');
        if (!uint.TryParse(text, System.Globalization.NumberStyles.HexNumber, null, out var rgb) || text.Length != 6)
            return LegacyCommandResult.Failure("COLOR MUST BE SIX HEX DIGITS");
        await _session.SetSpeciesColorAsync(organism.Species, 0xff000000 | rgb);
        return LegacyCommandResult.Success($"SPECIES COLOR · #{rgb:X6}");
    }

    private async Task<LegacyCommandResult> ShowSelectedRobotConsoleAsync()
    {
        var organism = _viewModel.SelectedOrganism!;
        return await ShowDocumentAsync("Robot Console", $"Live robot {organism.Slot}:{organism.Generation}", $".nrg = {organism.Energy}\n.body = {organism.Body}\n.waste = {organism.Waste}\n.chlr = {organism.Chloroplasts}\n.aim = {organism.Aim}\n.paralyzed = {organism.Paralyzed}\n.poisoned = {organism.Poisoned}\n\nPosition = [{organism.Position[0]:0.00}, {organism.Position[1]:0.00}]\nVelocity = [{organism.Velocity[0]:0.00}, {organism.Velocity[1]:0.00}]\n\nThis console is a read-only live memory summary. Use Edit / Export DNA to change the active program.");
    }

    private async Task<LegacyCommandResult> ShowSelectedGeneActivationsAsync()
    {
        var organism = _viewModel.SelectedOrganism!;
        ShowRobotDetails(_session!, organism, RobotDetailsTab.GeneActivations);
        return LegacyCommandResult.Success("GENE ACTIVATIONS OPEN");
    }

    private async Task<LegacyCommandResult> MakeSelectedNewSpeciesAsync()
    {
        var organism = _viewModel.SelectedOrganism!;
        var source = _session!.LatestSnapshot.Species.ElementAtOrDefault((int)organism.Species);
        var dialog = new LegacyValueWindow("Make New Species", "New species name", $"{source?.Name ?? "Species"} descendant", "Only the selected robot moves to the new species; its DNA and current state are preserved.");
        await dialog.ShowDialog(this);
        if (!dialog.Accepted) return LegacyCommandResult.Cancel();
        if (string.IsNullOrWhiteSpace(dialog.Value)) return LegacyCommandResult.Failure("SPECIES NAME IS REQUIRED");
        var sourceColor = source?.Color ?? organism.Phenotype.Color;
        var color = 0xff000000 | (((sourceColor & 0x00ffffff) + 0x00130d17) & 0x00ffffff);
        await _session.MakeNewSpeciesAsync(organism.Slot, organism.Generation, dialog.Value, color);
        return LegacyCommandResult.Success($"NEW SPECIES · {dialog.Value.ToUpperInvariant()}");
    }

    private IReadOnlyList<OrganismSnapshot> SelectedOrganismGroup()
    {
        var selected = _viewModel.SelectedOrganism;
        if (selected is null || _session is null) return [];
        var snapshot = _session.LatestSnapshot;
        var snapshotKeys = snapshot.Organisms
            .Select(value => new OrganismKey(value.Slot, value.Generation))
            .ToHashSet();
        var selectedKey = new OrganismKey(selected.Slot, selected.Generation);
        if (!snapshotKeys.Contains(selectedKey)) return [];
        var pending = new Queue<OrganismKey>();
        var memberKeys = new HashSet<OrganismKey> { selectedKey };
        pending.Enqueue(selectedKey);
        while (pending.Count > 0)
        {
            var current = pending.Dequeue();
            foreach (var tie in snapshot.Ties.Where(value => value.First == current || value.Second == current))
            {
                var other = tie.First == current ? tie.Second : tie.First;
                if (snapshotKeys.Contains(other) && memberKeys.Add(other)) pending.Enqueue(other);
            }
        }
        return snapshot.Organisms
            .Where(value => memberKeys.Contains(new OrganismKey(value.Slot, value.Generation)))
            .ToArray();
    }

    private LegacyCommandResult SelectEntireOrganism()
    {
        var group = SelectedOrganismGroup();
        var selected = _viewModel.SelectedOrganism!;
        Viewport.SelectSlots(group.Select(value => value.Slot), selected.Slot);
        return LegacyCommandResult.Success($"ORGANISM SELECTED · {group.Count:N0} BOT(S)");
    }

    private async Task<LegacyCommandResult> SaveEntireOrganismAsync()
    {
        var group = SelectedOrganismGroup();
        if (_session is null || group.Count == 0) return LegacyCommandResult.Failure("NO ORGANISM SELECTED");
        var memberKeys = group.Select(value => new OrganismKey(value.Slot, value.Generation)).ToHashSet();
        var snapshot = _session.LatestSnapshot;
        var ties = FilterTiesForCapturedGroup(snapshot.Ties, memberKeys);
        var saved = await _storage.SaveOrganismAsync(this, $"organism-{_viewModel.SelectedSlot}.json", async stream =>
        {
            var members = new List<object>();
            foreach (var member in group)
            {
                var dna = await _session.ExportDnaAsync(member.Slot, member.Generation);
                members.Add(new { member.Slot, member.Generation, member.Species, member.Position, member.Velocity, member.Energy, member.Body, dna });
            }
            await System.Text.Json.JsonSerializer.SerializeAsync(stream, new { version = 1, tick = snapshot.Tick, members, ties }, new System.Text.Json.JsonSerializerOptions { WriteIndented = true });
        });
        return saved
            ? LegacyCommandResult.Success($"ORGANISM SAVED · {group.Count:N0} BOT(S)")
            : LegacyCommandResult.Cancel();
    }

    private static TieSnapshot[] FilterTiesForCapturedGroup(
        IReadOnlyList<TieSnapshot> ties,
        IReadOnlySet<OrganismKey> memberKeys) =>
        ties.Where(tie => memberKeys.Contains(tie.First) && memberKeys.Contains(tie.Second)).ToArray();

    private async Task<LegacyCommandResult> SaveSelectedDnaCommandAsync()
    {
        if (_session is null || _viewModel.SelectedOrganism is not { } organism)
            return LegacyCommandResult.Failure("SELECT A ROBOT FIRST");
        var dna = await _session.ExportDnaAsync(organism.Slot, organism.Generation);
        var species = organism.Species < _session.LatestSnapshot.Species.Count
            ? _session.LatestSnapshot.Species[(int)organism.Species].Name
            : $"Robot-{organism.Slot}-{organism.Generation}";
        var suggestedName = _automaticTagByName ? $"{species}.txt" : $"Robot-{organism.Slot}-{organism.Generation}.txt";
        return await _storage.SaveDnaAsync(this, suggestedName, dna)
            ? LegacyCommandResult.Success("ROBOT DNA SAVED")
            : LegacyCommandResult.Cancel();
    }

    private LegacyCommandResult FindBestRobot()
    {
        if (_session?.LatestSnapshot.Organisms.MaxBy(organism => organism.Energy) is not { } organism)
            return LegacyCommandResult.Failure("NO LIVING ROBOTS");
        _viewModel.Select(organism.Slot);
        Viewport.SelectSlot(organism.Slot);
        return LegacyCommandResult.Success($"BEST ENERGY · {organism.Slot}:{organism.Generation} · {organism.Energy:0}");
    }

    private async Task<LegacyCommandResult> RemoveSelectedFeatureCommandAsync()
    {
        if (_session is null || _selectedFeature is not { } feature)
            return LegacyCommandResult.Failure("SELECT A WORLD OBJECT FIRST");
        if (feature.Kind == WorldFeatureKind.Obstacle) await _session.RemoveObstacleAsync(feature.Id);
        else if (_session.LatestSnapshot.FileTeleporters.Any(value => value.Id == feature.Id)) await _session.RemoveFileTeleporterAsync(feature.Id);
        else await _session.RemoveTeleporterAsync(feature.Id);
        _selectedFeature = null;
        Viewport.SelectFeature(null);
        RefreshFeatureEditorButtons();
        return LegacyCommandResult.Success($"{feature.Kind.ToString().ToUpperInvariant()} {feature.Id} REMOVED");
    }

    private async Task<LegacyCommandResult> DeleteAllTeleportersCommandAsync()
    {
        if (_session is null) return LegacyCommandResult.Failure("ENGINE IS NOT READY");
        var teleporters = _session.LatestSnapshot.Teleporters.ToArray();
        var fileTeleporters = _session.LatestSnapshot.FileTeleporters.ToArray();
        foreach (var teleporter in teleporters) await _session.RemoveTeleporterAsync(teleporter.Id);
        foreach (var teleporter in fileTeleporters) await _session.RemoveFileTeleporterAsync(teleporter.Id);
        return LegacyCommandResult.Success($"DELETED {teleporters.Length + fileTeleporters.Length} TELEPORTER(S)");
    }

    private async Task<LegacyCommandResult> DeleteAllObstaclesCommandAsync()
    {
        if (_session is null) return LegacyCommandResult.Failure("ENGINE IS NOT READY");
        var obstacles = _session.LatestSnapshot.Obstacles.ToArray();
        foreach (var obstacle in obstacles) await _session.RemoveObstacleAsync(obstacle.Id);
        return LegacyCommandResult.Success($"DELETED {obstacles.Length} SHAPE(S)");
    }

    private async Task<LegacyCommandResult> AddRandomObstaclesCommandAsync(int count)
    {
        if (_session is null) return LegacyCommandResult.Failure("ENGINE IS NOT READY");
        var random = new Random(unchecked((int)_session.LatestSnapshot.Tick + 0x4d42));
        var width = (float)_setup.WorldWidth;
        var height = (float)_setup.WorldHeight;
        var shapes = Enumerable.Range(0, count).Select(_ =>
        {
            var shapeWidth = width * (0.025f + (float)random.NextDouble() * 0.09f);
            var shapeHeight = height * (0.015f + (float)random.NextDouble() * 0.07f);
            var x = (float)random.NextDouble() * Math.Max(1, width - shapeWidth);
            var y = (float)random.NextDouble() * Math.Max(1, height - shapeHeight);
            return (Minimum: new[] { x, y }, Maximum: new[] { x + shapeWidth, y + shapeHeight });
        });
        await AddObstaclesAsync(shapes);
        return LegacyCommandResult.Success($"ADDED {count} RANDOM SHAPES");
    }

    private async Task<LegacyCommandResult> DeleteRandomObstaclesCommandAsync(int count)
    {
        if (_session is null) return LegacyCommandResult.Failure("ENGINE IS NOT READY");
        var selected = _session.LatestSnapshot.Obstacles
            .OrderBy(obstacle => HashCode.Combine(obstacle.Id, _session.LatestSnapshot.Tick))
            .Take(count)
            .ToArray();
        foreach (var obstacle in selected) await _session.RemoveObstacleAsync(obstacle.Id);
        return LegacyCommandResult.Success($"DELETED {selected.Length} RANDOM SHAPES");
    }

    private async Task<LegacyCommandResult> AddMazeCommandAsync(MazePattern pattern)
    {
        if (_session is null) return LegacyCommandResult.Failure("ENGINE IS NOT READY");
        var width = (float)_setup.WorldWidth;
        var height = (float)_setup.WorldHeight;
        var thickness = Math.Max(20f, Math.Min(width, height) * 0.012f);
        var shapes = new List<(float[] Minimum, float[] Maximum)>();
        void Add(float x1, float y1, float x2, float y2) =>
            shapes.Add((new[] { x1, y1 }, new[] { x2, y2 }));

        switch (pattern)
        {
            case MazePattern.Horizontal:
                for (var row = 1; row <= 6; row++)
                {
                    var y = height * row / 7f;
                    if (row % 2 == 0) Add(0, y, width * 0.78f, y + thickness);
                    else Add(width * 0.22f, y, width, y + thickness);
                }
                break;
            case MazePattern.Vertical:
                for (var column = 1; column <= 6; column++)
                {
                    var x = width * column / 7f;
                    if (column % 2 == 0) Add(x, 0, x + thickness, height * 0.78f);
                    else Add(x, height * 0.22f, x + thickness, height);
                }
                break;
            case MazePattern.Spiral:
                for (var ring = 0; ring < 4; ring++)
                {
                    var left = width * (0.12f + ring * 0.1f);
                    var right = width - left;
                    var top = height * (0.12f + ring * 0.1f);
                    var bottom = height - top;
                    Add(left, top, right, top + thickness);
                    Add(right - thickness, top, right, bottom);
                    Add(left + width * 0.08f, bottom - thickness, right, bottom);
                    if (ring < 3) Add(left, top + height * 0.08f, left + thickness, bottom);
                }
                break;
            case MazePattern.Checkerboard:
                for (var y = 0; y < 5; y++)
                for (var x = 0; x < 7; x++)
                    if ((x + y) % 2 == 0)
                        Add(width * x / 7f, height * y / 5f, width * (x + 0.58f) / 7f, height * (y + 0.58f) / 5f);
                break;
            case MazePattern.PolarIce:
                Add(0, 0, width * 0.42f, height * 0.12f);
                Add(width * 0.58f, 0, width, height * 0.12f);
                Add(0, height * 0.88f, width * 0.42f, height);
                Add(width * 0.58f, height * 0.88f, width, height);
                break;
            case MazePattern.TrashCompactor:
                Add(0, 0, width * 0.16f, height);
                Add(width * 0.84f, 0, width, height);
                for (var row = 0; row < 6; row++)
                {
                    var y = height * (row + 0.5f) / 6f;
                    Add(width * 0.16f, y, width * 0.3f, y + thickness);
                    Add(width * 0.7f, y, width * 0.84f, y + thickness);
                }
                break;
        }

        await AddObstaclesAsync(shapes);
        return LegacyCommandResult.Success($"{pattern.ToString().ToUpperInvariant()} MAZE ADDED · {shapes.Count} SHAPES");
    }

    private async Task AddObstaclesAsync(IEnumerable<(float[] Minimum, float[] Maximum)> shapes)
    {
        if (_session is null) return;
        var id = _session.LatestSnapshot.Obstacles.Select(obstacle => obstacle.Id).DefaultIfEmpty().Max() + 1;
        foreach (var shape in shapes)
            await _session.AddObstacleAsync(new ObstacleSnapshot(id++, shape.Minimum, shape.Maximum));
    }

    private LegacyCommandResult ToggleSafemodeBackup()
    {
        _safemodeBackup = !_safemodeBackup;
        if (_safemodeBackup) _autosaveTimer.Start();
        else _autosaveTimer.Stop();
        return LegacyCommandResult.Success(_safemodeBackup ? "SAFEMODE BACKUP ON" : "SAFEMODE BACKUP OFF");
    }

    private string? RecoveryArtifactPath()
    {
        if (_diagnostics.RecoveryArtifactPath is { } remembered && File.Exists(remembered))
            return remembered;
        return _storage.FindLatestAutosave();
    }

    private async Task<LegacyCommandResult> RecoverLatestAutosaveAsync()
    {
        if (_session is null)
            return LegacyCommandResult.Failure("ENGINE IS NOT READY");
        var path = RecoveryArtifactPath();
        if (path is null)
            return LegacyCommandResult.Failure("NO AUTOSAVE RECOVERY ARTIFACT");

        try
        {
            _runTimer.Stop();
            await _session.LoadAsync(await File.ReadAllBytesAsync(path));
            _viewModel.Status = $"RECOVERED · {Path.GetFileName(path)}";
            return LegacyCommandResult.Success(_viewModel.Status);
        }
        catch (Exception error)
        {
            await HandleEngineErrorAsync(error, attemptRecovery: false);
            return LegacyCommandResult.Failure(_viewModel.Status);
        }
    }

    private async Task HandleEngineErrorAsync(Exception error, bool attemptRecovery)
    {
        var tick = _session?.LatestSnapshot.Tick ?? 0;
        if (_diagnostics.IgnoreErrors)
        {
            var ignored = _diagnostics.HandleError(error, tick, null);
            _viewModel.Status = ignored.Status;
            RefreshLegacyMenuStates();
            return;
        }

        _runTimer.Stop();
        var artifact = RecoveryArtifactPath();
        if (attemptRecovery && _safemodeBackup && _session is not null)
        {
            try
            {
                await _storage.SaveAutosaveAsync(await _session.SaveAsync());
                artifact = _storage.FindLatestAutosave() ?? artifact;
            }
            catch
            {
                // Keep the previous artifact available when the failing engine cannot be saved again.
            }
        }

        var decision = _diagnostics.HandleError(error, tick, artifact);
        _viewModel.Status = decision.Status;
        RefreshLegacyMenuStates();
    }

    private async void Window_Opened(object? sender, EventArgs e)
    {
        try
        {
            _runtimeClock.Restart();
            var worldSetup = CreateNextWorldSetup();
            _activeCostMultiplier = worldSetup.GlobalSettings.Costs.GlobalMultiplier;
            _session = new SimulationSession(_engineFactory.Create(worldSetup));
            _insertion = new WorldInsertionController(_session);
            _session.SnapshotPublished += snapshot => Dispatcher.UIThread.Post(() =>
            {
                _viewModel.Update(snapshot, _runtimeClock.Elapsed, _activeCostMultiplier);
                Viewport.SetSnapshot(snapshot);
                Viewport.SelectSlot(_viewModel.SelectedSlot);
                PopulationChart.SetSnapshot(snapshot);
                EnergyChart.SetSnapshot(snapshot);
                _toolWindows.PublishSnapshot(snapshot);
                RecordDeadSnapshot(snapshot);
                RefreshLegacyMenuStates();
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
            await HandleEngineErrorAsync(error, attemptRecovery: true);
        }
    }

    private void RegisterInsertionSpecies(SpeciesImport species)
    {
        var existing = _insertionSpecies.FirstOrDefault(value =>
            value.Name.Equals(species.Name, StringComparison.OrdinalIgnoreCase)
            && value.Dna.Equals(species.Dna, StringComparison.Ordinal));
        if (existing is not null)
        {
            InsertionSpeciesSelector.SelectedItem = existing;
            return;
        }

        _insertionSpecies.Add(species);
        InsertionSpeciesSelector.SelectedItem = species;
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
        try
        {
            await _session.StepAsync();
        }
        catch (Exception error)
        {
            await HandleEngineErrorAsync(error, attemptRecovery: true);
        }
    }

    private async void Kill_Click(object? sender, RoutedEventArgs e)
    {
        _viewModel.Status = (await KillSelectedAsync()).Status;
        RefreshLegacyMenuStates();
    }

    private void Move_Click(object? sender, RoutedEventArgs e)
    {
        if (_viewModel.SelectedOrganism is not { } organism) return;
        _moveOrganism = new OrganismKey(organism.Slot, organism.Generation);
        _viewModel.Status = "CLICK AN EMPTY WORLD POSITION";
    }

    private void InsertSpecies_Click(object? sender, RoutedEventArgs e)
    {
        if (_session is null)
        {
            _viewModel.Status = "ENGINE IS NOT READY";
            return;
        }

        _insertion ??= new WorldInsertionController(_session);
        if (_insertion.IsActive)
        {
            _insertion.Cancel();
            InsertSpeciesButton.Content = "INSERT SPECIES";
            _viewModel.Status = "INSERTION CANCELLED";
            return;
        }

        if (InsertionSpeciesSelector.SelectedItem is not SpeciesImport species)
        {
            _viewModel.Status = "SELECT AN INSTALLED OR IMPORTED SPECIES";
            return;
        }

        try
        {
            _insertion.SelectSpecies(species);
            _insertion.Begin();
            _moveOrganism = null;
            _environmentPlacement = EnvironmentPlacement.None;
            _placementOrigin = null;
            InsertSpeciesButton.Content = "CANCEL INSERTION";
            _viewModel.Status = $"INSERT {species.Name.ToUpperInvariant()} · CLICK EMPTY WORLD";
        }
        catch (Exception error)
        {
            _viewModel.Status = $"INSERTION FAILED · {error.Message}";
        }
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
        _viewModel.Status = PauseSimulation().Status;
    }

    private void Reset_Click(object? sender, RoutedEventArgs e)
    {
        _runTimer.Stop();
        var replacement = new MainWindow(_startupBots, CreateNextWorldSetup(), _engineFactory, _storage);
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
        catch (Exception error) { await HandleEngineErrorAsync(error, attemptRecovery: true); }
    }

    private void AddObstacle_Click(object? sender, RoutedEventArgs e)
    {
        _viewModel.Status = BeginEnvironmentPlacement(EnvironmentPlacement.Obstacle).Status;
    }

    private void AddTeleporter_Click(object? sender, RoutedEventArgs e)
    {
        _viewModel.Status = BeginEnvironmentPlacement(EnvironmentPlacement.Teleporter).Status;
    }

    private async void AddFileTeleporter_Click(object? sender, RoutedEventArgs e)
    {
        if (_session is null) return;
        var world = _session.LatestSnapshot.WorldSize;
        var id = _session.LatestSnapshot.Teleporters
            .Concat(_session.LatestSnapshot.FileTeleporters.Select(value => new TeleporterSnapshot(value.Id, value.Center, value.Radius, value.Center)))
            .Select(value => value.Id)
            .DefaultIfEmpty()
            .Max() + 1;
        var draft = new FileTeleporterSnapshot(
            id,
            [world.ElementAtOrDefault(0) * 0.5f, world.ElementAtOrDefault(1) * 0.5f],
            180f,
            string.Empty,
            string.Empty,
            1,
            0,
            1,
            new TeleporterFilterSnapshot(true, true, true),
            0,
            0);
        var dialog = new FileTeleporterEditorWindow(draft);
        await dialog.ShowDialog(this);
        if (dialog.Result is not { } result) return;
        await _session.AddFileTeleporterAsync(result);
        _viewModel.Status = $"FILE TELEPORTER {result.Id} ADDED · OFFLINE";
    }

    private async void EditShape_Click(object? sender, RoutedEventArgs e)
    {
        if (_session is null || _selectedFeature is not { Kind: WorldFeatureKind.Obstacle } feature) return;
        var obstacle = _session.LatestSnapshot.Obstacles.FirstOrDefault(value => value.Id == feature.Id);
        if (obstacle is null) return;
        var dialog = new ShapeEditorWindow(obstacle);
        await dialog.ShowDialog(this);
        if (dialog.Result is not { } result) return;
        await _session.UpdateObstacleAsync(result);
        _viewModel.Status = $"SHAPE {result.Id} UPDATED";
    }

    private async void EditTeleporter_Click(object? sender, RoutedEventArgs e)
    {
        if (_session is null || _selectedFeature is not { Kind: WorldFeatureKind.Teleporter } feature) return;
        var teleporter = _session.LatestSnapshot.FileTeleporters.FirstOrDefault(value => value.Id == feature.Id);
        if (teleporter is null)
        {
            _viewModel.Status = "SELECT AN OFFLINE FILE TELEPORTER";
            return;
        }
        var dialog = new FileTeleporterEditorWindow(teleporter);
        await dialog.ShowDialog(this);
        if (dialog.Result is not { } result) return;
        await _session.UpdateFileTeleporterAsync(result);
        _viewModel.Status = $"FILE TELEPORTER {result.Id} UPDATED · OFFLINE";
    }

    private async void RemoveFeature_Click(object? sender, RoutedEventArgs e)
    {
        if (_session is null || _selectedFeature is not { } feature) return;
        if (feature.Kind == WorldFeatureKind.Obstacle) await _session.RemoveObstacleAsync(feature.Id);
        else if (_session.LatestSnapshot.FileTeleporters.Any(value => value.Id == feature.Id)) await _session.RemoveFileTeleporterAsync(feature.Id);
        else await _session.RemoveTeleporterAsync(feature.Id);
        _selectedFeature = null;
        Viewport.SelectFeature(null);
        RefreshFeatureEditorButtons();
        _viewModel.Status = $"{feature.Kind.ToString().ToUpperInvariant()} {feature.Id} REMOVED";
    }

    private void RefreshFeatureEditorButtons()
    {
        var selectedObstacle = _selectedFeature is { Kind: WorldFeatureKind.Obstacle };
        var selectedFileTeleporter = _selectedFeature is { Kind: WorldFeatureKind.Teleporter } feature
            && _session?.LatestSnapshot.FileTeleporters.Any(value => value.Id == feature.Id) == true;
        EditShapeButton.IsEnabled = selectedObstacle;
        EditTeleporterButton.IsEnabled = selectedFileTeleporter;
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

    private void LocalModeKind_Changed(object? sender, SelectionChangedEventArgs e)
    {
        if (!_controlsReady) return;
        _viewModel.LocalMode.SelectKind(LocalModeKindFromIndex(LocalModeKindSelector.SelectedIndex));
    }

    private void LocalModePreset_Changed(object? sender, SelectionChangedEventArgs e)
    {
        if (!_controlsReady) return;
        _viewModel.LocalMode.Preset = PresetFromIndex(LocalModePreset.SelectedIndex);
        _viewModel.LocalMode.MarkConfigured();
    }

    private async void ConfigureLocalMode_Click(object? sender, RoutedEventArgs e)
    {
        if (_session is null)
        {
            _viewModel.Status = "ENGINE IS NOT READY";
            return;
        }
        try
        {
            SynchronizeLocalModeControls();
            var config = _viewModel.LocalMode.BuildConfiguration();
            await _session.ConfigureLocalModeAsync(config);
            _viewModel.LocalMode.MarkConfigured();
            _viewModel.Status = $"LOCAL MODE CONFIGURED · {config.Kind.ToString().ToUpperInvariant()}";
        }
        catch (Exception error) { _viewModel.Status = $"LOCAL MODE CONFIG FAILED · {error.Message}"; }
    }

    private async void StartLocalMode_Click(object? sender, RoutedEventArgs e)
    {
        if (_session is null) return;
        try
        {
            await _session.StartLocalModeAsync();
            _viewModel.Status = "LOCAL MODE RUNNING";
        }
        catch (Exception error) { _viewModel.Status = $"LOCAL MODE START FAILED · {error.Message}"; }
    }

    private async void RestartLocalMode_Click(object? sender, RoutedEventArgs e)
    {
        if (_session is null) return;
        try
        {
            await _session.RestartLocalModeAsync();
            _viewModel.Status = "LOCAL MODE RESTARTED";
        }
        catch (Exception error) { _viewModel.Status = $"LOCAL MODE RESTART FAILED · {error.Message}"; }
    }

    private async void AdjudicateLocalMode_Click(object? sender, RoutedEventArgs e)
    {
        if (_session is null) return;
        try
        {
            var snapshot = _session.LatestSnapshot;
            var winner = snapshot.LocalMode.Results.LastOrDefault()?.Winner
                ?? snapshot.Organisms.Where(organism => !organism.Vegetable)
                    .GroupBy(organism => organism.Species)
                    .OrderByDescending(group => group.Count())
                    .Select(group => (uint?)group.Key)
                    .FirstOrDefault();
            var reason = string.IsNullOrWhiteSpace(LocalModeReason.Text)
                ? "operator decision"
                : LocalModeReason.Text.Trim();
            await _session.AdjudicateLocalRoundAsync(winner, reason);
            _viewModel.Status = "LOCAL ROUND ADJUDICATED";
        }
        catch (Exception error) { _viewModel.Status = $"LOCAL ADJUDICATION FAILED · {error.Message}"; }
    }

    private void SynchronizeLocalModeControls()
    {
        _viewModel.LocalMode.SelectKind(LocalModeKindFromIndex(LocalModeKindSelector.SelectedIndex));
        _viewModel.LocalMode.Preset = PresetFromIndex(LocalModePreset.SelectedIndex);
        _viewModel.LocalMode.MinimumRounds = ReadUInt(ModeMinimumRounds.Value, 1);
        _viewModel.LocalMode.MaximumRounds = ReadUInt(ModeMaximumRounds.Value, 3);
        _viewModel.LocalMode.MaximumCycles = ReadULong(ModeMaximumCycles.Value, 10_000);
        _viewModel.LocalMode.MaximumPopulation = (int)Math.Clamp(ModePopulationCap.Value ?? 25_000m, 1m, 1_000_000m);
        _viewModel.LocalMode.EnergyHandicap = (int)Math.Clamp(ModeEnergyHandicap.Value ?? 0m, -100_000m, 100_000m);
        _viewModel.LocalMode.RestartOnExtinction = ModeRestartOnExtinction.IsChecked == true;
        _viewModel.LocalMode.ProhibitInformationShots = ModeProhibitInformation.IsChecked == true;
        _viewModel.LocalMode.ProhibitViruses = ModeProhibitViruses.IsChecked == true;
        _viewModel.LocalMode.ProhibitSexualReproduction = ModeProhibitSexual.IsChecked == true;
    }

    private static LocalModeKind LocalModeKindFromIndex(int index) => index switch
    {
        1 => LocalModeKind.Contest,
        2 => LocalModeKind.SimpleSurvival,
        3 => LocalModeKind.EcoSurvival,
        _ => LocalModeKind.League,
    };

    private static LeaguePreset PresetFromIndex(int index) => index switch
    {
        1 => LeaguePreset.F2,
        2 => LeaguePreset.F3,
        3 => LeaguePreset.Multibot,
        4 => LeaguePreset.ShortBot,
        5 => LeaguePreset.SingleGene,
        6 => LeaguePreset.SingleStore,
        _ => LeaguePreset.F1,
    };

    private static uint ReadUInt(decimal? value, uint fallback) =>
        (uint)Math.Clamp(value ?? fallback, 1m, uint.MaxValue);

    private static ulong ReadULong(decimal? value, ulong fallback) =>
        (ulong)Math.Clamp(value ?? fallback, 1m, ulong.MaxValue);

    private async void LiveAdvanced_Click(object? sender, RoutedEventArgs e)
    {
        if (_session is null) return;
        var dialog = new AdvancedSettingsWindow(_liveEnvironment, _session.LatestSnapshot);
        await dialog.ShowDialog(this);
        if (!dialog.Accepted) return;
        var update = dialog.Update;
        await _session.UpdateEnvironmentAsync(update);
        await _session.UpdateMutationSettingsAsync(dialog.MutationUpdate);
        _liveEnvironment = update;
        _viewModel.Status = "LIVE SETTINGS APPLIED";
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
            var species = new SpeciesImport(
                Path.GetFileNameWithoutExtension(file.Name),
                file.Content,
                false,
                0xffd07a2d,
                1,
                1_000,
                0,
                false,
                1f);
            var report = await _session.ImportSpeciesAsync(species);
            RegisterInsertionSpecies(species);
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
        catch (Exception error)
        {
            await HandleEngineErrorAsync(error, attemptRecovery: true);
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
        _runtimeClock.Stop();
        _toolWindows.CloseAll();
        _trayIcon?.Dispose();
        if (_session is not null) await _session.DisposeAsync();
    }

    private void Graphs_Click(object? sender, RoutedEventArgs e)
    {
        _toolWindows.ShowOrActivate(
            "graph-suite",
            () => new GraphSuiteWindow(_session?.LatestSnapshot ?? EngineSnapshot.Empty));
    }

    private void ContestResults_Click(object? sender, RoutedEventArgs e)
    {
        var result = ContestResultSnapshot.FromSnapshot(_session?.LatestSnapshot ?? EngineSnapshot.Empty);
        _toolWindows.ShowOrActivate("contest-results", () => new ContestResultsWindow(result));
    }
}

internal enum EnvironmentPlacement { None, Obstacle, Teleporter }
internal enum MazePattern { Horizontal, Vertical, Spiral, Checkerboard, PolarIce, TrashCompactor }
