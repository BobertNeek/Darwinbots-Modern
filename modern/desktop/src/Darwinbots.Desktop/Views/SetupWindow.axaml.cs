using System.Collections.ObjectModel;
using Avalonia.Controls;
using Avalonia.Input;
using Avalonia.Interactivity;
using Avalonia.Platform.Storage;
using Darwinbots.Desktop.Core;

namespace Darwinbots.Desktop.Views;

public sealed partial class SetupWindow : Window
{
    private bool _controlsReady;
    private EnvironmentUpdate _environment = EnvironmentUpdate.Default;
    private const string ZerobotDna = "0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0";
    private const string FeederDna = "cond\nstart\n-2 .shoot store\n50 .shootval store\n314 rnd .aimdx store\nstop";
    private const string FeederReproducerDna = "cond\nstart\n-2 302 1 rnd rnd rnd rnd rnd rnd mult add .shoot store\n50 .shootval store\n314 rnd .aimdx store\nstop";

    public ObservableCollection<SetupSpeciesRow> Species { get; } = [];
    public event Action<WorldSetupOptions>? WorldCreated;

    public SetupWindow()
    {
        InitializeComponent();
        DataContext = this;
        _controlsReady = true;
        LoadMode(StartingMode.StarterBotsAndVegetables);
    }

    private void Mode_Checked(object? sender, RoutedEventArgs e)
    {
        if (!_controlsReady) return;
        LoadMode(CurrentMode());
    }

    private void LoadMode(StartingMode mode)
    {
        UpdateSustenanceAvailability(mode);
        Species.Clear();
        try
        {
            if (mode == StartingMode.StarterBotsAndVegetables)
            {
                Species.Add(new("Alga Minimalis", ReadBundled("Alga_Minimalis_Chloroplastus.txt"), true, 0xff4c963b, 300, 1_000, 0.5, true));
                Species.Add(new("Animal Minimalis", ReadBundled("Animal_Minimalis.txt"), false, 0xff239ac0, 100, 1_000, 1.0, true));
            }
            else
            {
                Species.Add(new("Zerobot", ZerobotDna, false, 0xff59636b, 100, 1_000, 10.0, false));
                if (mode == StartingMode.ZerobotsAndVegetables)
                    Species.Add(new("Energy Feeder", FeederDna, true, 0xff4c963b, 60, 2_000, 0.5, true));
            }
            SetupStatus.Text = string.Empty;
        }
        catch (Exception error) { SetupStatus.Text = error.Message; }
    }

    private static string ReadBundled(string name) => File.ReadAllText(Path.Combine(AppContext.BaseDirectory, "Bots", name));

    private async void AddDna_Click(object? sender, RoutedEventArgs e)
    {
        var files = await StorageProvider.OpenFilePickerAsync(new FilePickerOpenOptions
        {
            Title = "Add Darwinbots DNA species",
            AllowMultiple = true,
            FileTypeFilter = [new FilePickerFileType("Darwinbots robot") { Patterns = ["*.txt"] }],
        });
        foreach (var file in files)
        {
            await using var stream = await file.OpenReadAsync();
            using var reader = new StreamReader(stream);
            Species.Add(new(Path.GetFileNameWithoutExtension(file.Name), await reader.ReadToEndAsync(), false, 0xffd07a2d, 50, 1_000, 1.0, false));
        }
    }

    private void RemoveSpecies_Click(object? sender, RoutedEventArgs e)
    {
        if (sender is Button { Tag: SetupSpeciesRow row }) Species.Remove(row);
    }

    private void CreateWorld_Click(object? sender, RoutedEventArgs e)
    {
        if (Species.Count == 0) { SetupStatus.Text = "Add at least one species."; return; }
        var sustenance = (ZerobotSustenance)Math.Max(0, Sustenance.SelectedIndex);
        var species = Species.Select((row, index) => row.ToImport(index)).ToList();
        if (CurrentMode() != StartingMode.StarterBotsAndVegetables)
        {
            if (sustenance == ZerobotSustenance.PhotosyntheticZerobots)
                species[0] = species[0] with { Vegetable = true };
            if (sustenance is ZerobotSustenance.FeederReproducerVegetables or ZerobotSustenance.EnergyOnlyVegetables
                && species.All(value => !value.Name.Contains("Feeder", StringComparison.OrdinalIgnoreCase)))
            {
                species.Add(new SpeciesImport("Zerobot Feeder", sustenance == ZerobotSustenance.FeederReproducerVegetables ? FeederReproducerDna : FeederDna, true, 0xff4c963b, 60, 2_000, 20, true, 0.5f));
            }
        }
        var speed = Speed.SelectedIndex switch { 0 => 1u, 1 => 5u, 2 => 20u, _ => 100u };
        WorldCreated?.Invoke(new WorldSetupOptions
        {
            StartingMode = CurrentMode(),
            ZerobotSustenance = sustenance,
            AutomaticZerobotProgression = AutomaticProgression.IsChecked == true,
            Backend = ((ComboBoxItem)Backend.SelectedItem!).Content?.ToString() ?? "Auto",
            PopulationCapacity = (int)(PopulationCap.Value ?? 25_000),
            VegetablePopulationCap = (int)(VegetablePopulationCap.Value ?? 500),
            WorldWidth = (float)(WorldWidth.Value ?? 16_000),
            WorldHeight = (float)(WorldHeight.Value ?? 12_000),
            MetabolismCost = _environment.MetabolismCost,
            VegetableEnergyPerTick = _environment.VegetableEnergyPerTick,
            SunlightEnergy = _environment.SunlightEnergy,
            Gravity = _environment.Gravity.ToArray(),
            Drag = _environment.Drag,
            BrownianMotion = _environment.BrownianMotion,
            Physics = _environment.Physics,
            Shots = _environment.Shots,
            Vegetation = _environment.Vegetation,
            AutoSpeciation = _environment.AutoSpeciation,
            SpeciationGeneticDistancePercent = _environment.SpeciationGeneticDistancePercent,
            ToroidalWorld = _environment.ToroidalWorld,
            TicksPerUpdate = speed,
            Species = species,
        });
    }

