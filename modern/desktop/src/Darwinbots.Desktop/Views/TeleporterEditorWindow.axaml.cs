using Avalonia.Controls;
using Avalonia.Interactivity;
using Darwinbots.Desktop.Core;

namespace Darwinbots.Desktop.Views;

public sealed partial class TeleporterEditorWindow : Window
{
    private readonly TeleporterSnapshot _original;

    public TeleporterEditorWindow() : this(new TeleporterSnapshot(0, [0f, 0f], 1f, [0f, 0f]))
    {
    }

    public TeleporterEditorWindow(TeleporterSnapshot teleporter)
    {
        _original = teleporter ?? throw new ArgumentNullException(nameof(teleporter));
        InitializeComponent();
        Mode.SelectedIndex = ModeIndex(teleporter.Mode);
        CenterX.Value = (decimal)teleporter.Center.ElementAtOrDefault(0);
        CenterY.Value = (decimal)teleporter.Center.ElementAtOrDefault(1);
        DestinationX.Value = (decimal)teleporter.Destination.ElementAtOrDefault(0);
        DestinationY.Value = (decimal)teleporter.Destination.ElementAtOrDefault(1);
        Radius.Value = (decimal)teleporter.Radius;
        PollInterval.Value = teleporter.PollInterval;
        Capacity.Value = teleporter.Capacity;
        BackflowLimit.Value = teleporter.BackflowLimit;
        TeleportVegetables.IsChecked = teleporter.TeleportVegetables;
        TeleportCorpses.IsChecked = teleporter.TeleportCorpses;
        TeleportHeterotrophs.IsChecked = teleporter.TeleportHeterotrophs;
        FixedPosition.IsChecked = teleporter.FixedPosition;
        RespectShapes.IsChecked = teleporter.RespectShapes;
        Path.Text = teleporter.Path;
        InboundPath.Text = teleporter.InboundPath;
        OutboundPath.Text = teleporter.OutboundPath;
        Statistics.Text = $"Sent {teleporter.Sent:N0} · received {teleporter.Received:N0}";
    }

    public TeleporterSnapshot? Result { get; private set; }

    private static int ModeIndex(string? mode) => mode?.Trim().ToLowerInvariant() switch
    {
        "inbound" => 1,
        "outbound" => 2,
        "internet" => 3,
        _ => 0,
    };

    private void Apply_Click(object? sender, RoutedEventArgs e)
    {
        if (Mode.SelectedIndex == 3)
        {
            Statistics.Text = "Internet teleporters are disabled in offline mode.";
            return;
        }
        Result = _original with
        {
            Mode = Mode.SelectedIndex switch
            {
                1 => "inbound",
                2 => "outbound",
                _ => "intrasim",
            },
            Center = [(float)(CenterX.Value ?? 0m), (float)(CenterY.Value ?? 0m)],
            Destination = [(float)(DestinationX.Value ?? 0m), (float)(DestinationY.Value ?? 0m)],
            Radius = (float)(Radius.Value ?? 1m),
            PollInterval = (uint)(PollInterval.Value ?? 1m),
            Capacity = (int)(Capacity.Value ?? 1m),
            BackflowLimit = (uint)(BackflowLimit.Value ?? 0m),
            TeleportVegetables = TeleportVegetables.IsChecked == true,
            TeleportCorpses = TeleportCorpses.IsChecked == true,
            TeleportHeterotrophs = TeleportHeterotrophs.IsChecked == true,
            FixedPosition = FixedPosition.IsChecked == true,
            RespectShapes = RespectShapes.IsChecked == true,
            Path = Path.Text?.Trim() ?? string.Empty,
            InboundPath = InboundPath.Text?.Trim() ?? string.Empty,
            OutboundPath = OutboundPath.Text?.Trim() ?? string.Empty,
        };
        Close();
    }

    private void Cancel_Click(object? sender, RoutedEventArgs e) => Close();
}
