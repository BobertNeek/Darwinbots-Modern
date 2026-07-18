using Avalonia.Controls;
using Avalonia.Interactivity;
using Avalonia.Platform.Storage;
using Darwinbots.Desktop.Core;

namespace Darwinbots.Desktop.Views;

public sealed partial class DnaEditorWindow : Window
{
    private SimulationSession _session = null!;
    private OrganismSnapshot _organism = null!;

    public DnaEditorWindow() => InitializeComponent();

    public DnaEditorWindow(SimulationSession session, OrganismSnapshot organism) : this()
    {
        _session = session;
        _organism = organism;
        Identity.Text = $"{organism.Slot}:{organism.Generation}";
        Opened += LoadDna;
    }

    private async void LoadDna(object? sender, EventArgs e)
    {
        try { Editor.Text = await _session.ExportDnaAsync(_organism.Slot, _organism.Generation); }
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
        var file = await StorageProvider.SaveFilePickerAsync(new FilePickerSaveOptions
        {
            Title = "Export Darwinbots robot DNA",
            SuggestedFileName = $"bot-{_organism.Slot}-{_organism.Generation}.txt",
            FileTypeChoices = [new FilePickerFileType("Darwinbots robot") { Patterns = ["*.txt"] }],
        });
        if (file is null) return;
        await using var stream = await file.OpenWriteAsync();
        stream.SetLength(0);
        await using var writer = new StreamWriter(stream);
        await writer.WriteAsync(Editor.Text ?? string.Empty);
        Status.Text = "BOT EXPORTED";
    }
}
