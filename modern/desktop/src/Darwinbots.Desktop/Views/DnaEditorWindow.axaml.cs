using Avalonia.Controls;
using Avalonia.Interactivity;
using Darwinbots.Desktop.Core;
using Darwinbots.Desktop.Services;

namespace Darwinbots.Desktop.Views;

public sealed partial class DnaEditorWindow : Window
{
    private SimulationSession _session = null!;
    private OrganismSnapshot _organism = null!;
    private readonly IDesktopStorageService _storage;
    private MutationProfileUpdate? _mutationProfile;

    public DnaEditorWindow() : this(AvaloniaDesktopStorageService.Instance)
    {
    }

    private DnaEditorWindow(IDesktopStorageService storage)
    {
        _storage = storage ?? throw new ArgumentNullException(nameof(storage));
        InitializeComponent();
        SaveBotButton.IsEnabled = false;
    }

    public DnaEditorWindow(SimulationSession session, OrganismSnapshot organism) :
        this(session, organism, AvaloniaDesktopStorageService.Instance)
    {
    }

    public DnaEditorWindow(
        SimulationSession session,
        OrganismSnapshot organism,
        IDesktopStorageService storage) : this(storage)
    {
        _session = session;
        _organism = organism;
        Identity.Text = $"{organism.Slot}:{organism.Generation}";
        Opened += LoadDna;
    }

    private async void LoadDna(object? sender, EventArgs e)
    {
        try
        {
            var file = await _session.ExportRobotFileAsync(_organism.Slot, _organism.Generation);
            _mutationProfile = file.MutationProfile
                ?? throw new InvalidDataException("Engine returned no mutation profile.");
            Editor.Text = file.Dna;
            SaveBotButton.IsEnabled = true;
        }
        catch (Exception error) { Status.Text = error.Message; }
    }

    private async void Apply_Click(object? sender, RoutedEventArgs e)
    {
        try
        {
            await _session.ReplaceDnaAsync(_organism.Slot, _organism.Generation, Editor.Text ?? string.Empty);
            Status.Text = "DNA APPLIED";
        }
        catch (Exception error) { Status.Text = error.Message; }
    }

    private async void Clone_Click(object? sender, RoutedEventArgs e)
    {
        try
        {
            await _session.CloneWithDnaAsync(
                _organism.Slot,
                _organism.Generation,
                [_organism.Position[0] + 24f, _organism.Position[1] + 24f],
                Editor.Text ?? string.Empty);
            Status.Text = "EDITED DNA APPLIED TO CLONE";
        }
        catch (Exception error) { Status.Text = error.Message; }
    }

    private async void Save_Click(object? sender, RoutedEventArgs e)
    {
        try
        {
            if (_mutationProfile is null)
                throw new InvalidOperationException("Load robot DNA and mutation settings before exporting.");
            Status.Text = await _storage.SaveDnaAsync(
                this,
                $"bot-{_organism.Slot}-{_organism.Generation}.txt",
                Editor.Text ?? string.Empty,
                _mutationProfile) ? "BOT EXPORTED" : "EXPORT CANCELLED";
        }
        catch (Exception error) { Status.Text = $"EXPORT FAILED · {error.Message}"; }
    }
}
