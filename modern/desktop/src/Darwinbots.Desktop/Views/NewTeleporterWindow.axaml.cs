using Avalonia.Controls;
using Avalonia.Interactivity;
using Darwinbots.Desktop.Core;

namespace Darwinbots.Desktop.Views;

public sealed partial class NewTeleporterWindow : Window
{
    private readonly Func<NewTeleporterWindow, NativeTeleporterOptions, Task> _create;

    public NewTeleporterWindow(Func<NewTeleporterWindow, NativeTeleporterOptions, Task> create)
    {
        _create = create;
        InitializeComponent();
        Mode.SelectionChanged += (_, _) => UpdateTypeControls();
        UpdateTypeControls();
    }

    private NativeTeleporterKind Kind => Mode.SelectedIndex switch
    {
        1 => NativeTeleporterKind.Inbound,
        2 => NativeTeleporterKind.Outbound,
        _ => NativeTeleporterKind.Intrasim,
    };

    private void UpdateTypeControls()
    {
        var kind = Kind;
        PathSettings.IsEnabled = kind != NativeTeleporterKind.Intrasim;
        TransferFilters.IsEnabled = kind != NativeTeleporterKind.Inbound;
        PollingSettings.IsEnabled = kind == NativeTeleporterKind.Inbound;
    }

    private async void Apply_Click(object? sender, RoutedEventArgs e)
    {
        if (!ApplyButton.IsEnabled) return;
        ApplyButton.IsEnabled = CancelButton.IsEnabled = false;
        ErrorText.IsVisible = false;
        try
        {
            var kind = Kind;
            var path = FilePath.Text?.Trim() ?? string.Empty;
            if (kind == NativeTeleporterKind.Intrasim || string.IsNullOrWhiteSpace(path))
                path = string.Empty;
            else
                path = FileTeleporterPaths.Create(path, null).InboundDirectory;

            await _create(this, new NativeTeleporterOptions(
                kind,
                (uint)(PortalSize.Value ?? 300m),
                path,
                FixedPosition.IsChecked == true,
                new TeleporterFilterSnapshot(
                    TeleportVegetables.IsChecked == true,
                    TeleportCorpses.IsChecked == true,
                    TeleportHeterotrophs.IsChecked == true),
                (uint)(PollInterval.Value ?? 10m),
                (uint)(Capacity.Value ?? 10m)));
            if (IsVisible) Close();
        }
        catch (Exception error) when (error is ArgumentException or InvalidOperationException or NotSupportedException)
        {
            ErrorText.Text = error.Message;
            ErrorText.IsVisible = true;
            ApplyButton.IsEnabled = CancelButton.IsEnabled = true;
        }
    }

    private void Cancel_Click(object? sender, RoutedEventArgs e) => Close();
}
