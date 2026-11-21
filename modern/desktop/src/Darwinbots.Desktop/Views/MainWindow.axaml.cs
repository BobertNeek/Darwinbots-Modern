using Avalonia;
using Avalonia.Automation;
using Avalonia.Controls;
using Avalonia.Controls.ApplicationLifetimes;
using Avalonia.Data;
using Avalonia.Input;
using Avalonia.Input.Platform;
using Avalonia.Interactivity;
using Avalonia.Layout;
using Avalonia.Threading;
using Darwinbots.Desktop;
using Darwinbots.Desktop.Core;
using Darwinbots.Desktop.Core.LegacyMenus;
using Darwinbots.Desktop.Controls;
using Darwinbots.Desktop.Services;
using Darwinbots.Desktop.ViewModels;
using Avalonia.VisualTree;
using System.Collections.ObjectModel;
using System.ComponentModel;
using System.Diagnostics;
using System.Globalization;

namespace Darwinbots.Desktop.Views;

public sealed partial class MainWindow : Window
{
    private const string NewTeleporterToolKey = "new-teleporter";
    private const string TrayPngBase64 = "iVBORw0KGgoAAAANSUhEUgAAABAAAAAQCAYAAAAf8/9hAAAAAXNSR0IArs4c6QAAAARnQU1BAACxjwv8YQUAAAAJcEhZcwAADsMAAA7DAcdvqGQAAAAlSURBVDhPY1BQV/1PCQYb4FNrQBYeNWDUABAeNYCaBpCPVf8DAN1j1wSVGu34AAAAAElFTkSuQmCC";
    private const string NativeRobotInfoKey = "native-robot-info";

    private async void MainWindow_KeyDown(object? sender, KeyEventArgs e)
    {
        if (e.Source is not TextBox
            && HistoricMenuBar.FindCommandItem(LegacyCommandId.HelpAbout)?.InputGesture?.Matches(e) == true)
        {
            e.Handled = true;
            await ExecuteLegacyMenuCommandAsync(LegacyCommandId.HelpAbout);
            return;
        }

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

    private void MainWindow_KeyUp(object? sender, KeyEventArgs e)
    {
        if (HistoricMenuBar.FindCommandItem(LegacyCommandId.FilePauseSimulation)?.InputGesture?.Matches(e) != true)
            return;

        // A held F12 should toggle once even when the platform repeats key-down events.
        e.Handled = true;
        if (_runTimer.IsEnabled) Pause_Click(sender, e);
        else if (_session is not null) Run_Click(sender, e);
    }

    private static readonly SemaphoreSlim DeadRecordingGate = new(1, 1);
    private readonly MainWindowViewModel _viewModel = new();
    private SimulationSession? _session;
    private readonly DispatcherTimer _runTimer;
    private readonly IReadOnlyList<string> _startupArguments;
    private readonly IReadOnlyList<StartupDnaImport> _startupBots;
    private WorldSetupOptions _setup;
    private float? _initialNativeGaussianSpare;
    private byte? _initialNativeSunChange;
    private NativeProcessCachesSnapshot? _initialNativeProcessCaches;
    private readonly ObservableCollection<SpeciesImport> _insertionSpecies = [];
    private GlobalSettings _globalSettings;
    private ulong? _lastGraphRefreshTick;
    private (LocalModeKind Kind, uint RestartCount)? _automaticEvolutionGraphBoundary;
    private ulong _presentedOrdinaryRestartCount;
    private readonly IEngineClientFactory _engineFactory;
    private readonly IDesktopStorageService _storage;
    private readonly Action? _launchSafeModeBackup;
    private bool _tickInFlight;
    private bool _navigationInFlight;
    private bool _editingLiveSettings;
    private int _worldLoadGeneration;
    private bool _showingNativeRobotMutationRates;
    private readonly SemaphoreSlim _advanceGate = new(1, 1);
    private volatile bool _loadingSimulation;
    private RobotDetailsWindow? _nativeRobotInfoWindow;
    private OrganismKey? _moveOrganism;
    private WorldInsertionController? _insertion;
    private bool _turbo;
    private bool _limitMode;
    private bool _fastMode;
    private bool _flickerMode;
    private bool _syncingSpeciesSelection;
    private EnvironmentPlacement _environmentPlacement;
    private float[]? _placementOrigin;
    private WorldFeatureSelection? _selectedFeature;
    private (SimulationSession Session, WorldFeatureSelection Selection, float[] Center)? _pendingTeleporterDrag;
    private bool _sendingTeleporterDrag;
    private int _teleporterDeletionsInFlight;
    private TeleporterEditorWindow? _nativeTeleporterEditor;
    private readonly ZerobotProgressionController? _zerobotProgression;
    private bool _progressionInFlight;
    private const string EnergyOnlyFeederDna = "cond\r\nstart\r\n-2 .shoot store\r\n50 .shootval store\r\n314 rnd .aimdx store\r\nstop";
    private uint _ticksPerUpdate;
    private uint _maximumBatch = 100;
    private bool _maximumSpeed;
    private bool _following;
    private bool _controlsReady;
    private EnvironmentUpdate _liveEnvironment;
    private readonly LegacyCommandRouter _legacyCommands;
    private readonly OfflineDiagnosticRecovery _diagnostics = new();
    private bool _safemodeBackup;
    private bool _safeModeRecoveryWorld;
    private ulong? _lastAutomaticSaveTick;
    private bool _deadRecordingEnabled;
    private bool _excludeRepopulatingDead;
    private bool _startupComplete;
    private bool _ownsRestartContinuation;
    private bool _savingLastExit;
    private bool _skipLastExitCheckpoint;
    private bool _windowClosed;
    private bool _playerBotEnabled;
    private IReadOnlyList<PlayerBotBinding> _playerBotBindings = [];
    private readonly HashSet<OrganismKey> _playerBotHighlights = [];
    private HighlightedRobotDrag? _pendingHighlightedDrag;
    private bool _sendingHighlightedDrag;
    private bool _rgbMemoryMonitorEnabled;
    private RgbMemoryMonitorSettings? _rgbMemorySettings;
    private const string RgbMonitorToolKey = "rgb-memory-monitor";
    private readonly PersistentToolWindowRegistry _toolWindows;
    private readonly SeededSpawnPlacement _randomShapePlacement;
    private ShapeCreationDimensions _shapeDimensions = ShapeCreationDimensions.Default;
    private MazeDimensions _mazeDimensions = MazeDimensions.Default;
    private Db2ShapeDriftOptions? _shapeDrift;
    private int _shapeWorldGeneration;
    private ShapesWindow? _shapesWindow;
    private readonly Stopwatch _runtimeClock = new();
    private double _activeCostMultiplier;
    private TrayIcon? _trayIcon;
    private NativeMenuItem? _trayIgnoreErrorsItem;

    internal event Action<string>? RestartContinuationFailed;

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
        IDesktopStorageService storage) : this(arguments, setup, engineFactory, storage, null)
    {
    }

