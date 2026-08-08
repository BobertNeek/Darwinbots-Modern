using Avalonia.Controls;
using Avalonia.Interactivity;
using Darwinbots.Desktop.Core;

namespace Darwinbots.Desktop.Views;

public sealed partial class FileTeleporterEditorWindow : Window
{
    private readonly FileTeleporterSnapshot _original;

    public FileTeleporterEditorWindow() : this(new FileTeleporterSnapshot(
        0, [0f, 0f], 1f, "", "", 1, 0, 1,
        new TeleporterFilterSnapshot(true, true, true), 0, 0))
    {
    }

    public FileTeleporterEditorWindow(FileTeleporterSnapshot teleporter)
    {
        _original = teleporter ?? throw new ArgumentNullException(nameof(teleporter));
        InitializeComponent();
        CenterX.Value = (decimal)teleporter.Center.ElementAtOrDefault(0);
        CenterY.Value = (decimal)teleporter.Center.ElementAtOrDefault(1);
        Radius.Value = (decimal)teleporter.Radius;
        InboundPath.Text = teleporter.InboundPath;
        OutboundPath.Text = teleporter.OutboundPath;
        PollInterval.Value = teleporter.PollInterval;
        Capacity.Value = teleporter.Capacity;
        TeleportVegetables.IsChecked = teleporter.Filters.TeleportVegetables;
        TeleportCorpses.IsChecked = teleporter.Filters.TeleportCorpses;
        TeleportHeterotrophs.IsChecked = teleporter.Filters.TeleportHeterotrophs;
        Statistics.Text = $"Sent {teleporter.Sent:N0} · received {teleporter.Received:N0}" +
            (string.IsNullOrWhiteSpace(teleporter.BackflowGuardPath) ? string.Empty : " · backflow guard active");
    }

    public FileTeleporterSnapshot? Result { get; private set; }

    private void Apply_Click(object? sender, RoutedEventArgs e)
    {
        var inbound = InboundPath.Text?.Trim() ?? string.Empty;
        var outbound = OutboundPath.Text?.Trim() ?? string.Empty;
        if (string.IsNullOrWhiteSpace(inbound) && string.IsNullOrWhiteSpace(outbound))
        {
            Statistics.Text = "Enter an inbound or outbound local path.";
            return;
        }
        Result = _original with
        {
            Center = [(float)(CenterX.Value ?? 0m), (float)(CenterY.Value ?? 0m)],
            Radius = (float)(Radius.Value ?? 1m),
            InboundPath = inbound,
            OutboundPath = outbound,
            PollInterval = (uint)(PollInterval.Value ?? 1m),
            Capacity = (int)(Capacity.Value ?? 1m),
            Filters = new TeleporterFilterSnapshot(
                TeleportVegetables.IsChecked == true,
                TeleportCorpses.IsChecked == true,
                TeleportHeterotrophs.IsChecked == true),
        };
        Close();
    }

    private void Cancel_Click(object? sender, RoutedEventArgs e) => Close();
}