    private async void Advanced_Click(object? sender, RoutedEventArgs e)
    {
        try
        {
            var dialog = new AdvancedSettingsWindow(_environment);
            await dialog.ShowDialog(this);
            if (!dialog.Accepted) return;
            _environment = dialog.Update;
            SetupStatus.Text = "ADVANCED SETTINGS APPLIED";
        }
        catch (Exception error)
        {
            SetupStatus.Text = $"ADVANCED SETTINGS FAILED: {error.Message}";
        }
    }

    private void NewWorld_Click(object? sender, RoutedEventArgs e)
    {
        _environment = EnvironmentUpdate.Default;
        Backend.SelectedIndex = 0;
        WorldWidth.Value = 16_000;
        WorldHeight.Value = 12_000;
        PopulationCap.Value = 25_000;
        VegetablePopulationCap.Value = 500;
        Speed.SelectedIndex = 3;
        Sustenance.SelectedIndex = 3;
        AutomaticProgression.IsChecked = true;
        StarterMode.IsChecked = true;
        Mode_Checked(StarterMode, e);
        SetupStatus.Text = "New world defaults restored.";
    }

    private async void OpenSave_Click(object? sender, RoutedEventArgs e)
    {
        var files = await StorageProvider.OpenFilePickerAsync(new FilePickerOpenOptions
        {
            Title = "Open Darwinbots Modern save",
            AllowMultiple = false,
            FileTypeFilter = [new FilePickerFileType("Darwinbots Modern save") { Patterns = ["*.db3s"] }],
        });
        var file = files.FirstOrDefault();
        if (file is null) return;
        WorldCreated?.Invoke(new WorldSetupOptions { LoadSavePath = file.Path.LocalPath });
    }

    private void Exit_Click(object? sender, RoutedEventArgs e) => Close();

    private async void About_Click(object? sender, RoutedEventArgs e)
    {
        var close = new Button { Content = "CLOSE", HorizontalAlignment = Avalonia.Layout.HorizontalAlignment.Right };
        var dialog = new Window
        {
            Title = "About Darwinbots Modern",
            Width = 440,
            Height = 270,
            CanResize = false,
            WindowStartupLocation = WindowStartupLocation.CenterOwner,
            Content = new StackPanel
            {
                Margin = new Avalonia.Thickness(28),
                Spacing = 14,
                Children =
                {
                    new TextBlock { Text = "DARWINBOTS MODERN", FontSize = 25, FontWeight = Avalonia.Media.FontWeight.Bold },
                    new TextBlock { Text = "Windows 10 alpha", FontWeight = Avalonia.Media.FontWeight.SemiBold },
                    new TextBlock
                    {
                        Text = "A modern, legacy-DNA-compatible artificial life simulator with CPU and portable GPU backends.",
                        TextWrapping = Avalonia.Media.TextWrapping.Wrap,
                    },
                    close,
                },
            },
        };
        close.Click += (_, _) => dialog.Close();
        await dialog.ShowDialog(this);
    }

    private void SetupWindow_KeyDown(object? sender, KeyEventArgs e)
    {
        if (e.Key == Key.F1)
        {
            About_Click(sender, new RoutedEventArgs());
            e.Handled = true;
            return;
        }

        if (!e.KeyModifiers.HasFlag(KeyModifiers.Control)) return;
        switch (e.Key)
        {
            case Key.Enter:
                CreateWorld_Click(sender, new RoutedEventArgs());
                e.Handled = true;
                break;
            case Key.N:
                NewWorld_Click(sender, new RoutedEventArgs());
                e.Handled = true;
                break;
            case Key.O:
                OpenSave_Click(sender, new RoutedEventArgs());
                e.Handled = true;
                break;
            case Key.OemComma:
                Advanced_Click(sender, new RoutedEventArgs());
                e.Handled = true;
                break;
        }
    }

    private void UpdateSustenanceAvailability(StartingMode mode)
    {
        var enabled = mode != StartingMode.StarterBotsAndVegetables;
        Sustenance.IsEnabled = enabled;
        AutomaticProgression.IsEnabled = enabled;
        SustenanceLabel.Text = "ZEROBOT SUSTENANCE";
        SustenanceHint.Text = enabled
            ? "Optional assistance while functional DNA evolves."
            : "Starter bots always use normal metabolism and DB2 vegetable feeding.";
    }

    private void Recover_Click(object? sender, RoutedEventArgs e)
    {
        var autosaveDirectory = Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData), "Darwinbots Modern", "Autosaves");
        var latest = Directory.Exists(autosaveDirectory)
            ? Directory.GetFiles(autosaveDirectory, "*.db3s").OrderByDescending(File.GetLastWriteTimeUtc).FirstOrDefault()
            : null;
        if (latest is null) { SetupStatus.Text = "No autosave is available."; return; }
        WorldCreated?.Invoke(new WorldSetupOptions { LoadSavePath = latest });
    }

    private StartingMode CurrentMode() => ZerobotMode.IsChecked == true
        ? StartingMode.Zerobots
        : ZerobotVegetableMode.IsChecked == true
            ? StartingMode.ZerobotsAndVegetables
            : StartingMode.StarterBotsAndVegetables;
}