    public MainWindow(
        IReadOnlyList<string> arguments,
        WorldSetupOptions setup,
        IEngineClientFactory engineFactory,
        IDesktopStorageService storage,
        Action? launchSafeModeBackup)
    {
        _startupArguments = arguments.ToArray();
        _setup = setup with { NativeGaussianSpare = null, NativeSunChange = null, NativeProcessCaches = null };
        _shapeDrift = setup.ShapeDrift;
        _randomShapePlacement = new SeededSpawnPlacement(setup.Seed ^ 0x4d42UL);
        _initialNativeGaussianSpare = setup.NativeGaussianSpare;
        _initialNativeSunChange = setup.NativeSunChange;
        _initialNativeProcessCaches = setup.NativeProcessCaches;
        _globalSettings = setup.GlobalSettings;
        _safemodeBackup = setup.GlobalSettings.Main.SafeMode || setup.Recording.SafeModeBackup;
        _startupBots = new StartupDnaWorkflow(
            setup.WorldWidth,
            setup.WorldHeight,
            setup.GlobalSettings.Main.MainDirectory).Resolve(_startupArguments);
        _engineFactory = engineFactory ?? throw new ArgumentNullException(nameof(engineFactory));
        _storage = storage ?? throw new ArgumentNullException(nameof(storage));
        _launchSafeModeBackup = launchSafeModeBackup ?? (storage is AvaloniaDesktopStorageService files
            ? () => SafeModeBackupProcess.Start(files.AutosaveDirectory())
            : null);
        _zerobotProgression = null;
        InitializeComponent();
        Viewport.FixedRatio = _globalSettings.Main.FixedRatio;
        Viewport.OldColors = _globalSettings.Main.OldColors;
        if (_globalSettings.Evolution.Enabled) {
            var kind = _globalSettings.Evolution.Mode switch {
                EvolutionMode.Zerobot => LocalModeKind.Zerobot,
                EvolutionMode.Eco => LocalModeKind.EcoSurvival,
                _ => LocalModeKind.SimpleSurvival,
            };
            LocalModeKindSelector.SelectedIndex = (int)kind;
            _viewModel.LocalMode.SelectKind(kind);
            UpdateLocalModeLimitMinimums(kind);
        }
        UiPreferencesController.Register(this);
        var numericCultureAppliedAfterExpansion = false;
        EnergyExpander.LayoutUpdated += (_, _) =>
        {
            if (!EnergyExpander.IsExpanded || numericCultureAppliedAfterExpansion) return;
            if (!EnergyExpander.GetVisualDescendants().OfType<NumericUpDown>()
                .Any(numeric => numeric.GetVisualDescendants().OfType<TextBox>().Any())) return;
            numericCultureAppliedAfterExpansion = true;
            UiPreferencesController.ApplyNumericCulture(this);
        };
        _toolWindows = new PersistentToolWindowRegistry(this);
        _activeCostMultiplier = setup.EffectiveSimulationCosts.GlobalMultiplier;
        foreach (var species in setup.Species)
            _insertionSpecies.Add(species);
        InsertionSpeciesSelector.ItemsSource = _insertionSpecies;
        ToolbarSlot17.ItemsSource = _insertionSpecies;
        if (_insertionSpecies.Count > 0)
        {
            InsertionSpeciesSelector.SelectedIndex = 0;
            ToolbarSlot17.SelectedIndex = 0;
        }
        InitializeTray();
        KeyDown += async (_, e) => await UpdatePlayerBotKeyAsync(e, pressed: true);
        KeyUp += async (_, e) => await UpdatePlayerBotKeyAsync(e, pressed: false);
        _legacyCommands = CreateLegacyCommandRouter();
        HistoricMenuBar.CommandInvoked += HistoricMenuBar_CommandInvoked;
        _ticksPerUpdate = setup.TicksPerUpdate;
        _liveEnvironment = setup.ToEnvironmentUpdate();
        SynchronizeSafemodePreference(_safemodeBackup);
        SynchronizeRecordingPreferences();
        LocalModeKindSelector.SelectedIndex = setup.StartingMode is StartingMode.Zerobots or StartingMode.ZerobotsAndVegetables ? 4 : 0;
        LocalModePreset.SelectedIndex = 0;
        ThemeSelector.SelectedIndex = (int)DesktopUiPreferences.Theme;
        CultureSelector.SelectedIndex = UiResources.Cultures
            .Select((option, index) => (option, index))
            .FirstOrDefault(value => string.Equals(value.option.Name, DesktopUiPreferences.Culture.Name, StringComparison.OrdinalIgnoreCase)).index;
        RefreshEnvironmentControls();
        WorldParametersText.Text = $"{setup.WorldWidth:N0} × {setup.WorldHeight:N0}\nCapacity {setup.PopulationCapacity:N0}\nVegetables {setup.VegetablePopulationCap:N0}";
        Viewport.ToroidalWorld = _liveEnvironment.ToroidalWorld;
        Viewport.WrapX = _liveEnvironment.WrapX;
        Viewport.WrapY = _liveEnvironment.WrapY;
        RuntimeSpeed.SelectedIndex = _ticksPerUpdate switch { <= 1 => 0, <= 5 => 1, <= 20 => 2, _ => 3 };
        _maximumSpeed = DesktopControlRules.IsMaximumSpeed(RuntimeSpeed.SelectedIndex);
        Viewport.IsPlayerBotEnabled = () => _playerBotEnabled;
        Viewport.CanAimPlayerBot = () => _playerBotEnabled && _insertion?.IsActive != true
            && _moveOrganism is null && _environmentPlacement == EnvironmentPlacement.None;
        Viewport.PlayerBotPointerChanged += position => _session?.SetPlayerBotPointer(position);
        Viewport.HighlightedRobotKeys = () => _playerBotHighlights;
        Viewport.HighlightedRobotDragMoved += async drag => await ApplyHighlightedDragAsync(drag);
        Viewport.PlayerBotHighlightRequested += async key =>
        {
            if (_playerBotHighlights.Add(key))
            {
                Viewport.SelectKeys(_playerBotHighlights, SelectedRobotKey());
                await SynchronizePlayerBotTargetsAsync();
            }
        };
        Viewport.OrganismKeySelected += async key =>
        {
            if (!_playerBotHighlights.Contains(key))
                _playerBotHighlights.Clear();
            _viewModel.Select(key);
            Viewport.SelectKeys(_playerBotHighlights, key);
            RefreshLegacyMenuStates();
            await SynchronizePlayerBotTargetsAsync();
        };
        Viewport.ContextMenu = BuildViewportContextMenu();
        Viewport.OrganismContextRequested += (key, _) =>
        {
            _viewModel.Select(key);
            Viewport.SelectKeys(_playerBotHighlights, key);
            RefreshLegacyMenuStates();
            Viewport.ContextMenu?.Open(Viewport);
        };
        Viewport.OrganismDoubleClicked += async key =>
        {
            _viewModel.Select(key);
            Viewport.SelectKeys(_playerBotHighlights, key);
            var result = await _legacyCommands.ExecuteAsync(LegacyCommandId.RobotShowInfo);
            _viewModel.Status = result.Status;
            RefreshLegacyMenuStates();
        };
        Viewport.EmptyWorldDoubleClicked += () => _toolWindows.Hide(NativeRobotInfoKey);
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
            if (_environmentPlacement == EnvironmentPlacement.None)
            {
                if (_loadingSimulation || _safeModeRecoveryWorld || RecoveryPanel.IsVisible) return;
                if (_runTimer?.IsEnabled == false) _toolWindows.Hide(NativeRobotInfoKey);
                if (_playerBotEnabled && _viewModel.SelectedKey is not null) return;

                _viewModel.ClearSelection();
                Viewport.SetRobotFocus(null);
                RefreshLegacyMenuStates();
                return;
            }
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
        Viewport.CanDragNativeTeleporter = CanDragNativeTeleporter;
        Viewport.NativeTeleporterDragMoved += (feature, center) => _ = ApplyNativeTeleporterDragAsync(feature, center);
        Viewport.NativeTeleporterDoubleClicked += async () => await EditSelectedTeleporterAsync();
        Viewport.WorldFeatureSelected += feature =>
        {
            _selectedFeature = feature;
            _viewModel.Status = $"{feature.Kind.ToString().ToUpperInvariant()} {feature.Id} SELECTED";
            RefreshFeatureEditorButtons();
            RefreshLegacyMenuStates();
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
        Viewport.OrganismKeyDragCompleted += async (key, position) =>
        {
            if (_session is null) return;
            if (_session.LatestSnapshot.Organisms.FirstOrDefault(value =>
                    value.Slot == key.Slot && value.Generation == key.Generation) is not { } organism) return;
            await _session.MoveAsync(organism.Slot, organism.Generation, position);
            _viewModel.Status = $"ORGANISM {organism.Slot}:{organism.Generation} MOVED";
        };
        DataContext = _viewModel;
        _runTimer = new DispatcherTimer { Interval = TimeSpan.FromMilliseconds(_maximumSpeed ? 1 : 16) };
        _runTimer.Tick += RunTimer_Tick;
        if (setup.LoadSavePath is null && _globalSettings.Main.VideoOff)
            SetTurboMode(true);
        Opened += Window_Opened;
        Closing += Window_Closing;
        Closed += Window_Closed;
        _controlsReady = true;
        SetIgnoreErrors(!_safemodeBackup);
        ApplyLocalizedUi();
        UpdateResponsiveLayout(Bounds.Width);
        RefreshLegacyMenuStates();
    }

    private void MainWindow_SizeChanged(object? sender, SizeChangedEventArgs e) =>
        UpdateResponsiveLayout(e.NewSize.Width);

    private void UpdateResponsiveLayout(double width)
    {
        if (width <= 0) return;
        var compact = width < 860;
        SpeciesPanel.IsVisible = !compact;
        OrganismPanel.IsVisible = !compact;
        MainSurfaceGrid.ColumnDefinitions = new ColumnDefinitions(compact ? "0,*,0" : "250,*,280");
        Grid.SetColumnSpan(PlayfieldPanel, compact ? 3 : 1);
    }

    private void ThemeSelector_Changed(object? sender, SelectionChangedEventArgs e)
    {
        if (!_controlsReady) return;
        DesktopUiPreferences.ApplyTheme((DesktopUiTheme)Math.Clamp(ThemeSelector.SelectedIndex, 0, 2));
    }

    private void CultureSelector_Changed(object? sender, SelectionChangedEventArgs e)
    {
        if (!_controlsReady) return;
        var index = Math.Clamp(CultureSelector.SelectedIndex, 0, UiResources.Cultures.Count - 1);
        DesktopUiPreferences.ApplyCulture(UiResources.Cultures[index].Name);
        ApplyLocalizedUi();
        UiPreferencesController.ApplyNumericCulture(this);
    }

    private void ApplyLocalizedUi()
    {
        var culture = DesktopUiPreferences.Culture;
        InterfaceTitle.Text = UiResources.Get("interface", culture);
        ThemeLabel.Text = UiResources.Get("theme", culture);
        CultureLabel.Text = UiResources.Get("language", culture);
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
            trayIcon.Menu = CreateTrayMenu();
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

    private NativeMenu CreateTrayMenu()
    {
        var information = new NativeMenu();
        foreach (var index in new[] { 5, 7, 1, 2, 3, 4 })
        {
            var statistic = new NativeMenuItem { IsEnabled = false };
            statistic.Bind(NativeMenuItem.HeaderProperty, new Binding(nameof(StatusPanelViewModel.Display))
            {
                Source = _viewModel.StatusPanels[index],
            });
            information.Add(statistic);
        }

        var menu = new NativeMenu();
        menu.Add(new NativeMenuItem("Show Information") { Menu = information });
        var restore = new NativeMenuItem("Restore Darwinbots");
        restore.Click += (_, _) => RestoreFromTray();
        menu.Add(restore);
        _trayIgnoreErrorsItem = new NativeMenuItem("Ignore errors")
        {
            ToggleType = MenuItemToggleType.CheckBox,
            IsChecked = _diagnostics.IgnoreErrors,
        };
        _trayIgnoreErrorsItem.Click += (_, _) => SetIgnoreErrors(!_diagnostics.IgnoreErrors);
        menu.Add(_trayIgnoreErrorsItem);
        var exit = new NativeMenuItem("Exit");
        exit.Click += (_, _) => Close();
        menu.Add(exit);
        return menu;
    }

    private static WindowIcon CreateTrayIcon()
    {
        using var stream = new MemoryStream(Convert.FromBase64String(TrayPngBase64));
        return new WindowIcon(stream);
    }

    private void SetIgnoreErrors(bool enabled)
    {
        _diagnostics.SetIgnoreErrors(enabled);
        AutomationProperties.SetName(IgnoreErrorsToggle, enabled ? "IGNORE ERRORS ON" : "IGNORE ERRORS OFF");
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

    private async void ToolbarNew_Click(object? sender, RoutedEventArgs e)
    {
        _viewModel.Status = (await OpenNewSimulationSetupAsync()).Status;
    }

    private void ToolbarSpecies_Changed(object? sender, SelectionChangedEventArgs e)
    {
        if (_syncingSpeciesSelection || ToolbarSlot17.SelectedItem is not SpeciesImport species)
            return;
        _syncingSpeciesSelection = true;
        try { InsertionSpeciesSelector.SelectedItem = species; }
        finally { _syncingSpeciesSelection = false; }
    }

    private void RefreshEnvironmentControls()
    {
        var controlsReady = _controlsReady;
        _controlsReady = false;
        try
        {
            ToroidalWorldToggle.IsChecked = _liveEnvironment.ToroidalWorld;
            LiveMetabolism.Value = _liveEnvironment.MetabolismCost;
            LivePlantEnergy.Value = _liveEnvironment.Vegetation.MaxEnergyPerTick;
            LiveFeedingToBody.Value = (decimal)_liveEnvironment.Vegetation.FeedingToBody;
            UpdateSunIndicator(_session?.LatestSnapshot.NativeSunOn ?? _liveEnvironment.Vegetation.Daytime);
            Viewport.ToroidalWorld = _liveEnvironment.ToroidalWorld;
            Viewport.WrapX = _liveEnvironment.WrapX;
            Viewport.WrapY = _liveEnvironment.WrapY;
        }
        finally { _controlsReady = controlsReady; }
    }

    private void UpdateSunIndicator(bool enabled)
    {
        var label = SunLabel(enabled);
        SunButton.Content = label;
        AutomationProperties.SetName(SunButton, label);
    }

    private static string SunLabel(bool enabled) => enabled ? "SUN ON" : "SUN OFF";

    private async void Sun_Click(object? sender, RoutedEventArgs e)
    {
        var session = _session;
        try
        {
            if (session?.LatestSnapshot.NativeSunOn is not null)
            {
                if (await session.ToggleNativeSunAsync() is { } enabled && ReferenceEquals(_session, session))
                {
                    _liveEnvironment = _liveEnvironment with { Vegetation = _liveEnvironment.Vegetation with { Daytime = enabled } };
                    _viewModel.Status = SunLabel(enabled);
                }
                return;
            }

            var vegetation = _liveEnvironment.Vegetation with { Daytime = !_liveEnvironment.Vegetation.Daytime };
            _liveEnvironment = _liveEnvironment with { Vegetation = vegetation };
            UpdateSunIndicator(vegetation.Daytime);
            if (session is not null) await session.UpdateEnvironmentAsync(_liveEnvironment);
            _viewModel.Status = SunLabel(vegetation.Daytime);
        }
        catch (Exception error)
        {
            await HandleEngineErrorAsync(error, attemptRecovery: true);
        }
    }

    private void ToolbarLimit_Click(object? sender, RoutedEventArgs e)
    {
        _limitMode = !_limitMode;
        UpdateRunTimerInterval();
        ToolbarSlot09.Content = _limitMode ? "15 CPS ON" : "15 CPS OFF";
        AutomationProperties.SetName(ToolbarSlot09, _limitMode ? "15 CPS LIMIT ON" : "15 CPS LIMIT OFF");
        _viewModel.Status = _limitMode ? "THROTTLE · 15 CPS" : "THROTTLE · NORMAL";
    }

    private void ToolbarFast_Click(object? sender, RoutedEventArgs e)
    {
        _fastMode = !_fastMode;
        RuntimeSpeed.SelectedIndex = _fastMode ? 3 : 0;
        ToolbarSlot10.Content = _fastMode ? "FAST ON" : "FAST OFF";
        AutomationProperties.SetName(ToolbarSlot10, _fastMode ? "FAST MODE ON" : "FAST MODE OFF");
        _viewModel.Status = _fastMode ? "FAST MODE ON" : "FAST MODE OFF";
    }

    private void ToolbarFlicker_Click(object? sender, RoutedEventArgs e)
    {
        _flickerMode = !_flickerMode;
        Viewport.FlickerMode = _flickerMode;
        ToolbarSlot11.Content = _flickerMode ? "FLICKER ON" : "FLICKER OFF";
        AutomationProperties.SetName(ToolbarSlot11, _flickerMode ? "FLICKER MODE ON" : "FLICKER MODE OFF");
        _viewModel.Status = _flickerMode ? "FLICKER MODE ON" : "FLICKER MODE OFF";
    }

    private void ToolbarNoSkin_Click(object? sender, RoutedEventArgs e)
    {
        Viewport.RenderSkins = !Viewport.RenderSkins;
        ToolbarSlot12.Content = Viewport.RenderSkins ? "SKINS ON" : "SKINS OFF";
        AutomationProperties.SetName(ToolbarSlot12, Viewport.RenderSkins ? "ORGANISM SKINS ON" : "ORGANISM SKINS OFF");
        _viewModel.Status = Viewport.RenderSkins ? "ORGANISM SKINS ON" : "ORGANISM SKINS OFF";
    }

    private async void ToolbarNoPoffs_Click(object? sender, RoutedEventArgs e)
    {
        if (_session is not { } session)
        {
            _viewModel.Status = "ENGINE IS NOT READY";
            return;
        }
        ToolbarSlot13.IsEnabled = false;
        try
        {
            await session.SetDeathPoffsEnabledAsync(!session.LatestSnapshot.DeathPoffsEnabled);
            SynchronizeDeathPoffPolicy(session.LatestSnapshot.DeathPoffsEnabled);
            _viewModel.Status = session.LatestSnapshot.DeathPoffsEnabled ? "DEATH EFFECTS ON" : "DEATH EFFECTS OFF";
        }
        catch (Exception error)
        {
            SynchronizeDeathPoffPolicy(session.LatestSnapshot.DeathPoffsEnabled);
            _viewModel.Status = $"DEATH EFFECTS UPDATE FAILED · {error.Message}";
        }
        finally { ToolbarSlot13.IsEnabled = true; }
    }

    private void SynchronizeDeathPoffPolicy(bool enabled)
    {
        ToolbarSlot13.Content = enabled ? "POFFS ON" : "POFFS OFF";
        AutomationProperties.SetName(ToolbarSlot13, enabled ? "DEATH EFFECTS ON" : "DEATH EFFECTS OFF");
    }

    private void ToolbarNoGraphics_Click(object? sender, RoutedEventArgs e) => Turbo_Click(sender, e);

    private async void ToolbarFindBest_Click(object? sender, RoutedEventArgs e)
    {
        _viewModel.Status = (await FindBestRobotAsync()).Status;
    }

    private async void ToolbarSnapshot_Click(object? sender, RoutedEventArgs e) =>
        await ExecuteLegacyMenuCommandAsync(LegacyCommandId.RecordingSnapshotLiving);

    private void ToolbarIgnoreErrors_Click(object? sender, RoutedEventArgs e)
    {
        _viewModel.Status = ToggleIgnoreErrors().Status;
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
            _ => OpenNewSimulationSetupAsync());
        handlers[LegacyCommandId.FileLoadSimulation] = Handler(
            EngineState,
            async _ => await LoadSimulationCommandAsync());
        handlers[LegacyCommandId.FileSaveSimulation] = Handler(
            EngineState,
            async _ => await SaveSimulationCommandAsync());
        handlers[LegacyCommandId.FileSaveNativeSimulation] = Handler(
            EngineState,
            async _ => await SaveNativeSimulationCommandAsync());
        handlers[LegacyCommandId.FileSaveWithoutMutations] = Handler(
            EngineState,
            async _ => await SaveSimulationWithoutMutationsCommandAsync());
        handlers[LegacyCommandId.FileQuit] = Handler(
            () => LegacyCommandState.Available,
            _ => Task.FromResult(QuitApplication()));
        handlers[LegacyCommandId.EditInsertOrganism] = Handler(
            EngineState,
            async _ => await ImportOrganismCommandAsync());
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
            EngineState,
            async _ => await TagRobotsByNameAsync());
        handlers[LegacyCommandId.EditEnableAutomaticForking] = Handler(
            () => new LegacyCommandState(_session is not null, _liveEnvironment.AutoSpeciation, _session is null ? "Start a simulation first." : null),
            async _ => await ToggleAutomaticForkingAsync());
        handlers[LegacyCommandId.EditPlayerBotSettings] = Handler(
            EngineState,
            async _ => await OpenPlayerBotSettingsAsync());
        handlers[LegacyCommandId.EditEnablePlayerBot] = Handler(
            () => F1ToolState(EngineState() with { Checked = _playerBotEnabled }),
            async _ => await TogglePlayerBotAsync());
        handlers[LegacyCommandId.EditEyeDesigner] = Handler(
            () => F1ToolState(LegacyCommandState.Available),
            async _ => await OpenEyeDesignerAsync());
        handlers[LegacyCommandId.EditRestrictionOverwrites] = Handler(
            EngineState,
            async _ => await ShowRestrictionOverridesAsync());
        handlers[LegacyCommandId.EditSpecies] = Handler(EngineState, _ => OpenSpeciesSettingsCommandAsync());
        handlers[LegacyCommandId.EditGlobalSettings] = Handler(
            () => LegacyCommandState.Available,
            async _ => await OpenGlobalSettingsCommandAsync());
        handlers[LegacyCommandId.EditGeneralSettings] = Handler(
            EngineState,
            async _ => await OpenAdvancedSettingsCommandAsync(1));
        handlers[LegacyCommandId.EditPhysicsAndCosts] = Handler(
            EngineState,
            async _ => await OpenAdvancedSettingsCommandAsync(2));
        handlers[LegacyCommandId.EditGlobalMutationRates] = Handler(
            EngineState,
            async _ => await OpenGlobalMutationRatesCommandAsync());
        handlers[LegacyCommandId.RobotMutationRates] = Handler(
            SelectedRobotState,
            async _ => await OpenRobotMutationRatesCommandAsync());
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
            () => LegacyCommandState.Available,
            async _ => await OpenRgbMonitorSettingsAsync());
        handlers[LegacyCommandId.ViewRgbMemoryMonitor] = Handler(
            () => LegacyCommandState.Available with { Checked = _rgbMemoryMonitorEnabled },
            _ => Task.FromResult(ToggleRgbMemoryMonitor()));
        handlers[LegacyCommandId.ViewImportBackground] = Handler(
            () => new LegacyCommandState(true, false),
            async _ => await ImportBackgroundAsync());
        handlers[LegacyCommandId.ViewRemoveBackground] = Handler(
            () => LegacyCommandState.Available,
            _ => Task.FromResult(RemoveBackground()));
        handlers[LegacyCommandId.ObjectsNewTeleporter] = Handler(
            EngineState,
            _ => Task.FromResult(OpenNewTeleporter()));
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
            _ => OpenShapesWindowAsync());
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
            async _ => await DeleteRandomObstaclesCommandAsync());
        handlers[LegacyCommandId.ObjectsHorizontalMaze] = MazeHandler(MazePreset.Horizontal);
        handlers[LegacyCommandId.ObjectsVerticalMaze] = MazeHandler(MazePreset.Vertical);
        handlers[LegacyCommandId.ObjectsSprial] = MazeHandler(MazePreset.Spiral);
        handlers[LegacyCommandId.ObjectsCheckerboard] = MazeHandler(MazePreset.Checkerboard);
        handlers[LegacyCommandId.ObjectsPolarIce] = MazeHandler(MazePreset.PolarIce);
        handlers[LegacyCommandId.ObjectsTrashCompactor] = MazeHandler(MazePreset.TrashCompactor);
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
            async _ => await ChangeSelectedRobotColorAsync());
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
            () => LegacyCommandState.Available,
            async _ => await MakeSelectedNewSpeciesAsync());
        handlers[LegacyCommandId.RobotSelectEntireOrganism] = Handler(
            SelectedRobotState,
            async _ => await SelectEntireOrganismAsync());
        handlers[LegacyCommandId.RobotSaveEntireOrganism] = Handler(
            SelectedRobotState,
            async _ => await SaveEntireOrganismAsync());
        handlers[LegacyCommandId.RobotFindBest] = Handler(
            EngineState,
            _ => FindBestRobotAsync());
        handlers[LegacyCommandId.RecordingSafemodeBackup] = Handler(
            () => LegacyCommandState.Available,
            _ => Task.FromResult(OpenSafeModeBackup()));
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
            async _ => await ToggleDeadRecordingAsync());
        handlers[LegacyCommandId.RecordingDeadExcludeRepopulating] = Handler(
            () => new LegacyCommandState(_session is not null, _excludeRepopulatingDead, _session is null ? "Start a simulation first." : null),
            async _ => await ToggleDeadRecordingFilterAsync());
        handlers[LegacyCommandId.EGridEnable] = ViewportToggle(
            () => Viewport.EnergyGridEnabled,
            value => Viewport.EnergyGridEnabled = value,
            "E-GRID");
        handlers[LegacyCommandId.EGridLarge] = EnergyGridSizeHandler(EnergyGridSize.Large);
        handlers[LegacyCommandId.EGridMedium] = EnergyGridSizeHandler(EnergyGridSize.Medium);
        handlers[LegacyCommandId.EGridSmall] = EnergyGridSizeHandler(EnergyGridSize.Small);
        handlers[LegacyCommandId.HelpDna] = HelpHandler(LegacyCommandId.HelpDna);
        handlers[LegacyCommandId.HelpRobotTagInformation] = Handler(EngineState, _ => CopyRobotTagReportAsync());
        handlers[LegacyCommandId.HelpSurvivalInfo] = Handler(
            () => ActiveStagedSurvival() is not null ? LegacyCommandState.Available : new(false, false, Visible: false),
            _ => ShowSurvivalInfoAsync());
        handlers[LegacyCommandId.HelpAbout] = HelpHandler(LegacyCommandId.HelpAbout);

        return new LegacyCommandRouter(handlers);
    }

    private LegacyCommandState EngineState() => _session is null
        ? new LegacyCommandState(false, false, "The simulation engine is not ready.")
        : LegacyCommandState.Available;

    private LegacyCommandState SelectedRobotState() => _viewModel.SelectedOrganism is null
        ? new LegacyCommandState(false, false, "Select a robot first.")
        : LegacyCommandState.Available;

    private LegacyCommandState F1ToolState(LegacyCommandState state) =>
        _session?.LatestSnapshot.NativeF1Enabled == true
            ? state with { Enabled = false, DisabledReason = "Unavailable in F1 competition." }
            : state;

    private LegacyCommandState FeatureState(WorldFeatureKind kind)
    {
        if (_session is null)
            return EngineState();

        var route = ObjectEditorPolicy.RouteDelete(ToObjectEditorKind(kind), SelectedObject());
        return route.Allowed
            ? LegacyCommandState.Available
            : new LegacyCommandState(false, false, route.DisabledReason);
    }

    private ObjectEditorSelection? SelectedObject() =>
        _selectedFeature is { } feature
            ? new(ToObjectEditorKind(feature.Kind), feature.Id)
            : null;

    private static ObjectEditorKind ToObjectEditorKind(WorldFeatureKind kind) => kind switch
    {
        WorldFeatureKind.Obstacle => ObjectEditorKind.Shape,
        WorldFeatureKind.Teleporter => ObjectEditorKind.Teleporter,
        _ => throw new ArgumentOutOfRangeException(nameof(kind), kind, "Unknown world feature."),
    };

    private static DelegateLegacyCommandHandler Handler(
        Func<LegacyCommandState> state,
        Func<CancellationToken, Task<LegacyCommandResult>> execute) => new(state, execute);

    private DelegateLegacyCommandHandler HelpHandler(LegacyCommandId command)
    {
        var topic = LegacyHelpResolver.Resolve(command);
        return Handler(
            () => new LegacyCommandState(true, false, Visible: topic.Visible),
            _ => ShowDocumentAsync(topic));
    }

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

    private DelegateLegacyCommandHandler MazeHandler(MazePreset pattern) => Handler(
        EngineState,
        async _ => await AddMazeCommandAsync(pattern));

    private async void HistoricMenuBar_CommandInvoked(object? sender, LegacyCommandInvokedEventArgs e) =>
        await ExecuteLegacyMenuCommandAsync(e.Command);

    private async Task ExecuteLegacyMenuCommandAsync(LegacyCommandId command)
    {
        var result = await _legacyCommands.ExecuteAsync(command);
        _viewModel.Status = result.Status;
        RefreshLegacyMenuStates();
    }

    private void RefreshLegacyMenuStates() =>
        HistoricMenuBar.RefreshStates(_legacyCommands.GetState);

    private ContextMenu BuildViewportContextMenu()
    {
        var menu = new ContextMenu();
        foreach (var (header, command) in new[]
        {
            ("SHOW ROBOT INFO", LegacyCommandId.RobotShowInfo),
            ("OPEN ROBOT CONSOLE", LegacyCommandId.RobotOpenConsole),
            ("VIEW GENE ACTIVATIONS", LegacyCommandId.RobotViewGeneActivations),
            ("SHOW PHILOGENY", LegacyCommandId.RobotShowPhilogeny),
            ("SELECT ENTIRE ORGANISM", LegacyCommandId.RobotSelectEntireOrganism),
            ("KILL ROBOT", LegacyCommandId.RobotKill),
        })
        {
            var item = new MenuItem { Header = header };
            item.Click += async (_, _) =>
            {
                var result = await _legacyCommands.ExecuteAsync(command);
                _viewModel.Status = result.Status;
                RefreshLegacyMenuStates();
            };
            menu.Items.Add(item);
        }
        var follow = new MenuItem { Header = "FOLLOW ROBOT" };
        follow.Click += Follow_Click;
        menu.Items.Add(follow);
        return menu;
    }

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

    private LegacyCommandResult BeginObjectPlacementCommand(LegacyCommandId command)
    {
        var route = ObjectEditorPolicy.Route(command, null);
        if (!route.Allowed)
            return LegacyCommandResult.Failure(route.DisabledReason ?? "OBJECT COMMAND IS UNAVAILABLE");
        return BeginEnvironmentPlacement(route.Kind == ObjectEditorKind.Shape
            ? EnvironmentPlacement.Obstacle
            : EnvironmentPlacement.Teleporter);
    }

    private LegacyCommandResult OpenNewTeleporter()
    {
        if (_session is not { } session) return LegacyCommandResult.Failure("ENGINE IS NOT READY");
        if (_navigationInFlight || _loadingSimulation)
            return LegacyCommandResult.Failure("THE SIMULATION IS CHANGING");
        _toolWindows.ShowOrActivate(NewTeleporterToolKey, () => new NewTeleporterWindow(
            (dialog, options) => CreateNativeTeleporterAsync(dialog, session, options)));
        return LegacyCommandResult.Success("NEW TELEPORTER OPEN");
    }

    private async Task CreateNativeTeleporterAsync(NewTeleporterWindow dialog, SimulationSession session, NativeTeleporterOptions options)
    {
        await _advanceGate.WaitAsync();
        try
        {
            // A load that took the gate first closes the draft before queued edits run.
            if (!dialog.IsVisible) return;
            if (_navigationInFlight || !ReferenceEquals(session, _session))
                throw new InvalidOperationException("The simulation is changing.");
            await session.AddNativeTeleporterAsync(options);
            _viewModel.Status = "TELEPORTER CREATED";
        }
        finally { _advanceGate.Release(); }
    }

    private async Task<LegacyCommandResult> KillSelectedAsync()
    {
        if (_session is null) return LegacyCommandResult.Failure("ENGINE IS NOT READY");
        if (_viewModel.SelectedOrganism is not { } organism)
            return LegacyCommandResult.Failure("SELECT A ROBOT FIRST");
        return await KillRobotsAsync(_session, [new OrganismKey(organism.Slot, organism.Generation)], false, "ROBOT KILLED");
    }

    private async Task<LegacyCommandResult> KillSelectedOrganismAsync()
    {
        if (_session is not { } session) return LegacyCommandResult.Failure("ENGINE IS NOT READY");
        if (SelectedRobotKey() is not { } seed) return LegacyCommandResult.Failure("SELECT A ROBOT FIRST");
        await _advanceGate.WaitAsync();
        try
        {
            if (_session != session || _loadingSimulation || _savingLastExit || SelectedRobotKey() != seed)
                return LegacyCommandResult.Failure("ENGINE IS NOT READY");
            var group = await session.ListLegacyOrganismGroupAsync(seed);
            if (group.Count == 0) return LegacyCommandResult.Failure("SELECT A ROBOT FIRST");
            return await KillRobotsAsync(session, [.. group], true, $"ORGANISM KILLED · {group.Count:N0} BOT(S)");
        }
        finally { _advanceGate.Release(); }
    }

    private async Task<LegacyCommandResult> KillRobotsAsync(
        SimulationSession session, OrganismKey[] keys, bool suppressDeathPoffs, string status)
    {
        IReadOnlyList<DeathRecordSnapshot> removed = [];
        if (_deadRecordingEnabled)
            removed = await session.RemoveRecordingDeathsAsync(keys, suppressDeathPoffs, _excludeRepopulatingDead,
                _globalSettings.Main.EffectiveFindBestWeight);
        else if (suppressDeathPoffs)
            await session.RemoveManyAsync(keys, suppressDeathPoffs: true);
        else
            await session.RemoveAsync(keys[0].Slot, keys[0].Generation);
        if (!session.LatestSnapshot.PlayerBot.Enabled)
        {
            _viewModel.ClearSelection();
            Viewport.SelectSlot(null);
        }
        if (removed.Count > 0)
        {
            await DeadRecordingGate.WaitAsync();
            try
            {
                var (appended, error) = await AppendDeadRecordsAsync(removed);
                if (!appended) status = $"{status} · {error ?? "DEAD RECORDING UNAVAILABLE"}";
            }
            finally { DeadRecordingGate.Release(); }
        }
        return LegacyCommandResult.Success(status);
    }

    private Task<LegacyCommandResult> OpenNewSimulationSetupAsync() => ReplaceSimulationAsync(openSetup: true);

    private async Task<LegacyCommandResult> ReplaceSimulationAsync(bool openSetup, WorldSetupOptions? editedSetup = null)
    {
        if (_savingLastExit) return LegacyCommandResult.Failure("LAST SIMULATION IS BEING SAVED");
        if (_navigationInFlight) return LegacyCommandResult.Failure("NEW SIMULATION ALREADY OPENING");
        _navigationInFlight = true;
        _runTimer.Stop();
        await _advanceGate.WaitAsync();
        try
        {
            // Native caches belong to the original process. Capture them after
            // earlier queued work, before disposing this world.
            var spare = _initialNativeGaussianSpare;
            var sunChange = _initialNativeSunChange;
            var caches = _initialNativeProcessCaches;
            var mazeDimensions = _mazeDimensions;
            if (_session is not null)
            {
                var context = await _session.ReadNativeProcessStateAsync();
                spare = context.GaussianSpare;
                sunChange = context.SunChange;
                caches = context.Caches;
            }
            if (openSetup)
            {
                var stageAppearance = _engineFactory is NativeEngineClientFactory
                    ? SetupWindow.CreateNativeAppearanceProvider(_session is { } active ? await active.ReadNativeSetupUniformStateAsync() : null)
                    : null;
                var setupWindow = new SetupWindow(_storage, spare, sunChange, caches,
                    allowLastExitResume: _startupArguments.Count == 0, loadLastExitSettings: false, stageNativeAppearance: stageAppearance,
                    startChloroplasts: _liveEnvironment.Vegetation.StartChloroplasts);
                setupWindow.WorldCreated += options =>
                {
                    var replacement = new MainWindow(_startupArguments, options, _engineFactory, _storage, _launchSafeModeBackup)
                    {
                        _mazeDimensions = mazeDimensions,
                    };
                    SetApplicationMainWindow(replacement);
                    replacement.Show();
                    setupWindow.Close();
                };
                SetApplicationMainWindow(setupWindow);
                setupWindow.Show();
            }
            else
            {
                var replacement = new MainWindow(_startupArguments,
                    (editedSetup ?? CreateNextWorldSetup()) with { NativeGaussianSpare = spare, NativeSunChange = sunChange, NativeProcessCaches = caches }, _engineFactory, _storage, _launchSafeModeBackup)
                {
                    _mazeDimensions = mazeDimensions,
                };
                SetApplicationMainWindow(replacement);
                replacement.Show();
            }
            _skipLastExitCheckpoint = true;
            Close();
            return LegacyCommandResult.Success(openSetup ? "NEW SIMULATION SETUP" : "WORLD RESET");
        }
        catch (Exception error)
        {
            return LegacyCommandResult.Failure($"NEW SIMULATION ERROR · {error.Message}");
        }
        finally
        {
            _advanceGate.Release();
            _navigationInFlight = false;
        }
    }

    private static void SetApplicationMainWindow(Window window)
    {
        if (Application.Current?.ApplicationLifetime is IClassicDesktopStyleApplicationLifetime desktop)
            desktop.MainWindow = window;
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
        return LegacyCommandResult.Success(_windowClosed ? "QUIT" : _viewModel.Status);
    }

    private async Task<LegacyCommandResult> SaveSimulationWithoutMutationsCommandAsync()
    {
        if (_session is not { } session) return LegacyCommandResult.Failure("ENGINE IS NOT READY");
        var format = await _storage.SaveSimulationWithMutationOptionsAsync(this, format =>
            format == SimulationSaveFormat.Legacy
                ? session.SaveLegacyWithoutMutationDetailsAsync()
                : session.SaveWithoutMutationsAsync());
        return format switch
        {
            SimulationSaveFormat.Legacy => LegacyCommandResult.Success("DARWINBOTS 2 SIMULATION SAVED WITHOUT MUTATION DETAILS"),
            SimulationSaveFormat.Modern => LegacyCommandResult.Success("SAVED WITHOUT MUTATION SETTINGS"),
            _ => LegacyCommandResult.Cancel(),
        };
    }

    private async Task<LegacyCommandResult> SaveSimulationCommandAsync()
    {
        if (_session is not { } session) return LegacyCommandResult.Failure("ENGINE IS NOT READY");
        var format = await _storage.SaveSimulationWithFormatAsync(this, format =>
            format == SimulationSaveFormat.Legacy ? session.SaveLegacyAsync() : session.SaveAsync());
        return format switch
        {
            SimulationSaveFormat.Legacy => LegacyCommandResult.Success("DARWINBOTS 2 SIMULATION SAVED"),
            SimulationSaveFormat.Modern => LegacyCommandResult.Success("SAVED"),
            _ => LegacyCommandResult.Cancel(),
        };
    }

    private async Task<LegacyCommandResult> SaveNativeSimulationCommandAsync()
    {
        if (_session is null) return LegacyCommandResult.Failure("ENGINE IS NOT READY");
        var bytes = await _session.SaveLegacyAsync();
        return await _storage.SaveLegacySimulationAsync(this, bytes)
            ? LegacyCommandResult.Success("DARWINBOTS 2 SIMULATION EXPORTED")
            : LegacyCommandResult.Cancel();
    }

    private async Task<LegacyCommandResult> LoadSimulationCommandAsync()
    {
        if (_session is null) return LegacyCommandResult.Failure("ENGINE IS NOT READY");
        if (await _storage.OpenSimulationWithSourceAsync(this) is not { } save)
            return LegacyCommandResult.Cancel();
        await LoadSimulationWithSourceAsync(save.Data, save.LocalPath);
        return LegacyCommandResult.Success("LOADED");
    }

    private async Task<LegacyCommandResult> ImportOrganismCommandAsync()
    {
        if (_session is not { } session) return LegacyCommandResult.Failure("ENGINE IS NOT READY");
        if (await _storage.OpenOrganismAsync(this, _globalSettings.Main.MainDirectory) is not { } organismPayload)
            return LegacyCommandResult.Cancel();
        try
        {
            var imported = await session.ImportOrganismAsync(organismPayload);
            return LegacyCommandResult.Success($"IMPORTED ORGANISM · {imported:N0} BOT(S)");
        }
        catch (Exception error)
        {
            await HandleEngineErrorAsync(error, attemptRecovery: false);
            return LegacyCommandResult.Failure(_viewModel.Status);
        }
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

    private async Task<LegacyCommandResult> TagRobotsByNameAsync()
    {
        if (_session is not { } session) return LegacyCommandResult.Failure("ENGINE IS NOT READY");
        var name = new LegacyValueWindow("Automatically Tag by Name", "Founder name", "", "Enter the exact robot founder name.");
        await name.ShowDialog(this);
        if (!name.Accepted || name.RawValue.Length == 0) return LegacyCommandResult.Cancel();

        var tag = new LegacyValueWindow("Automatically Tag by Name", "New tag", "", "Tags can contain up to 45 characters. Apply a blank tag to clear it.");
        await tag.ShowDialog(this);
        if (!tag.Accepted || _session != session) return LegacyCommandResult.Cancel();

        var count = await session.TagOrganismsByNameAsync(name.RawValue, tag.RawValue);
        return LegacyCommandResult.Success(count == 0 ? "NO ROBOTS MATCH THAT NAME" : $"TAGGED · {count:N0} ROBOT(S)");
    }

    private async Task<LegacyCommandResult> ToggleAutomaticForkingAsync()
    {
        if (_session is not { } session) return LegacyCommandResult.Failure("ENGINE IS NOT READY");
        var enabled = !_liveEnvironment.AutoSpeciation;
        var threshold = _liveEnvironment.SpeciationGeneticDistancePercent;
        if (enabled)
        {
            const int maximumPercent = 10_000;
            var culture = DesktopUiPreferences.Culture;
            var maximum = maximumPercent.ToString("N0", culture);
            var dialog = new LegacyValueWindow("Automatic Forking", "Mutation distance threshold (%)",
                threshold.ToString(culture), $"Enter the percentage of DNA mutations that triggers a new species, from 0 to {maximum}.");
            await dialog.ShowDialog(this);
            if (!dialog.Accepted) return LegacyCommandResult.Cancel();
            if (!decimal.TryParse(dialog.Value, NumberStyles.Number, culture, out var value)
                || value is < 0 or > maximumPercent)
                return LegacyCommandResult.Failure($"MUTATION DISTANCE MUST BE 0–{maximum}%");
            threshold = (float)value;
        }

        var update = _liveEnvironment with { AutoSpeciation = enabled, SpeciationGeneticDistancePercent = threshold };
        await session.UpdateEnvironmentAsync(update);
        _liveEnvironment = update;
        return LegacyCommandResult.Success($"AUTOMATIC FORKING · {(enabled ? "ON" : "OFF")}");
    }

    private async Task<LegacyCommandResult> OpenPlayerBotSettingsAsync()
    {
        if (_session is null) return LegacyCommandResult.Failure("ENGINE IS NOT READY");
        var current = _session.LatestSnapshot.PlayerBot.Bindings.Count > 0
            ? _session.LatestSnapshot.PlayerBot.Bindings
            : _playerBotBindings;
        var dialog = new PlayerBotSettingsWindow(current);
        await dialog.ShowDialog(this);
        if (!dialog.Accepted) return LegacyCommandResult.Cancel();
        _playerBotBindings = dialog.Bindings;
        await _session.ConfigurePlayerBotAsync(new PlayerBotConfig(
            _playerBotEnabled,
            SelectedRobotKey(),
            _playerBotBindings));
        return LegacyCommandResult.Success($"PLAYERBOT KEY MAP · {_playerBotBindings.Count} BINDING(S)");
    }

    private async Task<LegacyCommandResult> TogglePlayerBotAsync()
    {
        if (_session is null) return LegacyCommandResult.Failure("ENGINE IS NOT READY");
        var next = !_playerBotEnabled;
        _playerBotBindings = _session.LatestSnapshot.PlayerBot.Bindings.Count > 0
            ? _session.LatestSnapshot.PlayerBot.Bindings
            : _playerBotBindings;
        await _session.ConfigurePlayerBotAsync(new PlayerBotConfig(next, SelectedRobotKey(), _playerBotBindings));
        _playerBotEnabled = next;
        if (next)
            await SynchronizePlayerBotTargetsAsync();
        else
            _session.SetPlayerBotPointer(null);
        return LegacyCommandResult.Success($"PLAYERBOT MODE · {(next ? "ON · MAPPED KEYS WRITE AFTER SENSE CLEAR" : "OFF")}");
    }

    private async Task UpdatePlayerBotKeyAsync(Avalonia.Input.KeyEventArgs e, bool pressed)
    {
        if (!_playerBotEnabled || _session is not { } session) return;
        var keyCode = (int)e.Key;
        if (!session.LatestSnapshot.PlayerBot.Bindings.Any(binding => binding.KeyCode == keyCode)
            && !_playerBotBindings.Any(binding => binding.KeyCode == keyCode)) return;
        e.Handled = true;
        await session.SetPlayerBotKeyStateAsync(keyCode, pressed);
    }

    private OrganismKey? SelectedRobotKey() =>
        _viewModel.SelectedOrganism is { } organism
            ? new OrganismKey(organism.Slot, organism.Generation)
            : null;

    private Task SynchronizePlayerBotTargetsAsync() =>
        _playerBotEnabled && _session is { } session
            ? session.SetPlayerBotTargetsAsync(SelectedRobotKey(), _playerBotHighlights.ToArray())
            : Task.CompletedTask;

    private Task<LegacyCommandResult> OpenEyeDesignerAsync()
    {
        var window = _toolWindows.ShowOrActivate("eye-designer", () =>
        {
            var designer = new EyeDesignerWindow(_storage, WriteEyeMemoryAsync, DisableEyeBrownianMotionAsync, DisableEyeCostsAsync);
            void OnSelectionChanged(object? sender, PropertyChangedEventArgs change)
            {
                if (change.PropertyName == nameof(MainWindowViewModel.SelectedId)) UpdateEyeDesignerFocus(designer);
            }
            _viewModel.PropertyChanged += OnSelectionChanged;
            designer.Closed += (_, _) => _viewModel.PropertyChanged -= OnSelectionChanged;
            return designer;
        });
        UpdateEyeDesignerFocus(window);
        return Task.FromResult(LegacyCommandResult.Success("EYE DESIGNER OPEN"));
    }

    private void UpdateEyeDesignerFocus(EyeDesignerWindow window) =>
        window.SetFocus(_session?.LatestSnapshot ?? EngineSnapshot.Empty, SelectedRobotKey());

    private Task WriteEyeMemoryAsync(OrganismKey focus, ushort address, int value) =>
        RequireEyeDesignerSession().SetOrganismMemoryAsync(focus.Slot, focus.Generation, address, value);

    private SimulationSession RequireEyeDesignerSession()
    {
        if (_session is not { } session || _loadingSimulation || _navigationInFlight)
            throw new InvalidOperationException("The simulation is not ready.");
        if (session.LatestSnapshot.NativeF1Enabled)
            throw new InvalidOperationException("Unavailable in F1 competition.");
        return session;
    }

    private async Task DisableEyeBrownianMotionAsync()
    {
        var session = RequireEyeDesignerSession();
        await session.SetBrownianMotionAsync(0f);
        if (IsVisible && ReferenceEquals(_session, session))
            _liveEnvironment = _liveEnvironment with { BrownianMotion = 0f };
    }

    private async Task DisableEyeCostsAsync()
    {
        var session = RequireEyeDesignerSession();
        await session.SetGlobalCostMultiplierAsync(0.0);
        if (!IsVisible || !ReferenceEquals(_session, session)) return;
        _activeCostMultiplier = 0.0;
        await session.RefreshSnapshotAsync();
    }

    private async Task<LegacyCommandResult> ShowRestrictionOverridesAsync()
    {
        if (_session is null) return LegacyCommandResult.Failure("ENGINE IS NOT READY");
        var dialog = new RestrictionOptionsWindow(_session.LatestSnapshot.RestrictionOptions);
        await dialog.ShowDialog(this);
        if (!dialog.Accepted) return LegacyCommandResult.Cancel();
        await _session.UpdateRestrictionOptionsAsync(dialog.Options);
        return LegacyCommandResult.Success("RESTRICTION OPTIONS APPLIED TO LIVE ROBOTS");
    }

    private async Task<LegacyCommandResult> OpenRgbMonitorSettingsAsync()
    {
        var dialog = new RgbMemoryMonitorWindow(
            SelectedRobotKey(),
            _rgbMemorySettings ?? RgbMemoryMonitorSettings.LegacyDefaults,
            ApplyRgbMonitorSettings,
            closeOnApply: true,
            storage: _storage);
        if (_session is { } session) dialog.SetSnapshot(session.LatestSnapshot);
        await dialog.ShowDialog(this);
        return dialog.Accepted
            ? LegacyCommandResult.Success("RGB SETTINGS APPLIED")
            : LegacyCommandResult.Cancel();
    }

    private LegacyCommandResult ToggleRgbMemoryMonitor()
    {
        if (_rgbMemoryMonitorEnabled)
        {
            _rgbMemoryMonitorEnabled = false;
            Viewport.SetRgbMemoryMonitor(null);
            _toolWindows.Close(RgbMonitorToolKey);
            return LegacyCommandResult.Success("RGB MEMORY MONITOR · OFF");
        }
        if (_rgbMemorySettings is not { } settings)
            return LegacyCommandResult.Failure("CONFIGURE RGB MONITOR SETTINGS FIRST");
        _rgbMemoryMonitorEnabled = true;
        Viewport.SetRgbMemoryMonitor(settings);
        if (_session is { } session && SelectedRobotKey() is { } focus)
        {
            var window = _toolWindows.ShowOrActivate(RgbMonitorToolKey,
                () => new RgbMemoryMonitorWindow(focus, settings, ApplyRgbMonitorSettings));
            window.SetSnapshot(session.LatestSnapshot);
        }
        return LegacyCommandResult.Success("RGB MEMORY MONITOR · ON");
    }

    private void ApplyRgbMonitorSettings(RgbMemoryMonitorSettings settings)
    {
        _rgbMemorySettings = settings;
        if (_rgbMemoryMonitorEnabled) Viewport.SetRgbMemoryMonitor(settings);
        if (_session is { } session && OwnedWindows.OfType<RgbMemoryMonitorWindow>()
            .FirstOrDefault(window => !window.CloseOnApply) is { } inspector)
        {
            inspector.SetSettings(settings);
            inspector.SetSnapshot(session.LatestSnapshot);
        }
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
        return LegacyCommandResult.Success($"BACKGROUND · {files[0].Name}");
    }

    private LegacyCommandResult RemoveBackground()
    {
        Viewport.SetBackgroundImage(null);
        return LegacyCommandResult.Success("BACKGROUND REMOVED");
    }

    private async Task<LegacyCommandResult> ToggleShotDecayAsync(bool energy)
    {
        await _advanceGate.WaitAsync();
        try
        {
            if (_session is not { } session) return LegacyCommandResult.Failure("ENGINE IS NOT READY");
            var noDecay = energy ? !_liveEnvironment.Shots.EnergyShotsDoNotDecay
                : !_liveEnvironment.Shots.WasteShotsDoNotDecay;
            await session.SetShotDecayAsync(energy, noDecay);
            var shots = energy ? _liveEnvironment.Shots with { EnergyShotsDoNotDecay = noDecay }
                : _liveEnvironment.Shots with { WasteShotsDoNotDecay = noDecay };
            _liveEnvironment = _liveEnvironment with { Shots = shots };
            return LegacyCommandResult.Success($"{(energy ? "ENERGY" : "WASTE")} SHOT DECAY · {(noDecay ? "OFF" : "ON")}");
        }
        finally { _advanceGate.Release(); }
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
        var capture = await _session.CaptureLivingSnpAsync(_globalSettings.Main.EffectiveFindBestWeight);
        var suggestedFileName = $"darwinbots-living-{capture.Tick}.snp";
        var bytes = LegacyDnaText.Encode(Db2GenomeRecordFile.EncodeGenomeRecords(capture.Records, includeHeader: true));
        var saved = await _storage.SaveLivingSnapshotAsync(this, suggestedFileName, bytes, async () =>
            await ChooseLivingMutationHistoryAsync()
                ? LegacyDnaText.Encode(Db2GenomeRecordFile.EncodeMutationHistory(MutationHistories(capture.Records), includeHeader: true))
                : null);
        return saved
            ? LegacyCommandResult.Success("LIVING SNAPSHOT SAVED")
            : LegacyCommandResult.Cancel();
    }

    private Task<bool> ChooseLivingMutationHistoryAsync()
    {
        var dialog = new Window
        {
            Title = "Snapshot of the living",
            Width = 430,
            SizeToContent = SizeToContent.Height,
            CanResize = false,
            WindowStartupLocation = WindowStartupLocation.CenterOwner,
        };
        var withoutHistory = new Button { Content = "NO", IsCancel = true };
        var withHistory = new Button { Content = "YES", IsDefault = true };
        withoutHistory.Click += (_, _) => dialog.Close(false);
        withHistory.Click += (_, _) => dialog.Close(true);
        dialog.Content = new StackPanel
        {
            Margin = new Thickness(18),
            Spacing = 16,
            Children =
            {
                new TextBlock { Text = "Save mutation history alongside this snapshot?" },
                new StackPanel
                {
                    Orientation = Orientation.Horizontal,
                    HorizontalAlignment = HorizontalAlignment.Right,
                    Spacing = 8,
                    Children = { withoutHistory, withHistory },
                },
            },
        };
        return dialog.ShowDialog<bool>(this);
    }

    private static IEnumerable<(ulong Id, IReadOnlyList<string> Log)> MutationHistories(
        IEnumerable<DeathRecordSnapshot> records) => records.Select(record => (record.Id, record.MutationLog));

    private void SynchronizeRecordingPreferences()
    {
        var recording = _liveEnvironment.Recording ?? new();
        SynchronizeSafemodePreference(recording.SafeModeBackup);
        _deadRecordingEnabled = recording.DeadSnapshotEnabled;
        _excludeRepopulatingDead = recording.ExcludeRepopulatingRobots;
    }

    private async Task ApplyRecordingPreferencesAsync(Db2RecordingOptions recording)
    {
        _liveEnvironment = _liveEnvironment with { Recording = recording };
        SynchronizeRecordingPreferences();
        if (_session is not null) await _session.UpdateEnvironmentAsync(_liveEnvironment);
    }

    private async Task<LegacyCommandResult> ToggleDeadRecordingAsync()
    {
        await ApplyRecordingPreferencesAsync((_liveEnvironment.Recording ?? new()) with
        {
            DeadSnapshotEnabled = !_deadRecordingEnabled,
        });
        return LegacyCommandResult.Success($"DEAD RECORDING · {(_deadRecordingEnabled ? "ON" : "OFF")}");
    }

    private async Task<LegacyCommandResult> ToggleDeadRecordingFilterAsync()
    {
        await ApplyRecordingPreferencesAsync((_liveEnvironment.Recording ?? new()) with
        {
            ExcludeRepopulatingRobots = !_excludeRepopulatingDead,
        });
        return LegacyCommandResult.Success($"REPOPULATING DEAD · {(_excludeRepopulatingDead ? "EXCLUDED" : "INCLUDED")}");
    }

    private async Task RecordSimulationDeathsAsync(IReadOnlyList<DeathRecordSnapshot> deaths)
    {
        if (deaths.Count == 0) return;
        await DeadRecordingGate.WaitAsync();
        try
        {
            var (appended, error) = await AppendDeadRecordsAsync(deaths);
            if (!appended) _viewModel.Status = error ?? "DEAD RECORDING UNAVAILABLE";
        }
        finally { DeadRecordingGate.Release(); }
    }

    private async Task<(bool Appended, string? Error)> AppendDeadRecordsAsync(
        IReadOnlyList<DeathRecordSnapshot> records)
    {
        try
        {
            var appended = await _storage.AppendDeadSnapshotAsync(
                async stream => await stream.WriteAsync(LegacyDnaText.Encode(
                    Db2GenomeRecordFile.EncodeGenomeRecords(records, includeHeader: stream.Length == 0))),
                async stream => await stream.WriteAsync(LegacyDnaText.Encode(
                    Db2GenomeRecordFile.EncodeMutationHistory(MutationHistories(records), includeHeader: stream.Length == 0))));
            return (appended, null);
        }
        catch (Exception error)
        {
            try
            {
                await ApplyRecordingPreferencesAsync((_liveEnvironment.Recording ?? new()) with { DeadSnapshotEnabled = false });
            }
            catch { /* The recording error remains the useful diagnostic when the engine also fails. */ }
            return (false, $"DEAD RECORDING FAILED · {error.Message}");
        }
    }

    private Task<LegacyCommandResult> ShowDocumentAsync(LegacyHelpTopic topic) =>
        ShowDocumentAsync(topic.Title, topic.Section, topic.Content);

    private LocalModeSnapshot? ActiveStagedSurvival() => _session?.LatestSnapshot.LocalMode is
    {
        Kind: LocalModeKind.SimpleSurvival or LocalModeKind.EcoSurvival,
        Status: LocalModeStatus.Running or LocalModeStatus.RoundComplete or LocalModeStatus.Completed,
        Evolution: not null,
    } mode ? mode : null;

    private async Task<LegacyCommandResult> ShowSurvivalInfoAsync()
    {
        if (ActiveStagedSurvival() is not { Evolution: { } evolution } mode)
            return LegacyCommandResult.Failure("STAGED SURVIVAL IS NOT ACTIVE");

        var normalized = mode.Configuration?.StagedEvolution?.NormalizeDnaSize == true;
        string DnaSize(uint? size) => normalized ? size?.ToString() ?? "N/A" : "N/A";
        var lines = new List<string>
        {
            $"Target DNA size: {DnaSize(evolution.CurrentDnaSize)}",
            $"Next DNA size change: {DnaSize(evolution.TargetDnaSize)}",
            $"Reduction unit: {evolution.ReductionUnit}",
            $"On/Off cycles: {evolution.HideCycles}",
        };
        if (evolution.Phase == "evolving")
            lines.Add($"Current Handicap: {evolution.RawHandicap} - {evolution.InverseHandicap} = {evolution.Handicap}");

        var section = mode.Kind == LocalModeKind.EcoSurvival ? "Eco Survival" : "Simple Survival";
        await new LegacyDocumentWindow("Survival information", section, string.Join('\n', lines)).ShowDialog(this);
        return LegacyCommandResult.Success("SURVIVAL INFORMATION");
    }

    private Task<LegacyCommandResult> ShowDocumentAsync(string title, string section, string content)
    {
        var key = $"document:{title}";
        var window = _toolWindows.ShowOrActivate(key, () => new LegacyDocumentWindow(title, section, content));
        window.SetDocument(title, section, content);
        return Task.FromResult(LegacyCommandResult.Success(title.ToUpperInvariant()));
    }

    private async Task<LegacyCommandResult> CopyRobotTagReportAsync()
    {
        if (_session is not { } session) return LegacyCommandResult.Failure("ENGINE IS NOT READY");
        if (Clipboard is not { } clipboard) return LegacyCommandResult.Failure("CLIPBOARD UNAVAILABLE");

        await clipboard.SetTextAsync(await session.ReadRobotTagReportAsync());
        return LegacyCommandResult.Success("ROBOT TAG INFORMATION COPIED");
    }

    private async Task<LegacyCommandResult> OpenSpeciesSettingsCommandAsync()
    {
        if (_session is not { } session) return LegacyCommandResult.Failure("ENGINE IS NOT READY");
        var running = _runTimer.IsEnabled;
        _runTimer.Stop();
        WorldSetupOptions? startNew = null;
        await _advanceGate.WaitAsync();
        try
        {
            var configuration = await _session.ReadConfigurationAsync();
            if (configuration is null) return LegacyCommandResult.Failure("SPECIES SETTINGS UNAVAILABLE");
            var dialog = new SetupWindow(_storage,
                speciesEditor: CreateNextWorldSetup() with { Species = configuration.Species },
                nativeSpeciesImport: session.LatestSnapshot.NativeCycle.HasValue,
                stageNativeAppearance: session.StageNativeSpeciesAppearanceAsync);
            await dialog.ShowDialog(this);
            if (dialog.EditedOptions is not { } edited) return LegacyCommandResult.Cancel();
            if (dialog.StartNewRequested) startNew = edited;
            else
            {
                await _session.UpdateSpeciesSettingsAsync(edited.Species);
                _setup = _setup with { Species = edited.Species };
                await SynchronizeLoadedWorldAsync();
                return LegacyCommandResult.Success("SPECIES SETTINGS APPLIED");
            }
        }
        finally
        {
            _advanceGate.Release();
            if (running && startNew is null) _runTimer.Start();
        }
        return await ReplaceSimulationAsync(false, startNew);
    }

    private async Task<LegacyCommandResult> OpenAdvancedSettingsCommandAsync(int selectedTab)
    {
        if (_session is not { } session) return LegacyCommandResult.Failure("ENGINE IS NOT READY");
        if (_editingLiveSettings) return LegacyCommandResult.Failure("LIVE SETTINGS ALREADY OPEN");
        _editingLiveSettings = true;
        try
        {
            var configuration = await session.ReadConfigurationAsync();
            if (_windowClosed || _savingLastExit || _loadingSimulation || _navigationInFlight || _session != session)
                return LegacyCommandResult.Cancel();
            var current = session.LatestSnapshot;
            var costs = configuration?.Costs ?? _globalSettings.Costs;
            var dialog = new AdvancedSettingsWindow(_liveEnvironment, current, selectedTab, costs,
                currentCostPreset: configuration?.NativeCostPreset,
                showFieldSize: current.NativeCycle is not null, currentFieldSize: configuration?.NativeFieldSize,
                fieldSizeUnavailable: FieldSizeRestriction(current));
            await dialog.ShowDialog(this);
            if (!dialog.Accepted) return LegacyCommandResult.Cancel();
            await _advanceGate.WaitAsync();
            try
            {
                if (_windowClosed || _savingLastExit || _loadingSimulation || _navigationInFlight || _session != session)
                    return LegacyCommandResult.Cancel();
                var update = dialog.Update;
                var before = session.LatestSnapshot;
                if (update.FieldSize is not null && FieldSizeRestriction(before) is { } restriction)
                    return LegacyCommandResult.Failure(restriction);
                var costUpdate = dialog.CostUpdate;
                var presetUpdate = dialog.CostPresetUpdate;
                if (dialog.LiveSettingsEdited)
                {
                    await session.UpdateLiveSettingsAsync(update, dialog.MutationUpdate, costUpdate, presetUpdate);
                    _liveEnvironment = update with { FieldSize = null };
                    SynchronizeRecordingPreferences();
                    RefreshEnvironmentControls();
                }
                else if (costUpdate is not null || presetUpdate is not null)
                    await session.UpdateCostSettingsAsync(costUpdate, presetUpdate);
                if (costUpdate is not null)
                    _activeCostMultiplier = (await session.ReadConfigurationAsync())?.GlobalCostMultiplier ?? costUpdate.GlobalMultiplier;
                if (update.FieldSize is not null)
                {
                    var applied = await session.ReadConfigurationAsync();
                    var resized = session.LatestSnapshot;
                    RefreshWorldDimensions(resized, applied);
                    if (!before.WorldSize.SequenceEqual(resized.WorldSize))
                    {
                        Viewport.SetSnapshot(resized);
                        Viewport.FitWorld();
                    }
                }
                return LegacyCommandResult.Success("LIVE SETTINGS APPLIED");
            }
            finally { _advanceGate.Release(); }
        }
        finally { _editingLiveSettings = false; }
    }

    private string? FieldSizeRestriction(EngineSnapshot snapshot)
    {
        if (_safeModeRecoveryWorld || (RecoveryPanel.IsVisible && RecoveryRecoverButton.IsVisible))
            return "Field size is locked during Safe Mode recovery.";
        return snapshot.NativeF1Enabled ? "Field size is locked during an F1 contest." : null;
    }

    private async Task<LegacyCommandResult> OpenGlobalMutationRatesCommandAsync()
    {
        if (_session is null) return LegacyCommandResult.Failure("ENGINE IS NOT READY");
        var session = _session;
        var configuration = await session.ReadConfigurationAsync();
        var advanced = configuration?.AdvancedMutation ?? _globalSettings.AdvancedMutation;
        var dialog = new MutationRatesWindow(session.LatestSnapshot.MutationSettings, advanced);
        await dialog.ShowDialog(this);
        if (!dialog.Accepted) return LegacyCommandResult.Cancel();
        var update = dialog.ReadGlobalMutationUpdate();
        var oscillation = dialog.ReadOscillation(advanced);
        // Native readback already includes the main epigenetic factors.
        var main = configuration is null ? _globalSettings.Main : new GlobalMainSettings();
        await session.UpdateGlobalMutationSettingsAsync(update, oscillation == advanced ? null : oscillation, main);
        _globalSettings = _globalSettings with { AdvancedMutation = dialog.ReadOscillation(_globalSettings.AdvancedMutation) };
        _setup = _setup with
        {
            MutationSettings = _setup.MutationSettings with
            {
                Enabled = update.Enabled,
                GlobalMultiplier = update.GlobalMultiplier,
                Profile = update.Profile ?? _setup.MutationSettings.Profile,
            },
        };
        return LegacyCommandResult.Success("GLOBAL MUTATION RATES APPLIED");
    }

    private async Task<LegacyCommandResult> OpenRobotMutationRatesCommandAsync()
    {
        if (_session is not { } session) return LegacyCommandResult.Failure("ENGINE IS NOT READY");
        var organism = _viewModel.SelectedOrganism;
        if (organism is null) return LegacyCommandResult.Failure("SELECT A ROBOT FIRST");

        var native = session.LatestSnapshot.NativeCycle.HasValue;
        if (!native) return await ShowAndApplyAsync(organism);

        if (_showingNativeRobotMutationRates || !CanEdit()) return LegacyCommandResult.Cancel();
        _showingNativeRobotMutationRates = true;
        var wasRunning = _runTimer.IsEnabled;
        _runTimer.Stop();
        var focus = new OrganismKey(organism.Slot, organism.Generation);
        await _advanceGate.WaitAsync();
        try
        {
            if (!CanEdit() || SelectedRobotKey() != focus) return LegacyCommandResult.Cancel();
            var current = session.LatestSnapshot.Organisms.FirstOrDefault(robot =>
                robot.Slot == focus.Slot && robot.Generation == focus.Generation);
            return current is null ? LegacyCommandResult.Cancel() : await ShowAndApplyAsync(current);
        }
        finally
        {
            _advanceGate.Release();
            _showingNativeRobotMutationRates = false;
            if (wasRunning && CanEdit()) _runTimer.Start();
        }

        bool CanEdit() => _session == session && IsVisible && !_loadingSimulation && !_navigationInFlight;

        async Task<LegacyCommandResult> ShowAndApplyAsync(OrganismSnapshot target)
        {
            var dialog = new MutationRatesWindow(target.MutationProfile, applyOnClose: native);
            await dialog.ShowDialog(this);
            if (!dialog.Accepted || (native && !CanEdit())) return LegacyCommandResult.Cancel();
            await session.UpdateOrganismMutationProfileAsync(target.Slot, target.Generation, dialog.Profile);
            return LegacyCommandResult.Success($"ROBOT {target.Slot}:{target.Generation} MUTATION RATES APPLIED");
        }
    }

    private async Task<LegacyCommandResult> OpenGlobalSettingsCommandAsync()
    {
        var settings = _globalSettings;
        if (_session?.LatestSnapshot.LocalMode is { Status: LocalModeStatus.Running, Kind: LocalModeKind.Zerobot,
            Evolution: { Phase: "evolving", FitnessWeight: { } weight } })
            settings = settings with { Main = settings.Main with { FindBestWeight = weight } };
        var dialog = new GlobalSettingsWindow(settings);
        await dialog.ShowDialog(this);
        if (!dialog.Accepted) return LegacyCommandResult.Cancel();
        _globalSettings = dialog.Settings;
        SynchronizeSafemodePreference(_globalSettings.Main.SafeMode);
        Viewport.FixedRatio = _globalSettings.Main.FixedRatio;
        Viewport.OldColors = _globalSettings.Main.OldColors;
        if (_globalSettings.Evolution.Enabled) {
            var kind = _globalSettings.Evolution.Mode switch {
                EvolutionMode.Zerobot => LocalModeKind.Zerobot,
                EvolutionMode.Eco => LocalModeKind.EcoSurvival,
                _ => LocalModeKind.SimpleSurvival,
            };
            LocalModeKindSelector.SelectedIndex = (int)kind;
            _viewModel.LocalMode.SelectKind(kind);
        }
        if (_session is not null) {
            await _session.UpdateMainRulesAsync(_globalSettings.Main);
            await _session.UpdateDisqualificationRulesAsync(_globalSettings.League.DisqualificationRule);
            await _session.UpdateRecognitionFudgingAsync(_globalSettings.League.RecognitionFudging);
            await _session.UpdateAdvancedMutationSettingsAsync(_globalSettings.AdvancedMutation, _globalSettings.Main);
            await _session.UpdateCostSettingsAsync(_globalSettings.Costs);
            await _session.UpdateEnvironmentAsync(_liveEnvironment);
            _activeCostMultiplier = _globalSettings.Costs.GlobalMultiplier;
        }
        if (_trayIcon is not null)
            _trayIcon.IsVisible = !_globalSettings.Main.HideTray;
        try
        {
            await _storage.SaveGlobalSettingsAsync(dialog.Settings);
        }
        catch (Exception error) when (error is IOException or UnauthorizedAccessException)
        {
            return LegacyCommandResult.Failure($"GLOBAL SETTINGS APPLIED · PROFILE SAVE FAILED · {error.Message}", error);
        }
        _viewModel.Status = _session is null ? "GLOBAL SETTINGS SAVED FOR NEXT WORLD" : "GLOBAL RULES AND COSTS APPLIED";
        return LegacyCommandResult.Success("GLOBAL SETTINGS SAVED");
    }

    private WorldSetupOptions CreateNextWorldSetup() => _setup with
    {
        ResumeRestartMode = false,
        GlobalSettings = _globalSettings,
        Vegetation = _setup.Vegetation with { StartChloroplasts = _liveEnvironment.Vegetation.StartChloroplasts },
        Recording = _liveEnvironment.Recording ?? new(),
        ShapesVisible = _liveEnvironment.ShapesVisible ?? _setup.ShapesVisible,
        ShapeDrift = _shapeDrift,
    };

    private async Task<LegacyCommandResult> ShowSelectedRobotInfoAsync()
    {
        var organism = _viewModel.SelectedOrganism!;
        var session = _session!;
        Viewport.SelectKey(new OrganismKey(organism.Slot, organism.Generation));
        ShowRobotDetails(session, organism, RobotDetailsTab.Facts);
        return LegacyCommandResult.Success("ROBOT DETAILS OPEN");
    }

    private RobotDetailsWindow ShowRobotDetails(
        SimulationSession session,
        OrganismSnapshot organism,
        RobotDetailsTab tab)
    {
        var focus = new OrganismKey(organism.Slot, organism.Generation);
        if (session.LatestSnapshot.NativeCycle.HasValue && tab is RobotDetailsTab.Facts or RobotDetailsTab.Dna)
        {
            var info = _toolWindows.ShowOrActivate(NativeRobotInfoKey, () =>
            {
                var viewer = new RobotDetailsWindow(session, focus, tab, SelectedRobotKey,
                    parent => ReproduceFromNativeRobotInfoAsync(session, parent),
                    openNativeConsole: () => _session == session && !_loadingSimulation && session.LatestSnapshot.NativeCycle.HasValue
                        ? ShowSelectedRobotConsoleAsync()
                        : Task.FromResult(LegacyCommandResult.Failure("SIMULATION IS UNAVAILABLE")))
                { Topmost = true };
                _nativeRobotInfoWindow = viewer;
                void FollowSelection(object? sender, System.ComponentModel.PropertyChangedEventArgs change)
                {
                    if (change.PropertyName == nameof(MainWindowViewModel.SelectedId) && !_loadingSimulation)
                        viewer.SynchronizePostDnaMemoryWatch();
                }

                _viewModel.PropertyChanged += FollowSelection;
                viewer.Closed += (_, _) =>
                {
                    _viewModel.PropertyChanged -= FollowSelection;
                    if (_nativeRobotInfoWindow == viewer) _nativeRobotInfoWindow = null;
                };
                return viewer;
            });
            info.SynchronizePostDnaMemoryWatch(reset: true);
            if (tab == RobotDetailsTab.Dna) info.SelectTab(tab);
            info.SetFollowedSnapshot(session.LatestSnapshot, tab == RobotDetailsTab.Dna ? focus : null);
            return info;
        }

        var key = $"robot-details:{organism.Slot}:{organism.Generation}";
        var window = _toolWindows.ShowOrActivate(
            key,
            () => new RobotDetailsWindow(session, focus, tab));
        window.SelectTab(tab);
        window.SetSnapshot(session.LatestSnapshot);
        return window;
    }

    private async Task<OrganismKey?> ReproduceFromNativeRobotInfoAsync(SimulationSession session, OrganismSnapshot displayed)
    {
        var generation = _worldLoadGeneration;
        await _advanceGate.WaitAsync();
        try
        {
            if (!IsVisible || _loadingSimulation || _session != session || _worldLoadGeneration != generation)
                throw new InvalidOperationException("The simulation changed before reproduction.");

            var snapshot = session.LatestSnapshot;
            if (!snapshot.NativeCycle.HasValue || !snapshot.Organisms.Any(robot => robot.Slot == displayed.Slot
                && robot.Generation == displayed.Generation && robot.UniqueId == displayed.UniqueId
                && robot.SimulationId == displayed.SimulationId))
                throw new InvalidOperationException("The displayed robot is no longer available.");

            return await session.ReproduceNativeRobotInfoAsync(new(displayed.Slot, displayed.Generation));
        }
        finally { _advanceGate.Release(); }
    }

    private Task<LegacyCommandResult> ShowSelectedPhylogenyAsync()
    {
        if (_session is null || SelectedRobotKey() is not { } focus)
            return Task.FromResult(LegacyCommandResult.Failure("NO ROBOT SELECTED"));
        var window = _toolWindows.ShowOrActivate(
            $"phylogeny:{focus.Slot}:{focus.Generation}",
            () => new PhylogenyWindow(_session.LatestSnapshot, focus));
        window.SetSnapshot(_session.LatestSnapshot);
        return Task.FromResult(LegacyCommandResult.Success("PHILOGENY OPEN"));
    }

    private async Task<LegacyCommandResult> ChangeSelectedRobotColorAsync()
    {
        var organism = _viewModel.SelectedOrganism!;
        var session = _session!;
        var dialog = new RobotColorEditorWindow(
            organism.Phenotype.Color,
            async edit =>
            {
                if (edit.WriteToDna)
                {
                    var dna = await session.ExportDnaAsync(organism.Slot, organism.Generation);
                    await session.ReplaceDnaAsync(
                        organism.Slot,
                        organism.Generation,
                        RobotColorDna.Apply(dna, edit.Color));
                }

                await session.SetOrganismColorAsync(organism.Slot, organism.Generation, edit.Color);
            });
        await dialog.ShowDialog(this);
        if (!dialog.Accepted) return LegacyCommandResult.Cancel();
        var status = dialog.WriteToDna ? "ROBOT COLOR + DNA METADATA" : "ROBOT COLOR";
        return LegacyCommandResult.Success($"{status} · {dialog.PreviewHex}");
    }

    private async Task<LegacyCommandResult> ShowSelectedRobotConsoleAsync()
    {
        if (_session is not { } session) return LegacyCommandResult.Failure("ENGINE IS NOT READY");
        if (_viewModel.SelectedOrganism is not { } organism) return LegacyCommandResult.Failure("NO ROBOT SELECTED");
        var focus = new OrganismKey(organism.Slot, organism.Generation);
        var generation = _worldLoadGeneration;
        var native = session.LatestSnapshot.NativeCycle.HasValue;
        _toolWindows.ShowOrActivate(
            $"robot-console:{generation}:{focus.Slot}:{focus.Generation}",
            () => new RobotConsoleWindow(
                session,
                focus,
                () =>
                {
                    Run_Click(this, new RoutedEventArgs());
                    return Task.CompletedTask;
                },
                () =>
                {
                    Pause_Click(this, new RoutedEventArgs());
                    return Task.CompletedTask;
                },
                (count, cancellationToken) => AdvanceSimulationCoreAsync(count, session, generation, cancellationToken),
                showDna: native ? ShowNativeDna : null));
        return LegacyCommandResult.Success("ROBOT CONSOLE OPEN");

        void ShowNativeDna(bool selectConsoleRobot)
        {
            if (_loadingSimulation || _session != session || generation != _worldLoadGeneration
                || !session.LatestSnapshot.NativeCycle.HasValue)
                throw new InvalidOperationException("This console belongs to a previous simulation.");
            var actor = session.LatestSnapshot.Organisms.FirstOrDefault(value => value.Slot == focus.Slot
                && value.Generation == focus.Generation && value.UniqueId == organism.UniqueId
                && value.SimulationId == organism.SimulationId)
                ?? throw new InvalidOperationException("The console robot no longer exists.");
            if (selectConsoleRobot)
            {
                _viewModel.Select(focus);
                Viewport.SelectKey(focus);
                RefreshLegacyMenuStates();
            }
            ShowRobotDetails(session, actor, RobotDetailsTab.Dna);
        }
    }

    private async Task<LegacyCommandResult> ShowSelectedGeneActivationsAsync()
    {
        var organism = _viewModel.SelectedOrganism!;
        ShowRobotDetails(_session!, organism, RobotDetailsTab.GeneActivations);
        return LegacyCommandResult.Success("GENE ACTIVATIONS OPEN");
    }

    private async Task<LegacyCommandResult> MakeSelectedNewSpeciesAsync()
    {
        if (_session is not { } session || _viewModel.SelectedOrganism is not { } organism)
            return LegacyCommandResult.Cancel();
        if (organism.IsCorpse) return LegacyCommandResult.Failure("A CORPSE CANNOT START A NEW SPECIES");
        var dialog = new LegacyConfirmationWindow("Make New Species", "Start new species using this robot?");
        if (!await dialog.ShowDialog<bool>(this)) return LegacyCommandResult.Cancel();
        var name = await session.ForkRobotAndDescendantsAsync(organism.Slot, organism.Generation);
        return LegacyCommandResult.Success($"NEW SPECIES · {name.ToUpperInvariant()}");
    }

    private async Task<LegacyCommandResult> SelectEntireOrganismAsync()
    {
        if (_session is not { } session || SelectedRobotKey() is not { } seed)
            return LegacyCommandResult.Failure("SELECT A ROBOT FIRST");
        var group = await session.ListLegacyOrganismGroupAsync(seed);
        if (group.Count == 0) return LegacyCommandResult.Failure("SELECT A ROBOT FIRST");
        _playerBotHighlights.UnionWith(group);
        Viewport.SelectKeys(_playerBotHighlights, SelectedRobotKey());
        await SynchronizePlayerBotTargetsAsync();
        return LegacyCommandResult.Success($"ORGANISM SELECTED · {group.Count:N0} BOT(S)");
    }

    private async Task<LegacyCommandResult> SaveEntireOrganismAsync()
    {
        if (_session is not { } session || SelectedRobotKey() is not { } seed)
            return LegacyCommandResult.Failure("NO ORGANISM SELECTED");
        await _advanceGate.WaitAsync();
        IReadOnlyList<OrganismKey> group;
        byte[] bytes;
        try
        {
            group = await session.ListLegacyOrganismGroupAsync(seed);
            if (group.Count == 0) return LegacyCommandResult.Failure("NO ORGANISM SELECTED");
            bytes = await session.ExportOrganismGroupAsync(group);
        }
        finally { _advanceGate.Release(); }
        var saved = await _storage.SaveOrganismAsync(this, $"organism-{seed.Slot}.dbo",
            async stream => await stream.WriteAsync(bytes), _globalSettings.Main.MainDirectory);
        return saved
            ? LegacyCommandResult.Success($"ORGANISM SAVED · {group.Count:N0} BOT(S)")
            : LegacyCommandResult.Cancel();
    }

    private async Task<LegacyCommandResult> SaveSelectedDnaCommandAsync()
    {
        if (_session is null || _viewModel.SelectedOrganism is not { } organism)
            return LegacyCommandResult.Failure("SELECT A ROBOT FIRST");
        var file = await _session.ExportRobotFileAsync(organism.Slot, organism.Generation);
        var suggestedName = $"Robot-{organism.Slot}-{organism.Generation}.txt";
        return await _storage.SaveDnaAsync(this, suggestedName, file.Dna, file.MutationProfile,
            _globalSettings.Main.MainDirectory)
            ? LegacyCommandResult.Success("ROBOT DNA SAVED")
            : LegacyCommandResult.Cancel();
    }

    private async Task<LegacyCommandResult> FindBestRobotAsync()
    {
        if (_session is not { } session) return LegacyCommandResult.Failure("NO LIVING ROBOTS");
        var result = await session.SelectBestRobotAsync(_globalSettings.Main.EffectiveFindBestWeight);
        if (!ReferenceEquals(session, _session)) return LegacyCommandResult.Cancel();
        if (result.Restarted) {
            _viewModel.ClearSelection();
            Viewport.SelectKey(null);
            await SynchronizePlayerBotTargetsAsync();
            return LegacyCommandResult.Success("ZEROBOT ADVANCED TO TESTING");
        }
        var native = session.LatestSnapshot.NativeCycle is not null;
        if (result.Robot is not { } key)
        {
            // The native command assigns the search result directly to robot focus.
            if (native)
            {
                _viewModel.ClearSelection();
                Viewport.SetRobotFocus(null);
            }
            return LegacyCommandResult.Failure("NO LIVING ROBOTS");
        }
        _viewModel.Select(key);
        if (native) Viewport.SetRobotFocus(key);
        else Viewport.SelectKey(key);
        await SynchronizePlayerBotTargetsAsync();
        return LegacyCommandResult.Success($"BEST FAMILY · {key.Slot}:{key.Generation}");
    }

    private async Task<LegacyCommandResult> RemoveSelectedFeatureCommandAsync()
    {
        if (_session is not { } session || _selectedFeature is not { } feature)
            return LegacyCommandResult.Failure("SELECT A WORLD OBJECT FIRST");
        var route = ObjectEditorPolicy.RouteDelete(ToObjectEditorKind(feature.Kind), SelectedObject());
        if (!route.Allowed)
            return LegacyCommandResult.Failure(route.DisabledReason ?? "SELECT A WORLD OBJECT FIRST");
        await _advanceGate.WaitAsync();
        try
        {
            if (_loadingSimulation || _navigationInFlight || !IsVisible || _session != session)
                return LegacyCommandResult.Failure("ENGINE IS NOT READY");
            var snapshot = session.LatestSnapshot;
            if (feature.Kind == WorldFeatureKind.Obstacle && snapshot.Obstacles.Any(obstacle => obstacle.Id == feature.Id))
                await session.RemoveObstacleAsync(feature.Id);
            else if (feature.Kind == WorldFeatureKind.Teleporter
                && (snapshot.FileTeleporters.Any(teleporter => teleporter.Id == feature.Id)
                    || snapshot.Teleporters.Any(teleporter => teleporter.Id == feature.Id)))
                await DeleteSelectedTeleporterAsync(session, feature.Id);
            else return LegacyCommandResult.Failure("SELECT A WORLD OBJECT FIRST");
            if (_selectedFeature == feature)
            {
                _selectedFeature = null;
                Viewport.SelectFeature(null);
                RefreshFeatureEditorButtons();
            }
            return LegacyCommandResult.Success($"{feature.Kind.ToString().ToUpperInvariant()} {feature.Id} REMOVED");
        }
        finally { _advanceGate.Release(); }
    }

    private async Task<LegacyCommandResult> DeleteAllTeleportersCommandAsync()
    {
        if (_session is not { } session) return LegacyCommandResult.Failure("ENGINE IS NOT READY");
        var snapshot = session.LatestSnapshot;
        var teleporterIds = snapshot.Teleporters.Select(teleporter => teleporter.Id).ToArray();
        var fileTeleporterIds = snapshot.FileTeleporters.Select(teleporter => teleporter.Id).ToArray();
        await DeleteTeleportersAsync(session, teleporterIds, fileTeleporterIds);
        return LegacyCommandResult.Success($"DELETED {teleporterIds.Length + fileTeleporterIds.Length} TELEPORTER(S)");
    }

    private Task DeleteSelectedTeleporterAsync(SimulationSession session, uint id) =>
        session.LatestSnapshot.FileTeleporters.Any(teleporter => teleporter.Id == id)
            ? DeleteTeleportersAsync(session, [], [id])
            : DeleteTeleportersAsync(session, [id], []);

    private async Task DeleteTeleportersAsync(SimulationSession session, IReadOnlyList<uint> teleporterIds,
        IReadOnlyList<uint> fileTeleporterIds)
    {
        _teleporterDeletionsInFlight++;
        try
        {
            InvalidateTeleporterDrag();
            if (_nativeTeleporterEditor is { } editor && teleporterIds.Contains(editor.TeleporterId))
                CloseNativeTeleporterEditor();
            foreach (var id in teleporterIds) await session.RemoveTeleporterAsync(id);
            foreach (var id in fileTeleporterIds) await session.RemoveFileTeleporterAsync(id);
        }
        finally { _teleporterDeletionsInFlight--; }
    }

    private async Task<LegacyCommandResult> DeleteAllObstaclesCommandAsync()
    {
        if (_session is not { } session) return LegacyCommandResult.Failure("ENGINE IS NOT READY");
        await _advanceGate.WaitAsync();
        try
        {
            if (_loadingSimulation || _navigationInFlight || !IsVisible || _session != session)
                return LegacyCommandResult.Failure("ENGINE IS NOT READY");
            var ids = session.LatestSnapshot.Obstacles.Select(obstacle => obstacle.Id).ToArray();
            if (ids.Length == 0) return LegacyCommandResult.Success("DELETED 0 SHAPES");
            await session.RemoveObstaclesAsync(ids);
            if (_selectedFeature is { Kind: WorldFeatureKind.Obstacle })
            {
                _selectedFeature = null;
                Viewport.SelectFeature(null);
                RefreshFeatureEditorButtons();
            }
            return LegacyCommandResult.Success($"DELETED {ids.Length} SHAPE(S)");
        }
        finally { _advanceGate.Release(); }
    }

    private async Task<LegacyCommandResult> OpenShapesWindowAsync()
    {
        await _advanceGate.WaitAsync();
        try
        {
            if (_session is { } session)
                _shapeDrift = (await session.ReadConfigurationAsync())?.ShapeDrift;
            _shapesWindow = _toolWindows.ShowOrActivate("shapes", () =>
            {
                var window = new ShapesWindow(
                    _shapeDimensions,
                    dimensions => _shapeDimensions = dimensions,
                    () => AddGeneratedShapesAsync(1, centered: true),
                    _shapeDrift,
                    SetShapeDriftAsync,
                    _mazeDimensions,
                    dimensions => _mazeDimensions = dimensions);
                window.Closed += (_, _) => _shapesWindow = null;
                return window;
            });
            return LegacyCommandResult.Success("SHAPES OPENED");
        }
        finally { _advanceGate.Release(); }
    }

    private async Task<LegacyCommandResult> SetShapeDriftAsync(bool? horizontal, bool? vertical, short? rate)
    {
        if (_session is not { } session || _loadingSimulation)
            return LegacyCommandResult.Failure("ENGINE IS NOT READY");
        var generation = _shapeWorldGeneration;
        await _advanceGate.WaitAsync();
        try
        {
            if (_loadingSimulation || _navigationInFlight || !IsVisible || _session != session
                || generation != _shapeWorldGeneration)
                return LegacyCommandResult.Failure("ENGINE IS NOT READY");
            if (await session.SetShapeDriftAsync(horizontal, vertical, rate) is not { } settings)
                return LegacyCommandResult.Failure("SHAPE MOVEMENT SETTINGS UNAVAILABLE");
            _shapeDrift = settings;
            return LegacyCommandResult.Success(string.Empty);
        }
        finally { _advanceGate.Release(); }
    }

    private Task<LegacyCommandResult> AddRandomObstaclesCommandAsync(int count) =>
        AddGeneratedShapesAsync(count, centered: false);

    private async Task<LegacyCommandResult> AddGeneratedShapesAsync(int count, bool centered)
    {
        var dimensions = _shapeDimensions;
        await _advanceGate.WaitAsync();
        try
        {
            if (_session is not { } session || _navigationInFlight || !IsVisible)
                return LegacyCommandResult.Failure("ENGINE IS NOT READY");

            var added = centered
                ? await session.AddNativeCenteredObstacleAsync(dimensions)
                : await session.AddNativeRandomObstaclesAsync(count, dimensions);
            if (added is null)
            {
                var size = session.LatestSnapshot.WorldSize;
                (float[] Minimum, float[] Maximum)[] shapes = centered
                    ? [_randomShapePlacement.NextCenteredObstacle(size[0], size[1], dimensions)]
                    : _randomShapePlacement.NextObstacles(count, size[0], size[1], dimensions);
                await AddObstaclesAsync(shapes);
                added = count;
            }

            if (!centered) return LegacyCommandResult.Success($"ADDED {added} RANDOM SHAPES");
            return added == 1
                ? LegacyCommandResult.Success("ADDED SHAPE")
                : LegacyCommandResult.Failure("SHAPE LIMIT REACHED");
        }
        finally { _advanceGate.Release(); }
    }

    private async Task<LegacyCommandResult> DeleteRandomObstaclesCommandAsync()
    {
        if (_session is not { } session) return LegacyCommandResult.Failure("ENGINE IS NOT READY");
        await _advanceGate.WaitAsync();
        try
        {
            if (_navigationInFlight || !IsVisible || !ReferenceEquals(_session, session))
                return LegacyCommandResult.Failure("ENGINE IS NOT READY");
            if (await session.RemoveNativeRandomObstaclesAsync() is { } removed)
            {
                if (_selectedFeature is { Kind: WorldFeatureKind.Obstacle } feature
                    && !session.LatestSnapshot.Obstacles.Any(obstacle => obstacle.Id == feature.Id))
                {
                    _selectedFeature = null;
                    Viewport.SelectFeature(null);
                    RefreshFeatureEditorButtons();
                }
                return LegacyCommandResult.Success($"DELETED {removed} RANDOM SHAPES");
            }
        }
        finally { _advanceGate.Release(); }

        var snapshot = session.LatestSnapshot;
        var selected = snapshot.Obstacles
            .OrderBy(obstacle => HashCode.Combine(obstacle.Id, snapshot.Tick))
            .Take(10)
            .ToArray();
        if (selected.Length == 0) return LegacyCommandResult.Success("DELETED 0 RANDOM SHAPES");
        if (!await ConfirmObstacleDeletionAsync(selected.Length))
            return LegacyCommandResult.Failure("DELETION CANCELLED");
        foreach (var obstacle in selected) await session.RemoveObstacleAsync(obstacle.Id);
        return LegacyCommandResult.Success($"DELETED {selected.Length} RANDOM SHAPES");
    }

    private async Task<bool> ConfirmObstacleDeletionAsync(int count)
    {
        var label = count == 1 ? "obstacle" : "obstacles";
        var dialog = new DeleteConfirmationWindow($"Delete {count} {label}? This cannot be undone.");
        await dialog.ShowDialog(this);
        return ObstacleEditorPolicy.ConfirmDelete(dialog.Confirmed);
    }

    private async Task<LegacyCommandResult> AddMazeCommandAsync(MazePreset pattern)
    {
        if (_session is not { } session) return LegacyCommandResult.Failure("ENGINE IS NOT READY");
        if (pattern != MazePreset.TrashCompactor)
            return await AddNativeMazeAsync(session, pattern);
        var snapshot = session.LatestSnapshot;
        var size = snapshot.WorldSize;
        var shapes = MazePresetBuilder.Build(pattern, size[0], size[1]);
        var id = checked(snapshot.Obstacles.Select(obstacle => obstacle.Id).DefaultIfEmpty().Max() + 1);
        await session.AddTrashCompactorAsync(
            new ObstacleSnapshot(id, shapes[0].Minimum, shapes[0].Maximum) { Drift = shapes[0].Drift },
            new ObstacleSnapshot(checked(id + 1), shapes[1].Minimum, shapes[1].Maximum) { Drift = shapes[1].Drift });
        return LegacyCommandResult.Success($"{pattern.ToString().ToUpperInvariant()} MAZE ADDED · {shapes.Count} SHAPES");
    }

    private async Task<LegacyCommandResult> AddNativeMazeAsync(SimulationSession session, MazePreset pattern)
    {
        var generation = _shapeWorldGeneration;
        var dimensions = _mazeDimensions;
        await _advanceGate.WaitAsync();
        try
        {
            if (_loadingSimulation || _navigationInFlight || !IsVisible || _session != session
                || generation != _shapeWorldGeneration)
                return LegacyCommandResult.Failure("ENGINE IS NOT READY");

            int? added;
            if (pattern == MazePreset.PolarIce)
            {
                var polarIce = await session.AddNativePolarIceAsync();
                added = polarIce?.Added;
                if (polarIce is { } result)
                {
                    _shapeDrift = result.Drift;
                    _shapesWindow?.SetDrift(result.Drift);
                }
            }
            else if (pattern is MazePreset.Horizontal or MazePreset.Vertical)
            {
                added = await session.AddNativeSimpleMazeAsync(pattern == MazePreset.Horizontal, dimensions);
            }
            else
            {
                added = await session.AddNativePatternMazeAsync(pattern, dimensions);
            }

            if (added is null)
            {
                var size = session.LatestSnapshot.WorldSize;
                var shapes = MazePresetBuilder.BuildWithDimensions(pattern, size[0], size[1], dimensions);
                await AddObstaclesAsync(shapes);
                added = shapes.Count;
            }
            return LegacyCommandResult.Success($"{pattern.ToString().ToUpperInvariant()} MAZE ADDED · {added} SHAPES");
        }
        finally { _advanceGate.Release(); }
    }

    private async Task AddObstaclesAsync(IEnumerable<MazePresetShape> shapes)
    {
        if (_session is null) return;
        var id = _session.LatestSnapshot.Obstacles.Select(obstacle => obstacle.Id).DefaultIfEmpty().Max() + 1;
        foreach (var shape in shapes)
            await _session.AddObstacleAsync(new ObstacleSnapshot(id++, shape.Minimum, shape.Maximum) { Drift = shape.Drift });
    }

    private Task AddObstaclesAsync(IEnumerable<(float[] Minimum, float[] Maximum)> shapes) =>
        AddObstaclesAsync(shapes.Select(shape => new MazePresetShape(shape.Minimum, shape.Maximum)));

    private void SynchronizeSafemodePreference(bool enabled)
    {
        _safemodeBackup = enabled;
        _globalSettings = _globalSettings with { Main = _globalSettings.Main with { SafeMode = enabled } };
        _liveEnvironment = _liveEnvironment with
        {
            Recording = (_liveEnvironment.Recording ?? new()) with { SafeModeBackup = enabled },
        };
    }

    private LegacyCommandResult OpenSafeModeBackup()
    {
        if (_launchSafeModeBackup is null)
            return LegacyCommandResult.Failure("SAFEMODE BACKUP REQUIRES AN AUTOSAVE DIRECTORY");
        _launchSafeModeBackup();
        return LegacyCommandResult.Success("SAFEMODE BACKUP OPEN");
    }

    private string? RecoveryArtifactPath()
    {
        if (_diagnostics.RecoveryArtifactPath is { } remembered && File.Exists(remembered))
            return remembered;
        return _storage.FindLatestAutosave() ?? _storage.FindLastStart();
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
            if (Path.GetExtension(path).Equals(".json", StringComparison.OrdinalIgnoreCase))
            {
                _viewModel.Status = $"LAST START SETTINGS READY · {Path.GetFileName(path)}";
                return LegacyCommandResult.Success(_viewModel.Status);
            }
            _runTimer.Stop();
            await LoadSimulationWithSourceAsync(await File.ReadAllBytesAsync(path), path, isSafeModeRecovery: true);
            _viewModel.Status = $"RECOVERED · {Path.GetFileName(path)}";
            ClearRecoveryPanel();
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
        if (_windowClosed || _savingLastExit) return;
        var tick = _session?.LatestSnapshot.Tick ?? 0;
        if (_diagnostics.IgnoreErrors)
        {
            var ignored = _diagnostics.HandleError(error, tick, null);
            _viewModel.Status = ignored.Status;
            ClearRecoveryPanel();
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
                _diagnostics.RecordCheckpoint(tick, artifact);
            }
            catch
            {
                // Keep the previous artifact available when the failing engine cannot be saved again.
            }
        }

        var decision = _diagnostics.HandleError(error, tick, artifact);
        _viewModel.Status = decision.Status;
        ShowRecoveryDecision(decision);
        RefreshLegacyMenuStates();
    }

    private void ShowRecoveryDecision(DiagnosticRecoveryDecision decision)
    {
        RecoveryPanel.IsVisible = true;
        RecoveryTitle.Text = decision.Action switch
        {
            DiagnosticRecoveryAction.Recover => "SAFE MODE RECOVERY READY",
            DiagnosticRecoveryAction.Report => "SAFE MODE DIAGNOSTICS",
            _ => "ERROR HANDLING",
        };
        RecoveryFaultWindowText.Text = decision.FaultWindow is { } faultWindow
            ? $"FAULT WINDOW {faultWindow.StartTick:N0}–{faultWindow.EndTick:N0} · {faultWindow.Width:N0} CYCLES"
            : "FAULT WINDOW UNAVAILABLE";
        RecoveryArtifactText.Text = RecoveryArtifactPath() is { } artifact
            ? $"RECOVERY ARTIFACT · {Path.GetFileName(artifact)}"
            : "NO RECOVERY ARTIFACT · LAST START SETTINGS MAY STILL BE AVAILABLE";
        RecoveryRecoverButton.IsVisible = decision.Action == DiagnosticRecoveryAction.Recover;
    }

    private void ClearRecoveryPanel() => RecoveryPanel.IsVisible = false;

    private async void RecoveryRecover_Click(object? sender, RoutedEventArgs e)
    {
        await RecoverLatestAutosaveAsync();
    }

    private void RecoveryIgnore_Click(object? sender, RoutedEventArgs e)
    {
        _diagnostics.SetIgnoreErrors(true);
        ClearRecoveryPanel();
        _viewModel.Status = "ERRORS WILL BE IGNORED";
        RefreshLegacyMenuStates();
    }

    private async void Window_Opened(object? sender, EventArgs e)
    {
        if (_session is not null || _windowClosed) return;
        try
        {
            await _storage.SaveLastStartAsync(
                System.Text.Json.JsonSerializer.SerializeToUtf8Bytes(CreateNextWorldSetup()));
            if (_windowClosed) return;
            _diagnostics.BeginSession(_storage.FindLastStart());
            _runtimeClock.Restart();
            var worldSetup = CreateNextWorldSetup() with
            {
                NativeGaussianSpare = _initialNativeGaussianSpare,
                NativeSunChange = _initialNativeSunChange,
                NativeProcessCaches = _initialNativeProcessCaches,
            };
            _activeCostMultiplier = worldSetup.EffectiveSimulationCosts.GlobalMultiplier;
            _session = new SimulationSession(_engineFactory.Create(worldSetup));
            _initialNativeGaussianSpare = null;
            _initialNativeSunChange = null;
            _initialNativeProcessCaches = null;
            _insertion = new WorldInsertionController(_session);
            _session.SnapshotPublished += snapshot =>
            {
                var loading = _loadingSimulation;
                if (!loading && snapshot.OrdinaryRestartPending) return;
                Dispatcher.UIThread.Post(async () =>
                {
                    if (_windowClosed) return;
                    if (!loading && snapshot.OrdinaryRestartCount > _presentedOrdinaryRestartCount)
                        _runtimeClock.Restart();
                    _presentedOrdinaryRestartCount = snapshot.OrdinaryRestartCount;
                    SynchronizeOrdinaryRestartControl(snapshot);
                    _playerBotEnabled = snapshot.PlayerBot.Enabled;
                    _playerBotBindings = snapshot.PlayerBot.Bindings;
                    var previousSelection = _viewModel.SelectedKey;
                    _viewModel.Update(snapshot, _runtimeClock.Elapsed, _activeCostMultiplier);
                    SynchronizeDeathPoffPolicy(snapshot.DeathPoffsEnabled);
                    if (snapshot.NativeSunOn is { } sunOn) UpdateSunIndicator(sunOn);
                    if (snapshot.LocalMode.Evolution is { Phase: "complete" }) {
                        _runTimer.Stop();
                        _viewModel.Status = "ZEROBOT EVOLUTION COMPLETE";
                    }
                    if (!loading && _ownsRestartContinuation && snapshot.LocalMode.Status == LocalModeStatus.Completed
                        && !snapshot.LocalMode.HasActiveRestartMode)
                    {
                        try
                        {
                            _storage.ClearRestartContinuation();
                            _ownsRestartContinuation = false;
                        }
                        catch (Exception error) when (error is IOException or UnauthorizedAccessException)
                        {
                            _viewModel.Status = $"RESTART CONTINUATION CLEANUP FAILED · {error.Message}";
                        }
                    }
                    Viewport.SetSnapshot(snapshot);
                    if (_nativeTeleporterEditor is { } editor
                        && (!snapshot.NativeCycle.HasValue
                            || !snapshot.Teleporters.Any(teleporter => teleporter.Id == editor.TeleporterId && !teleporter.Internet)))
                        CloseNativeTeleporterEditor();
                    Viewport.SelectKeys(_playerBotHighlights, _viewModel.SelectedKey);
                    var graphInterval = (ulong)Math.Max(1, _globalSettings.Main.GraphUpdatesPerTick);
                    var updateGraphs = _lastGraphRefreshTick is not { } lastTick
                        || snapshot.Tick < lastTick || snapshot.Tick - lastTick >= graphInterval;
                    if (updateGraphs)
                    {
                        PopulationChart.SetSnapshot(snapshot);
                        EnergyChart.SetSnapshot(snapshot);
                        _lastGraphRefreshTick = snapshot.Tick;
                    }
                    _toolWindows.PublishSnapshot(snapshot, updateGraphs);
                    if (!loading) SynchronizeAutomaticEvolutionGraphs(snapshot);
                    BuoyancyWarning.IsVisible = !_globalSettings.Main.SuppressBuoyancy
                        && (_liveEnvironment.Gravity[1] == 0 || !_liveEnvironment.Vegetation.PondMode || _liveEnvironment.WrapY)
                        && snapshot.Organisms.Any(robot => robot.Memory.Any(cell => cell.Address == 315 && cell.Value > 0));
                    RefreshLegacyMenuStates();
                    if (_zerobotProgression is not null && !_progressionInFlight
                        && _zerobotProgression.Observe(snapshot) is { } transition)
                        _ = ApplyZerobotTransitionAsync(transition);
                    if (!loading && previousSelection is not null && _viewModel.SelectedKey is null
                        && snapshot.PlayerBot.Focus is not null)
                        await SynchronizePlayerBotTargetsAsync();
                });
            };
            _viewModel.Status = "ENGINE READY — IMPORT A LEGACY BOT";
            await _session.RefreshSnapshotAsync();
            if (_windowClosed) return;
            var compatibilityWarnings = new List<string>();
            if (_setup.LoadSavePath is { } savePath)
            {
                await LoadSimulationWithSourceAsync(await File.ReadAllBytesAsync(savePath), savePath, _setup.ResumeRestartMode, _setup.IsAutosaveRecovery);
            }
            else
                foreach (var species in _setup.Species)
                {
                    if (_windowClosed) return;
                    compatibilityWarnings.AddRange((await _session.ImportSpeciesAsync(species)).CompatibilityWarnings);
                }
            foreach (var bot in _startupBots)
            {
                var file = await Db2RobotFileStorage.ReadAsync(bot.Path);
                if (_windowClosed) return;
                if (file.MutationProfile is null)
                {
                    compatibilityWarnings.AddRange((await _session.ImportDnaAsync(
                        file.Dna, bot.Position)).CompatibilityWarnings);
                    continue;
                }
                var species = new SpeciesImport(
                    Path.GetFileNameWithoutExtension(bot.Path), file.Dna, false, 0xffd07a2d,
                    1, 1_000, 0, false, 1f)
                {
                    DnaPath = bot.Path,
                    MutationProfile = file.MutationProfile,
                    Placement = new(SpeciesPlacementMode.Fixed, bot.Position[0], bot.Position[1]),
                };
                compatibilityWarnings.AddRange((await _session.ImportSpeciesAsync(species)).CompatibilityWarnings);
                RegisterInsertionSpecies(species);
            }
            if (worldSetup.EnableOrdinaryRestartOnStart && _session.LatestSnapshot.OrdinaryRestartAvailable)
                await _session.SetOrdinaryRestartAsync(true);
            // Worker completion means the snapshot was queued for presentation.
            // Do not announce readiness ahead of those normal-priority UI posts.
            await Dispatcher.UIThread.InvokeAsync(() => { }, DispatcherPriority.Background);
            if (_windowClosed) return;
            if (compatibilityWarnings.Count > 0) _viewModel.Status = $"DNA COMPATIBILITY · {string.Join(" · ", compatibilityWarnings.Distinct())}";
            else if (_startupBots.Count > 0) _viewModel.Status = $"IMPORTED {_startupBots.Count} STARTUP BOT(S)";
            else if (_setup.Species.Count > 0) _viewModel.Status = $"WORLD READY · {_setup.Species.Count} SPECIES";
            else if (_setup.LoadSavePath is not null) _viewModel.Status = _setup.IsAutosaveRecovery ? "AUTOSAVE RECOVERED" : "LOADED";
            if (_setup.StartingMode is StartingMode.Zerobots or StartingMode.ZerobotsAndVegetables)
                await ConfigureZerobotModeAsync();
            if (_windowClosed) return;
            SetIgnoreErrors(!_safemodeBackup);
            _startupComplete = true;
            if (_setup.ResumeRestartMode)
            {
                _runTimer.Start();
                _viewModel.Status = "RESTART MODE RESUMED";
            }
        }
        catch (Exception error)
        {
            if (!_windowClosed && _setup.ResumeRestartMode && RestartContinuationFailed is { } recover)
            {
                recover($"Restart mode could not be resumed. {error.Message}");
                return;
            }
            await HandleEngineErrorAsync(error, attemptRecovery: true);
        }
    }

    private async Task ConfigureZerobotModeAsync()
    {
        if (_session is null) return;
        LocalModeKindSelector.SelectedIndex = (int)LocalModeKind.Zerobot;
        _viewModel.LocalMode.SelectKind(LocalModeKind.Zerobot);
        var config = _viewModel.LocalMode.BuildConfiguration() with
        {
            Kind = LocalModeKind.Zerobot,
            MaximumPopulation = _setup.PopulationCapacity,
        };
        await _session.ConfigureLocalModeAsync(config);
        _viewModel.LocalMode.MarkConfigured();
        if (_setup.AutomaticZerobotProgression)
        {
            await _session.StartLocalModeAsync();
            _viewModel.Status = "NATIVE ZEROBOT SEARCH RUNNING";
        }
        else
        {
            _viewModel.Status = "NATIVE ZEROBOT SEARCH CONFIGURED";
        }
    }

    private void RefreshWorldDimensions(EngineSnapshot snapshot, WorldConfiguration? configuration)
    {
        var size = snapshot.WorldSize;
        _setup = _setup with
        {
            WorldWidth = size[0],
            WorldHeight = size[1],
            NativeFieldSize = configuration?.NativeFieldSize,
        };
        int capacity = configuration?.PopulationCapacity ?? _setup.PopulationCapacity;
        int vegetables = configuration?.VegetablePopulationCap ?? _setup.VegetablePopulationCap;
        WorldParametersText.Text = $"{size[0]:N0} × {size[1]:N0}\nCapacity {capacity:N0}\nVegetables {vegetables:N0}";
    }

    private async Task SynchronizeLoadedWorldAsync()
    {
        if (_session is null) return;
        await _session.ConfigureGraphSamplingAsync(_toolWindows.GraphSampling);
        if (await _session.ReadConfigurationAsync() is not { } configuration) return;
        var controlsReady = _controlsReady;
        _controlsReady = false;
        try
        {
            _liveEnvironment = configuration.Environment;
            _shapeDrift = configuration.ShapeDrift;
            _shapesWindow?.SetDrift(_shapeDrift);
            SynchronizeRecordingPreferences();
            _lastAutomaticSaveTick = _session.LatestSnapshot.Tick;
            _activeCostMultiplier = configuration.GlobalCostMultiplier;
            _globalSettings = _globalSettings with
            {
                Costs = configuration.Costs,
                AdvancedMutation = configuration.AdvancedMutation,
                League = _globalSettings.League with { DisqualificationRule = configuration.DisqualificationRule, RecognitionFudging = configuration.RecognitionFudging },
                Main = _globalSettings.Main with
                {
                    BodyCutoff = configuration.BodyCutoff,
                    GreedyEnforcement = configuration.GreedyReproduction,
                    // The engine persists the effective epigenetic policy.
                    // Keep its factor in AdvancedMutation so saving cannot square it.
                    EpigeneticReset = false,
                    EpigeneticAmplification = 1.0,
                    TemporaryGeneData = false,
                },
            };
            RefreshEnvironmentControls();
            RefreshWorldDimensions(_session.LatestSnapshot, configuration);
            _insertionSpecies.Clear();
            foreach (var species in configuration.Species) _insertionSpecies.Add(species);
            InsertionSpeciesSelector.SelectedIndex = _insertionSpecies.Count > 0 ? 0 : -1;
            ToolbarSlot17.SelectedIndex = InsertionSpeciesSelector.SelectedIndex;
            if (_session.LatestSnapshot.LocalMode.Configuration is { } localMode)
            {
                _viewModel.LocalMode.Observe(_session.LatestSnapshot.LocalMode,
                    _session.LatestSnapshot.NativeCycle, _session.LatestSnapshot.Species);
                LocalModeKindSelector.SelectedIndex = (int)localMode.Kind;
                UpdateLocalModeLimitMinimums(localMode.Kind);
                LocalModePreset.SelectedIndex = (int)localMode.Preset;
                LocalModeWorkflow.SelectedIndex = (int)localMode.Workflow;
                ModeEntrantOrder.Text = string.Join(", ", localMode.EntrantOrder);
                ModeInitialLadder.Text = string.Join(", ", localMode.InitialLadder);
                ModeSeedingOpponent.Value = localMode.SeedingOpponent;
                ModeFinishWithLadder.IsChecked = localMode.FinishWithLadder;
                if (localMode.StagedEvolution is { } evolution) {
                    _globalSettings = _globalSettings with { Evolution = _globalSettings.Evolution with {
                        Enabled = true, Mode = evolution.Mode, SourceBot = string.Empty,
                        StageLength = evolution.ReductionUnit, CycleLength = (int)evolution.CycleLength,
                        NormalizeDnaSize = evolution.NormalizeDnaSize, ZerobotInitialDnaLength = evolution.ZerobotInitialDnaLength,
                        AutoGraphs = evolution.AutoGraphs,
                    }};
                }
                ModeMinimumRounds.Value = localMode.MinimumRounds;
                ModeMaximumRounds.Value = localMode.MaximumRounds;
                ModeMaximumCycles.Value = localMode.MaximumCycles;
                ModePopulationCap.Value = localMode.MaximumPopulation;
                ModeEnergyHandicap.Value = localMode.EnergyHandicap;
                ModeRestartOnExtinction.IsChecked = localMode.RestartOnExtinction;
                ModeProhibitInformation.IsChecked = localMode.Restrictions.ProhibitInformationShots;
                ModeProhibitViruses.IsChecked = localMode.Restrictions.ProhibitViruses;
                ModeProhibitSexual.IsChecked = localMode.Restrictions.ProhibitSexualReproduction;
            }
        }
        finally { _controlsReady = controlsReady; }
        RefreshLegacyMenuStates();
    }

    private void RegisterInsertionSpecies(SpeciesImport species)
    {
        var existing = _insertionSpecies.FirstOrDefault(value =>
            value.Name.Equals(species.Name, StringComparison.OrdinalIgnoreCase)
            && value.Dna.Equals(species.Dna, StringComparison.Ordinal));
        if (existing is not null)
        {
            InsertionSpeciesSelector.SelectedItem = existing;
            ToolbarSlot17.SelectedItem = existing;
            return;
        }

        _insertionSpecies.Add(species);
        InsertionSpeciesSelector.SelectedItem = species;
        ToolbarSlot17.SelectedItem = species;
    }

    private async void Step_Click(object? sender, RoutedEventArgs e)
    {
        if (_session is null) return;
        try
        {
            await AdvanceSimulationAsync(1);
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
            ClearRuntimeOverlay();
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
            ShowRuntimeOverlay("INSERTION MODE", $"INSERT {species.Name.ToUpperInvariant()} · CLICK EMPTY WORLD");
            _viewModel.Status = $"INSERT {species.Name.ToUpperInvariant()} · CLICK EMPTY WORLD";
        }
        catch (Exception error)
        {
            _viewModel.Status = $"INSERTION FAILED · {error.Message}";
        }
    }

    private void ShowRuntimeOverlay(string title, string detail)
    {
        OverlayText.Text = $"{title}\n{detail}";
        RuntimeOverlay.IsVisible = true;
    }

    private void ClearRuntimeOverlay()
    {
        OverlayText.Text = string.Empty;
        RuntimeOverlay.IsVisible = false;
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
        var request = new ManualReproductionRequest(
            new OrganismKey(organism.Slot, organism.Generation),
            null,
            [organism.Position[0] + 20f, organism.Position[1] + 20f],
            organism.Energy,
            organism.Body,
            _session.LatestSnapshot.MutationSettings.Enabled
                ? ManualReproductionMutationMode.Apply
                : ManualReproductionMutationMode.Preserve,
            Focus: true);
        var child = await _session.ReproduceAsync(request);
        if (request.Focus)
        {
            Dispatcher.UIThread.Post(async () =>
            {
                _viewModel.Select(child);
                Viewport.SelectKey(child);
                await SynchronizePlayerBotTargetsAsync();
            });
        }
        _viewModel.Status = "MANUAL REPRODUCTION COMPLETE · 50% SPLIT";
    }

    private async void EditDna_Click(object? sender, RoutedEventArgs e)
    {
        if (_session is null || _viewModel.SelectedOrganism is not { } organism) return;
        await new DnaEditorWindow(_session, organism, _storage).ShowDialog(this);
    }

    private void Run_Click(object? sender, RoutedEventArgs e)
    {
        if (_savingLastExit) return;
        _runTimer.Start();
        _viewModel.Status = "RUNNING";
    }

    private void Pause_Click(object? sender, RoutedEventArgs e)
    {
        _viewModel.Status = PauseSimulation().Status;
    }

    private async void Reset_Click(object? sender, RoutedEventArgs e)
    {
        _viewModel.Status = (await ReplaceSimulationAsync(openSetup: false)).Status;
    }

    private void Speed_Changed(object? sender, SelectionChangedEventArgs e)
    {
        if (!_controlsReady) return;
        _ticksPerUpdate = DesktopControlRules.TicksForSelection(RuntimeSpeed.SelectedIndex);
        _maximumSpeed = DesktopControlRules.IsMaximumSpeed(RuntimeSpeed.SelectedIndex);
        if (_maximumSpeed) _maximumBatch = _ticksPerUpdate;
        UpdateRunTimerInterval();
        _viewModel.Status = _maximumSpeed ? "THROTTLE · MAXIMUM" : $"THROTTLE · {_ticksPerUpdate} TICKS/FRAME";
    }

    private void UpdateRunTimerInterval() =>
        _runTimer.Interval = TimeSpan.FromMilliseconds(_limitMode ? 67 : (_turbo || _maximumSpeed ? 1 : 16));

    private void Follow_Click(object? sender, RoutedEventArgs e)
    {
        if (_viewModel.SelectedOrganism is not { } organism) return;
        _following = !_following;
        Viewport.FollowKey(_following ? new OrganismKey(organism.Slot, organism.Generation) : null);
        _viewModel.Status = _following ? $"FOLLOWING {organism.Slot}:{organism.Generation}" : "FOLLOW OFF";
    }

    private void ZoomIn_Click(object? sender, RoutedEventArgs e)
    {
        Viewport.ZoomIn();
        _viewModel.Status = $"ZOOM · {Viewport.Zoom:0.00}X";
    }

    private void ZoomOut_Click(object? sender, RoutedEventArgs e)
    {
        Viewport.ZoomOut();
        _viewModel.Status = $"ZOOM · {Viewport.Zoom:0.00}X";
    }

    private void ZoomLock_Click(object? sender, RoutedEventArgs e)
    {
        Viewport.ZoomLocked = !Viewport.ZoomLocked;
        ZoomLockButton.Content = Viewport.ZoomLocked ? "LOCK ON" : "LOCK OFF";
        _viewModel.Status = Viewport.ZoomLocked ? "CAMERA LOCKED TO WORLD" : "CAMERA UNLOCKED";
    }

    private void Turbo_Click(object? sender, RoutedEventArgs e)
    {
        SetTurboMode(!_turbo);
        _viewModel.Status = _turbo ? "TURBO · RENDERING SUSPENDED" : "VIEWPORT RESTORED";
        if (_turbo) _runTimer.Start();
    }

    private void SetTurboMode(bool enabled)
    {
        _turbo = enabled;
        UpdateRunTimerInterval();
        Viewport.IsVisible = !_turbo;
        TurboButton.Content = _turbo ? "SHOW WORLD" : "TURBO";
        ToolbarSlot14.Content = _turbo ? "GRAPHICS OFF" : "GRAPHICS ON";
        AutomationProperties.SetName(ToolbarSlot14, _turbo ? "WORLD GRAPHICS OFF" : "WORLD GRAPHICS ON");
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
        if (_session is null || _selectedFeature is not { Kind: WorldFeatureKind.Obstacle } feature
            || !ObjectEditorPolicy.CanEdit(ObjectEditorKind.Shape, SelectedObject())) return;
        var obstacle = _session.LatestSnapshot.Obstacles.FirstOrDefault(value => value.Id == feature.Id);
        if (obstacle is null) return;
        var dialog = new ShapeEditorWindow(obstacle, _liveEnvironment.ShapesVisible ?? false);
        await dialog.ShowDialog(this);
        if (dialog.Result is not { } result || !IsVisible) return;
        var session = _session;
        await session.UpdateObstacleAsync(result);
        if (!IsVisible || !ReferenceEquals(_session, session)) return;
        _liveEnvironment = _liveEnvironment with { ShapesVisible = dialog.ShapesVisible };
        await session.UpdateEnvironmentAsync(_liveEnvironment);
        _viewModel.Status = $"SHAPE {result.Id} UPDATED";
    }

    private async void EditTeleporter_Click(object? sender, RoutedEventArgs e) =>
        await EditSelectedTeleporterAsync();

    private async Task EditSelectedTeleporterAsync()
    {
        if (_session is not { } session || _selectedFeature is not { Kind: WorldFeatureKind.Teleporter } feature
            || !ObjectEditorPolicy.CanEdit(ObjectEditorKind.Teleporter, SelectedObject())) return;
        var snapshot = session.LatestSnapshot;
        var localTeleporter = snapshot.Teleporters.FirstOrDefault(value => value.Id == feature.Id);
        if (localTeleporter is not null)
        {
            if (localTeleporter.Internet || string.Equals(localTeleporter.Mode.Trim(), "internet", StringComparison.OrdinalIgnoreCase)) return;
            if (snapshot.NativeCycle.HasValue)
            {
                OpenNativeTeleporterEditor(session, localTeleporter);
                return;
            }
            var dialog = new TeleporterEditorWindow(localTeleporter);
            await dialog.ShowDialog(this);
            if (dialog.Result is not { } result) return;
            await _session.UpdateTeleporterAsync(result);
            _viewModel.Status = $"TELEPORTER {result.Id} UPDATED · OFFLINE";
            return;
        }
        var teleporter = _session.LatestSnapshot.FileTeleporters.FirstOrDefault(value => value.Id == feature.Id);
        if (teleporter is null)
        {
            _viewModel.Status = "SELECT A TELEPORTER";
            return;
        }
        var fileDialog = new FileTeleporterEditorWindow(teleporter);
        await fileDialog.ShowDialog(this);
        if (fileDialog.Result is not { } fileResult) return;
        await _session.UpdateFileTeleporterAsync(fileResult);
        _viewModel.Status = $"FILE TELEPORTER {fileResult.Id} UPDATED · OFFLINE";
    }

    private void OpenNativeTeleporterEditor(SimulationSession session, TeleporterSnapshot teleporter)
    {
        if (_loadingSimulation || _navigationInFlight || _teleporterDeletionsInFlight > 0) return;
        if (_nativeTeleporterEditor is { } existing)
        {
            if (!existing.IsVisible) return;
            existing.LoadTeleporter(teleporter);
            existing.Activate();
            return;
        }

        var editor = new TeleporterEditorWindow(teleporter) { Topmost = true };
        _nativeTeleporterEditor = editor;
        editor.Activated += (_, _) =>
        {
            if (!ReferenceEquals(_nativeTeleporterEditor, editor)) return;
            if (CurrentNativeTeleporter(session) is { } focused)
                editor.LoadTeleporter(focused);
            else
                CloseNativeTeleporterEditor();
        };
        editor.Closed += async (_, _) =>
        {
            try
            {
                if (editor.Result is not null && CurrentNativeTeleporter(session)?.Id == editor.TeleporterId)
                    await ApplyNativeTeleporterEditAsync(session, editor);
            }
            finally
            {
                if (ReferenceEquals(_nativeTeleporterEditor, editor)) _nativeTeleporterEditor = null;
            }
        };
        editor.Show(this);
    }

    private async Task ApplyNativeTeleporterEditAsync(SimulationSession session, TeleporterEditorWindow editor)
    {
        await _advanceGate.WaitAsync();
        try
        {
            if (!IsVisible || _loadingSimulation || _navigationInFlight || !ReferenceEquals(_session, session)
                || !ReferenceEquals(_nativeTeleporterEditor, editor)) return;
            var current = session.LatestSnapshot.Teleporters
                .FirstOrDefault(teleporter => teleporter.Id == editor.TeleporterId && !teleporter.Internet);
            if (current is null) return;
            await session.UpdateTeleporterAsync(editor.BuildResult(current));
            if (ReferenceEquals(_nativeTeleporterEditor, editor))
                _viewModel.Status = $"TELEPORTER {current.Id} UPDATED · OFFLINE";
        }
        catch (Exception error) { await HandleEngineErrorAsync(error, attemptRecovery: false); }
        finally { _advanceGate.Release(); }
    }

    private void CloseNativeTeleporterEditor()
    {
        var editor = _nativeTeleporterEditor;
        _nativeTeleporterEditor = null;
        if (editor?.IsVisible == true) editor.Close();
    }

    private TeleporterSnapshot? CurrentNativeTeleporter(SimulationSession session)
    {
        if (_loadingSimulation || _navigationInFlight || !ReferenceEquals(_session, session)
            || _selectedFeature is not { Kind: WorldFeatureKind.Teleporter } feature)
            return null;
        var snapshot = session.LatestSnapshot;
        if (!snapshot.NativeCycle.HasValue) return null;
        return snapshot.Teleporters.FirstOrDefault(teleporter =>
            teleporter.Id == feature.Id && !teleporter.Internet
            && !string.Equals(teleporter.Mode.Trim(), "internet", StringComparison.OrdinalIgnoreCase));
    }

    private async void RemoveFeature_Click(object? sender, RoutedEventArgs e)
    {
        if (_session is null || _selectedFeature is not { } feature) return;
        var route = ObjectEditorPolicy.RouteDelete(ToObjectEditorKind(feature.Kind), SelectedObject());
        if (!route.Allowed) return;
        if (feature.Kind == WorldFeatureKind.Obstacle)
        {
            if (!await ConfirmObstacleDeletionAsync(1))
            {
                _viewModel.Status = "DELETION CANCELLED";
                return;
            }
            await _session.RemoveObstacleAsync(feature.Id);
        }
        else await DeleteSelectedTeleporterAsync(_session, feature.Id);
        _selectedFeature = null;
        Viewport.SelectFeature(null);
        RefreshFeatureEditorButtons();
        _viewModel.Status = $"{feature.Kind.ToString().ToUpperInvariant()} {feature.Id} REMOVED";
    }

    private void RefreshFeatureEditorButtons()
    {
        var selectedObstacle = _selectedFeature is { Kind: WorldFeatureKind.Obstacle } obstacle
            && ObjectEditorPolicy.CanEdit(ObjectEditorKind.Shape, SelectedObject())
            && _session?.LatestSnapshot.Obstacles.Any(value => value.Id == obstacle.Id) == true;
        var selectedTeleporter = _selectedFeature is { Kind: WorldFeatureKind.Teleporter } feature
            && ObjectEditorPolicy.CanEdit(ObjectEditorKind.Teleporter, SelectedObject())
            && (_session?.LatestSnapshot.Teleporters.Any(value => value.Id == feature.Id && !value.Internet
                    && !string.Equals(value.Mode.Trim(), "internet", StringComparison.OrdinalIgnoreCase)) == true
                || _session?.LatestSnapshot.FileTeleporters.Any(value => value.Id == feature.Id) == true);
        EditShapeButton.IsEnabled = selectedObstacle;
        EditTeleporterButton.IsEnabled = selectedTeleporter;
    }

    private async void AdvanceZerobot_Click(object? sender, RoutedEventArgs e)
    {
        if (_zerobotProgression is null)
        {
            if (_setup.StartingMode is StartingMode.Zerobots or StartingMode.ZerobotsAndVegetables
                && _session is not null && !_setup.AutomaticZerobotProgression)
            {
                try
                {
                    await _session.StartLocalModeAsync();
                    _viewModel.Status = "NATIVE ZEROBOT SEARCH RUNNING";
                }
                catch (Exception error) { _viewModel.Status = $"ZEROBOT START FAILED · {error.Message}"; }
                return;
            }
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
        var kind = LocalModeKindFromIndex(LocalModeKindSelector.SelectedIndex);
        _viewModel.LocalMode.SelectKind(kind);
        UpdateLocalModeLimitMinimums(kind);
    }

    private void UpdateLocalModeLimitMinimums(LocalModeKind kind)
    {
        var minimum = kind is LocalModeKind.League or LocalModeKind.Contest ? 0m : 1m;
        foreach (var limit in new[] { ModeMaximumRounds, ModeMaximumCycles, ModePopulationCap })
        {
            limit.Minimum = minimum;
            if (limit.Value < minimum) limit.Value = minimum;
        }
    }

    private void LocalModePreset_Changed(object? sender, SelectionChangedEventArgs e)
    {
        if (!_controlsReady) return;
        _viewModel.LocalMode.Preset = PresetFromIndex(LocalModePreset.SelectedIndex);
        _viewModel.LocalMode.MarkConfigured();
    }

    private void LocalModeWorkflow_Changed(object? sender, SelectionChangedEventArgs e)
    {
        if (!_controlsReady) return;
        _viewModel.LocalMode.Workflow = (LeagueWorkflow)Math.Clamp(LocalModeWorkflow.SelectedIndex, 0, 4);
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
            var config = await ResolveStagedEvolutionAsync(_viewModel.LocalMode.BuildConfiguration());
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
            _runTimer.Start();
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
            _runTimer.Start();
            _viewModel.Status = "LOCAL MODE RESTARTED";
        }
        catch (Exception error) { _viewModel.Status = $"LOCAL MODE RESTART FAILED · {error.Message}"; }
    }

    private async void OrdinaryRestart_Click(object? sender, RoutedEventArgs e)
    {
        if (_session is not { } session) return;
        var enabled = OrdinaryRestartToggle.IsChecked == true;
        await _advanceGate.WaitAsync();
        try
        {
            if (!session.LatestSnapshot.OrdinaryRestartAvailable) return;
            await session.SetOrdinaryRestartAsync(enabled);
            _viewModel.Status = $"ORDINARY EXTINCTION RESTART · {(enabled ? "ON" : "OFF")}";
        }
        catch (Exception error) { _viewModel.Status = $"ORDINARY RESTART SETTING FAILED · {error.Message}"; }
        finally
        {
            SynchronizeOrdinaryRestartControl(session.LatestSnapshot);
            _advanceGate.Release();
        }
    }

    private void SynchronizeOrdinaryRestartControl(EngineSnapshot snapshot)
    {
        OrdinaryRestartToggle.IsChecked = snapshot.OrdinaryRestartOnExtinction;
        OrdinaryRestartToggle.IsEnabled = snapshot.OrdinaryRestartAvailable;
        OrdinaryRestartUnavailable.IsVisible = !snapshot.OrdinaryRestartAvailable;
    }

    private async void AdjudicateLocalMode_Click(object? sender, RoutedEventArgs e)
    {
        if (_session is null) return;
        try
        {
            var snapshot = _session.LatestSnapshot;
            if (snapshot.LocalMode.Configuration?.Workflow == LeagueWorkflow.PopulationSeeding
                && (snapshot.LocalMode.Status != LocalModeStatus.Running || snapshot.LocalMode.Series?.IsClassicCompetitive != true))
            {
                _viewModel.Status = "POPULATION SEEDING IS SCORED AT CYCLE 2,000";
                return;
            }
            var pair = snapshot.LocalMode.Series?.ActiveMatch;
            var resultStart = snapshot.LocalMode.Evolution?.TestResultCursor
                ?? snapshot.LocalMode.Series?.MatchStartResult ?? 0;
            var winner = snapshot.LocalMode.Results.Skip(Math.Max(0, resultStart)).LastOrDefault()?.Winner;
            if (winner is { } id && pair is { Count: 2 } && !pair.Contains(id)) winner = null;
            winner ??= snapshot.Organisms.Where(organism => !organism.Vegetable && !organism.IsCorpse
                    && (pair is not { Count: 2 } || pair.Contains(organism.Species)))
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

    private async void ExportPopulationSeeds_Click(object? sender, RoutedEventArgs e)
    {
        if (_session is null) return;
        _runTimer.Stop();
        await _advanceGate.WaitAsync();
        try
        {
            if (_session is null) return;
            var snapshot = _session.LatestSnapshot;
            if (snapshot.LocalMode.Series is not { Workflow: LeagueWorkflow.PopulationSeeding } series
                || series.PopulationSeeds.Count == 0)
            {
                _viewModel.Status = "NO POPULATION SEEDS TO EXPORT";
                return;
            }
            if (await _storage.PickPopulationSeedDirectoryAsync(this) is not { } directory)
            {
                _viewModel.Status = "POPULATION SEED EXPORT CANCELLED";
                return;
            }
            var seeds = new List<PopulationSeedDna>(series.PopulationSeeds.Count);
            var scores = series.SeedFileOrder.Count == 0 ? series.PopulationSeeds
                : series.SeedFileOrder.Select(id => series.PopulationSeeds.Single(score => score.Species == id)).ToArray();
            if (scores.Count != series.PopulationSeeds.Count)
                throw new InvalidDataException("Population seed file order is incomplete.");
            foreach (var score in scores)
            {
                if (score.Species >= snapshot.Species.Count)
                    throw new InvalidDataException("Population seed source species is missing.");
                var dna = await _session.ExportSpeciesTemplateDnaAsync(score.Species);
                seeds.Add(new PopulationSeedDna(score, snapshot.Species[(int)score.Species].Name, dna));
            }
            var paths = await PopulationSeedingExport.WriteAsync(directory, seeds);
            _viewModel.Status = $"EXPORTED {paths.Count} POPULATION SEEDS";
        }
        catch (Exception error) { _viewModel.Status = $"POPULATION SEED EXPORT FAILED · {error.Message}"; }
        finally { _advanceGate.Release(); }
    }

    private async void ExportClassicTournament_Click(object? sender, RoutedEventArgs e)
    {
        if (_session is null) return;
        _runTimer.Stop();
        await _advanceGate.WaitAsync();
        try
        {
            if (_session is null) return;
            var snapshot = _session.LatestSnapshot;
            if (!ClassicTournamentDnaExport.IsEligible(snapshot.LocalMode))
            {
                _viewModel.Status = "CLASSIC TOURNAMENT IS NOT COMPLETE";
                return;
            }
            if (await _storage.PickPopulationSeedDirectoryAsync(this) is not { } directory)
            {
                _viewModel.Status = "CLASSIC TOURNAMENT DNA EXPORT CANCELLED";
                return;
            }
            var finalists = new List<ClassicTournamentDna>();
            foreach (var id in snapshot.LocalMode.Series!.Ladder)
            {
                if (id >= snapshot.Species.Count)
                    throw new InvalidDataException("Classic tournament source species is missing.");
                finalists.Add(new(id, snapshot.Species[(int)id].Name,
                    await _session.ExportSpeciesTemplateDnaAsync(id)));
            }
            var paths = await ClassicTournamentDnaExport.WriteAsync(directory, snapshot.LocalMode, finalists);
            _viewModel.Status = $"EXPORTED {paths.Count} CLASSIC TOURNAMENT DNA FILE(S)";
        }
        catch (Exception error) { _viewModel.Status = $"CLASSIC TOURNAMENT DNA EXPORT FAILED · {error.Message}"; }
        finally { _advanceGate.Release(); }
    }

    private async Task<LocalModeConfig> ResolveStagedEvolutionAsync(LocalModeConfig config)
    {
        var settings = _globalSettings.Evolution;
        if (!settings.Enabled || config.Kind is LocalModeKind.League or LocalModeKind.Contest)
            return config with { StagedEvolution = null };
        var mode = config.Kind switch {
            LocalModeKind.Zerobot => EvolutionMode.Zerobot,
            LocalModeKind.EcoSurvival => EvolutionMode.Eco,
            _ => EvolutionMode.SimpleSurvival,
        };
        var source = config.StagedEvolution?.SourceDna ?? string.Empty;
        var profile = config.StagedEvolution?.SourceMutationProfile;
        if (mode != EvolutionMode.Zerobot && !string.IsNullOrWhiteSpace(settings.SourceBot))
        {
            var file = await Db2RobotFileStorage.ReadAsync(Path.GetFullPath(settings.SourceBot));
            source = file.Dna;
            profile = file.MutationProfile;
        }
        var evolution = new StagedEvolutionConfiguration(mode, source, (float)settings.StageLength,
            (uint)settings.CycleLength, settings.NormalizeDnaSize, settings.ZerobotInitialDnaLength,
            settings.AutoGraphs) { FitnessWeight = (ushort)_globalSettings.Main.EffectiveFindBestWeight,
                SourceMutationProfile = mode == EvolutionMode.Zerobot ? null : profile };
        _viewModel.LocalMode.StagedEvolution = evolution;
        return config with { StagedEvolution = evolution };
    }

    private void SynchronizeLocalModeControls()
    {
        _viewModel.LocalMode.SelectKind(LocalModeKindFromIndex(LocalModeKindSelector.SelectedIndex));
        _viewModel.LocalMode.Preset = PresetFromIndex(LocalModePreset.SelectedIndex);
        _viewModel.LocalMode.Workflow = (LeagueWorkflow)Math.Clamp(LocalModeWorkflow.SelectedIndex, 0, 4);
        _viewModel.LocalMode.EntrantOrderText = ModeEntrantOrder.Text ?? string.Empty;
        _viewModel.LocalMode.InitialLadderText = ModeInitialLadder.Text ?? string.Empty;
        _viewModel.LocalMode.SeedingOpponent = _viewModel.LocalMode.Workflow == LeagueWorkflow.Seeding
            ? (uint)Math.Clamp(ModeSeedingOpponent.Value ?? 0m, 0m, uint.MaxValue) : null;
        _viewModel.LocalMode.FinishWithLadder = ModeFinishWithLadder.IsChecked == true;
        _viewModel.LocalMode.MinimumRounds = ReadUInt(ModeMinimumRounds, 5);
        _viewModel.LocalMode.MaximumRounds = ReadUInt(ModeMaximumRounds, 1);
        _viewModel.LocalMode.MaximumCycles = (ulong)Math.Clamp(ModeMaximumCycles.Value ?? 15_000m,
            ModeMaximumCycles.Minimum, ulong.MaxValue);
        _viewModel.LocalMode.MaximumPopulation = (int)Math.Clamp(ModePopulationCap.Value ?? 700m,
            ModePopulationCap.Minimum, ModePopulationCap.Maximum);
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
        4 => LocalModeKind.Zerobot,
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

    private static uint ReadUInt(NumericUpDown input, uint fallback) =>
        (uint)Math.Clamp(input.Value ?? fallback, input.Minimum, uint.MaxValue);

    private async void LiveAdvanced_Click(object? sender, RoutedEventArgs e) =>
        await ExecuteLegacyMenuCommandAsync(LegacyCommandId.EditPhysicsAndCosts);

    private async void ToroidalWorld_Changed(object? sender, RoutedEventArgs e)
    {
        if (!_controlsReady || _session is null) return;
        var enabled = ToroidalWorldToggle.IsChecked == true;
        var update = _liveEnvironment with { ToroidalWorld = enabled, WrapX = enabled, WrapY = enabled };
        await _session.UpdateEnvironmentAsync(update);
        _liveEnvironment = update;
        Viewport.WrapX = update.WrapX;
        Viewport.WrapY = update.WrapY;
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
        try
        {
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
                    1f) { MutationProfile = file.MutationProfile, DnaPath = file.LocalPath };
                var report = await _session.ImportSpeciesAsync(species);
                RegisterInsertionSpecies(species);
                _viewModel.Status = report.CompatibilityWarnings.Count == 0
                    ? $"IMPORTED {file.Name}"
                    : $"DNA COMPATIBILITY · {string.Join(" · ", report.CompatibilityWarnings)}";
            }
        }
        catch (Exception error) { _viewModel.Status = $"IMPORT FAILED · {error.Message}"; }
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
        if (await _storage.OpenSimulationWithSourceAsync(this) is not { } save) return;
        await LoadSimulationWithSourceAsync(save.Data, save.LocalPath);
        _viewModel.Status = "LOADED";
    }

    private async void RunTimer_Tick(object? sender, EventArgs e)
    {
        if (_session is null || _tickInFlight || !_runTimer.IsEnabled) return;
        _tickInFlight = true;
        var adaptive = !_limitMode && (_turbo || _maximumSpeed);
        var batch = _limitMode ? 1u : adaptive ? _maximumBatch : _ticksPerUpdate;
        var started = Stopwatch.GetTimestamp();
        try
        {
            await AdvanceSimulationAsync(batch);
            if (adaptive)
                _maximumBatch = DesktopControlRules.AdaptMaximumBatch(batch, Stopwatch.GetElapsedTime(started), _turbo);
        }
        catch (Exception error)
        {
            await HandleEngineErrorAsync(error, attemptRecovery: true);
        }
        finally
        {
            // The original limiter waits after UpdateSim and Redraw. Re-arm
            // only if the user still has the limiter and simulation running.
            if (_limitMode && _runTimer.IsEnabled)
            {
                _runTimer.Stop();
                _runTimer.Start();
            }
            _tickInFlight = false;
        }
    }

    private bool CanDragNativeTeleporter() => _session is not null && IsVisible && !_loadingSimulation
        && !_navigationInFlight && _teleporterDeletionsInFlight == 0 && !RecoveryPanel.IsVisible
        && _insertion?.IsActive != true && _moveOrganism is null && _environmentPlacement == EnvironmentPlacement.None;

    private void InvalidateTeleporterDrag()
    {
        _pendingTeleporterDrag = null;
        Viewport.CancelNativeTeleporterDrag();
    }

    private async Task ApplyNativeTeleporterDragAsync(WorldFeatureSelection selection, float[] center)
    {
        if (!CanDragNativeTeleporter() || _session is not { } session) return;
        _pendingTeleporterDrag = (session, selection, center);
        if (_sendingTeleporterDrag) return;
        _sendingTeleporterDrag = true;
        try
        {
            // Read the newest sample only after taking the gate, so a load can discard pending input.
            while (_pendingTeleporterDrag is not null)
            {
                await _advanceGate.WaitAsync();
                try
                {
                    var pending = _pendingTeleporterDrag;
                    _pendingTeleporterDrag = null;
                    if (pending is not { } move || !CanDragNativeTeleporter()
                        || _session != move.Session || !ReferenceEquals(_selectedFeature, move.Selection)) continue;
                    if (!move.Session.LatestSnapshot.Teleporters.Any(portal =>
                            portal.Id == move.Selection.Id && portal.NativeTopLeft is not null && !portal.Internet)) continue;
                    await move.Session.DragNativeTeleporterAsync(move.Selection.Id, move.Center);
                }
                finally { _advanceGate.Release(); }
            }
        }
        catch (Exception error)
        {
            await HandleEngineErrorAsync(error, attemptRecovery: false);
        }
        finally
        {
            _pendingTeleporterDrag = null;
            _sendingTeleporterDrag = false;
        }
    }

    private async Task ApplyHighlightedDragAsync(HighlightedRobotDrag drag)
    {
        if (_loadingSimulation || _session is not { } session) return;
        _pendingHighlightedDrag = drag;
        if (_sendingHighlightedDrag) return;
        _sendingHighlightedDrag = true;
        try
        {
            // Keep at most one queued worker operation and the latest waiting pointer sample.
            while (_pendingHighlightedDrag is { } latest && !_loadingSimulation && _session == session)
            {
                _pendingHighlightedDrag = null;
                await session.DragHighlightedRobotsAsync(latest);
            }
        }
        catch (Exception error)
        {
            await HandleEngineErrorAsync(error, attemptRecovery: false);
        }
        finally
        {
            _pendingHighlightedDrag = null;
            _sendingHighlightedDrag = false;
        }
    }

    private Task LoadSimulationAsync(byte[] save) => LoadSimulationWithSourceAsync(save, null);

    private async Task LoadSimulationWithSourceAsync(byte[] save, string? sourcePath, bool resumeRestartMode = false, bool isSafeModeRecovery = false)
    {
        _runTimer.Stop();
        await _advanceGate.WaitAsync();
        try
        {
            if (_session is null || _savingLastExit) return;
            _loadingSimulation = true;
            InvalidateTeleporterDrag();
            _pendingHighlightedDrag = null;
            Viewport.CancelHighlightedDrag();
            var mainDirectory = _globalSettings.Main.MainDirectory;
            await _session.LoadWithNativeRestartSourcesAsync(save,
                source => SetupStorageService.ResolveNativeRestartSource(source, sourcePath, mainDirectory));
            if (resumeRestartMode) await _session.ResumeRestartModeAsync();
            _ownsRestartContinuation = resumeRestartMode;
            CloseNativeTeleporterEditor();
            _toolWindows.Close(NewTeleporterToolKey);
            _playerBotHighlights.Clear();
            _shapeWorldGeneration++;
            _shapeDimensions = ShapeCreationDimensions.Default;
            _shapesWindow?.SetDimensions(_shapeDimensions);
            _worldLoadGeneration++;
            _safeModeRecoveryWorld = isSafeModeRecovery;
            // A rejected load publishes no snapshot and must keep user-closed
            // graphs closed. Loading publications defer this until success.
            _automaticEvolutionGraphBoundary = null;
            SynchronizeAutomaticEvolutionGraphs(_session.LatestSnapshot);
            await SynchronizeLoadedWorldAsync();
            _nativeRobotInfoWindow?.SynchronizePostDnaMemoryWatch(reset: true);
        }
        finally
        {
            _loadingSimulation = false;
            _advanceGate.Release();
        }
    }

    private Task AdvanceSimulationAsync(uint count) => AdvanceSimulationCoreAsync(count);

    private async Task<(ulong Tick, ulong Advanced)> AdvanceSimulationCoreAsync(uint count,
        SimulationSession? expectedSession = null, int? expectedGeneration = null, CancellationToken cancellationToken = default)
    {
        await _advanceGate.WaitAsync(cancellationToken);
        try
        {
            if (expectedSession is not null && (_windowClosed || expectedSession != _session
                || expectedGeneration != _worldLoadGeneration))
                throw new InvalidOperationException("This console belongs to a previous simulation.");
            if (_windowClosed || _session is not { } session || count == 0 || _savingLastExit)
                return (_session?.LatestSnapshot.Tick ?? 0, 0);

            var startingTick = session.LatestSnapshot.Tick;
            // A recovered pending checkpoint already contains the completed cycle.
            if (session.LatestSnapshot.OrdinaryRestartPending)
                await session.FinishOrdinaryRestartAsync();
            var requiresOriginalSource = session.LatestSnapshot.NativeCycle.HasValue
                && await session.HasUnresolvedNativeRepopulationSourcesAsync();
            while (count > 0)
            {
                cancellationToken.ThrowIfCancellationRequested();
                var before = session.LatestSnapshot;
                if (_lastAutomaticSaveTick is { } savedTick && before.Tick < savedTick)
                    _lastAutomaticSaveTick = null;
                var interval = _liveEnvironment.Recording?.AutosaveInterval ?? 0;
                // With restart enabled, let the host retain the completed cycle
                // before another engine tick can consume the pending restart.
                var observesOrdinaryRestart = before.OrdinaryRestartAvailable && before.OrdinaryRestartOnExtinction;
                var batch = observesOrdinaryRestart || requiresOriginalSource
                    ? 1u
                    : CycleAutosavePolicy.LimitBatch(before.Tick, count, _safemodeBackup, interval, before.LocalMode, before.NativeCycle);
                try
                {
                    if (_deadRecordingEnabled)
                    {
                        var deaths = new List<DeathRecordSnapshot>();
                        var weight = _globalSettings.Main.EffectiveFindBestWeight;
                        try
                        {
                            if (requiresOriginalSource)
                                await StepResolvingNativeRepopulationSourcesAsync(session, cancellationToken, weight, deaths.AddRange);
                            else
                                await session.StepRecordingDeathsAsync(batch, weight, deaths.AddRange, cancellationToken);
                        }
                        finally { await RecordSimulationDeathsAsync(deaths); }
                    }
                    else if (requiresOriginalSource)
                        await StepResolvingNativeRepopulationSourcesAsync(session, cancellationToken);
                    else
                        await session.StepAsync(batch, cancellationToken);
                }
                catch (MissingNativeRepopulationSourceException)
                {
                    _runTimer.Stop();
                    break;
                }
                count -= batch;
                var after = session.LatestSnapshot;
                if (_lastAutomaticSaveTick != after.Tick
                    && CycleAutosavePolicy.IsDue(after.Tick, _safemodeBackup, interval, after.LocalMode, after.NativeCycle))
                    await SaveAutomaticCheckpointAsync();
                if (after.OrdinaryRestartPending)
                    await session.FinishOrdinaryRestartAsync();
                if (count > 0 && requiresOriginalSource)
                    requiresOriginalSource = await session.HasUnresolvedNativeRepopulationSourcesAsync();
            }
            var tick = session.LatestSnapshot.Tick;
            return (tick, tick >= startingTick ? tick - startingTick : 0);
        }
        finally { _advanceGate.Release(); }
    }

    private async Task StepResolvingNativeRepopulationSourcesAsync(SimulationSession session, CancellationToken cancellationToken,
        int weightPercent = 100,
        Action<IReadOnlyList<DeathRecordSnapshot>>? onCommitted = null)
    {
        List<ulong> cancelledLineages = [];
        while (true)
        {
            try
            {
                await session.StepRequiringNativeRepopulationSourceAsync(cancelledLineages, weightPercent, onCommitted, cancellationToken);
                return;
            }
            catch (MissingNativeRepopulationSourceException error)
            {
                var source = error.RequiredSource;
                if (cancelledLineages.Contains(source.LineageId))
                {
                    _viewModel.Status = $"PAUSED · COULD NOT SKIP ORIGINAL DNA · {source.Filename}";
                    throw;
                }

                var resolution = await ResolveNativeRepopulationSourceAsync(session, source);
                if (resolution == RepopulationSourceResolution.Unavailable) throw;
                if (resolution == RepopulationSourceResolution.Cancelled)
                    cancelledLineages.Add(source.LineageId);
            }
        }
    }

    private enum RepopulationSourceResolution { Bound, Cancelled, Unavailable }

    private async Task<RepopulationSourceResolution> ResolveNativeRepopulationSourceAsync(
        SimulationSession session, NativeRestartSource source)
    {
        try
        {
            var local = SetupStorageService.ResolveNativeRestartSource(source, null, _globalSettings.Main.MainDirectory);
            if (local is not null && await session.BindNativeRepopulationSourceAsync(new(source.LineageId, local)))
                return RepopulationSourceResolution.Bound;

            var previousStatus = _viewModel.Status;
            var locatingStatus = $"ORIGINAL DNA REQUIRED · {source.Filename}";
            _viewModel.Status = locatingStatus;
            var selected = await _storage.OpenNativeRepopulationSourceAsync(this, source);
            if (_windowClosed) return RepopulationSourceResolution.Unavailable;
            if (selected is null)
            {
                if (_viewModel.Status == locatingStatus) _viewModel.Status = previousStatus;
                return RepopulationSourceResolution.Cancelled;
            }
            var selectedDirectory = SetupStorageService.SelectedDnaDirectory(selected.LocalPath);
            if (await session.BindNativeRepopulationSourceAsync(new(source.LineageId, selected.Content, selectedDirectory)))
            {
                if (_viewModel.Status == locatingStatus) _viewModel.Status = previousStatus;
                return RepopulationSourceResolution.Bound;
            }
            _viewModel.Status = $"PAUSED · DNA COULD NOT BE LOADED · {selected.Name}";
        }
        catch (Exception error) when (error is IOException or UnauthorizedAccessException or System.Text.DecoderFallbackException)
        {
            _viewModel.Status = $"PAUSED · DNA COULD NOT BE READ · {error.Message}";
        }
        return RepopulationSourceResolution.Unavailable;
    }

    private async Task SaveAutomaticCheckpointAsync()
    {
        if (_session is null) return;
        try
        {
            var checkpointTick = _session.LatestSnapshot.Tick;
            await _storage.SaveAutosaveAsync(await _session.SaveAsync());
            _lastAutomaticSaveTick = checkpointTick;
            _diagnostics.RecordCheckpoint(checkpointTick, _storage.FindLatestAutosave());
        }
        catch (Exception error) { _viewModel.Status = $"AUTOSAVE ERROR · {error.Message}"; }
    }

    private void Window_Closing(object? sender, WindowClosingEventArgs e)
    {
        if (e.Cancel || e.CloseReason is not (WindowCloseReason.WindowClosing or WindowCloseReason.Undefined)
            || _skipLastExitCheckpoint || !_startupComplete || !_storage.SupportsLastExitSimulation
            || _session is not { } session)
            return;

        e.Cancel = true;
        if (_savingLastExit || _navigationInFlight) return;
        if (OwnedWindows.Any(window => window.IsVisible && window.IsDialog))
        {
            _viewModel.Status = "CLOSE THE OPEN DIALOG BEFORE QUITTING";
            return;
        }

        _savingLastExit = true;
        var wasRunning = _runTimer.IsEnabled;
        _runTimer.Stop();
        Dispatcher.UIThread.Post(() => _ = CompleteWindowCloseAsync(session, wasRunning));
    }

    private async Task CompleteWindowCloseAsync(SimulationSession session, bool wasRunning)
    {
        var close = false;
        var wasEnabled = IsEnabled;
        var disablingForSave = false;
        var restartMode = false;
        var failure = "LAST SIMULATION SAVE FAILED";
        await _advanceGate.WaitAsync();
        try
        {
            if (_windowClosed || !ReferenceEquals(_session, session) || _navigationInFlight) return;
            restartMode = session.LatestSnapshot.LocalMode.HasActiveRestartMode;
            if (restartMode || _storage.SupportsLastExitSettings)
            {
                if (OwnedWindows.Any(window => window.IsVisible && window.IsDialog)) return;
                if (!IsVisible)
                {
                    _diagnostics.RestoreFromTray();
                    Show();
                    WindowState = WindowState.Normal;
                }
            }
            var restartChoice = restartMode
                ? await new RestartQuitWindow().ShowDialog<RestartQuitChoice>(this)
                : RestartQuitChoice.Stop;
            if (restartChoice == RestartQuitChoice.Cancel || _windowClosed || !ReferenceEquals(_session, session)) return;
            if (restartMode && restartChoice == RestartQuitChoice.Keep)
            {
                IsEnabled = false;
                disablingForSave = true;
                failure = "RESTART CONTINUATION SAVE FAILED";
                _viewModel.Status = "SAVING RESTART CONTINUATION";
                await _storage.SaveRestartContinuationAsync(await session.SaveAsync());
            }
            else
            {
                if (restartMode || _ownsRestartContinuation)
                {
                    failure = "RESTART CONTINUATION CLEANUP FAILED";
                    _storage.ClearRestartContinuation();
                    _ownsRestartContinuation = false;
                    if (restartMode && session.LatestSnapshot.LocalMode.Evolution is not null)
                        await session.ClearRestartModeHidingAsync();
                }
                var choice = _storage.SupportsLastExitSettings
                    ? await new LiveQuitSettingsWindow(_storage.HasLastExitSettings).ShowDialog<LiveQuitChoice>(this)
                    : LiveQuitChoice.KeepSettings;
                if (choice == LiveQuitChoice.Cancel || _windowClosed || !ReferenceEquals(_session, session)) return;

                IsEnabled = false;
                disablingForSave = true;
                _viewModel.Status = "SAVING LAST SIMULATION";
                byte[] simulation;
                if (choice == LiveQuitChoice.SaveSettings)
                {
                    failure = "LAST EXIT SETTINGS SAVE FAILED";
                    var capture = await session.CaptureLastExitAsync(
                        CreateNextWorldSetup() with { TicksPerUpdate = _ticksPerUpdate }, stopRestartMode: restartMode);
                    await _storage.SaveLastExitSettingsAsync(capture.Settings);
                    simulation = capture.Simulation;
                }
                else
                {
                    failure = "LAST SIMULATION SAVE FAILED";
                    simulation = restartMode ? await session.SaveWithoutRestartModeAsync() : await session.SaveAsync();
                }
                failure = "LAST SIMULATION SAVE FAILED";
                await _storage.SaveLastExitSimulationAsync(simulation);
            }
            _skipLastExitCheckpoint = true;
            close = true;
        }
        catch (Exception error)
        {
            _skipLastExitCheckpoint = !restartMode && !_ownsRestartContinuation;
            if (!_windowClosed)
                _viewModel.Status = $"{failure} · {error.Message} · {(_skipLastExitCheckpoint ? "CLOSE AGAIN TO EXIT" : "CLOSE AGAIN TO RETRY")}";
        }
        finally
        {
            _advanceGate.Release();
            if (!close && !_windowClosed)
            {
                _savingLastExit = false;
                if (disablingForSave) IsEnabled = wasEnabled;
                if (wasRunning) _runTimer.Start();
            }
        }
        if (close && !_windowClosed) Close();
    }

    private async void Window_Closed(object? sender, EventArgs e)
    {
        _windowClosed = true;
        CloseNativeTeleporterEditor();
        InvalidateTeleporterDrag();
        _runTimer.Stop();
        _runtimeClock.Stop();
        _toolWindows.CloseAll();
        _trayIcon?.Dispose();
        var session = _session;
        _session = null;
        if (session is not null) await session.DisposeAsync();
    }

    private void Graphs_Click(object? sender, RoutedEventArgs e) => ShowGraphSuite();

    private GraphSuiteWindow ShowGraphSuite() =>
        _toolWindows.ShowOrActivate(
            "graph-suite",
            () =>
            {
                var window = new GraphSuiteWindow(_session?.LatestSnapshot ?? EngineSnapshot.Empty);
                window.SamplingChanged += ConfigureGraphSampling;
                return window;
            });

    private void SynchronizeAutomaticEvolutionGraphs(EngineSnapshot snapshot)
    {
        // MDIForm1.fixcam opens these graphs for restart modes 4/5, not the
        // F1 testing phase (6) or Zerobot search/testing (7/8).
        if (snapshot.LocalMode is not {
            Status: LocalModeStatus.Running,
            Kind: LocalModeKind.SimpleSurvival or LocalModeKind.EcoSurvival,
            Configuration: { StagedEvolution: { AutoGraphs: true, Mode: EvolutionMode.SimpleSurvival or EvolutionMode.Eco } },
            Evolution: { Phase: "evolving" },
        })
        {
            _automaticEvolutionGraphBoundary = null;
            return;
        }
        var boundary = (snapshot.LocalMode.Kind, snapshot.LocalMode.RestartCount);
        if (_automaticEvolutionGraphBoundary is { } previous && previous == boundary) return;
        _automaticEvolutionGraphBoundary = boundary;
        var suite = ShowGraphSuite();
        suite.SetSnapshot(snapshot);
        suite.OpenEvolutionGraphs();
    }

    private async void ConfigureGraphSampling(GraphSamplingSettings settings)
    {
        if (_session is null) return;
        try { await _session.ConfigureGraphSamplingAsync(settings); }
        catch (Exception error) { _viewModel.Status = $"GRAPH ERROR · {error.Message}"; }
    }

    private void ContestResults_Click(object? sender, RoutedEventArgs e)
    {
        var result = ContestResultSnapshot.FromSnapshot(_session?.LatestSnapshot ?? EngineSnapshot.Empty);
        _toolWindows.ShowOrActivate("contest-results", () => new ContestResultsWindow(result));
    }
}

internal enum EnvironmentPlacement { None, Obstacle, Teleporter }
