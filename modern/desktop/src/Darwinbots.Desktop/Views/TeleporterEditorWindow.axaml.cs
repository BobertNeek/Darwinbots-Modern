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
        if (teleporter.Internet || string.Equals(teleporter.Mode.Trim(), "internet", StringComparison.OrdinalIgnoreCase))
            throw new ArgumentException("This legacy teleporter cannot be edited.", nameof(teleporter));
        InitializeComponent();
        Mode.SelectedIndex = ModeIndex(teleporter.Mode);
        CenterX.Value = (decimal)teleporter.Center.ElementAtOrDefault(0);
        CenterY.Value = (decimal)teleporter.Center.ElementAtOrDefault(1);
        DestinationX.Value = (decimal)teleporter.Destination.ElementAtOrDefault(0);
        DestinationY.Value = (decimal)teleporter.Destination.ElementAtOrDefault(1);
        DestinationX.IsEnabled = DestinationY.IsEnabled = !teleporter.RandomDestination;
        RandomDestinationNotice.IsVisible = teleporter.RandomDestination;
        Radius.Value = (decimal)teleporter.Radius;
        PollInterval.Value = teleporter.PollInterval;
        Capacity.Value = teleporter.Capacity;
        BackflowLimit.Value = teleporter.BackflowLimit;
        BackflowLimit.IsEnabled = !teleporter.RandomDestination;
        TeleportVegetables.IsChecked = teleporter.TeleportVegetables;
        TeleportCorpses.IsChecked = teleporter.TeleportCorpses;
        TeleportHeterotrophs.IsChecked = teleporter.TeleportHeterotrophs;
        FixedPosition.IsChecked = teleporter.FixedPosition;
        RespectShapes.IsChecked = teleporter.RespectShapes;
        Path.Text = teleporter.Path;
        InboundPath.Text = teleporter.InboundPath;
        OutboundPath.Text = teleporter.OutboundPath;
        Statistics.Text = $"Sent {teleporter.Sent:N0} Â· received {teleporter.Received:N0}";
    }

    public TeleporterSnapshot? Result { get; private set; }

    private static int ModeIndex(string? mode) => mode?.Trim().ToLowerInvariant() switch
    {
        "inbound" => 1,
        "outbound" => 2,
        _ => 0,
    };

    private void Apply_Click(object? sender, RoutedEventArgs e)
    {
        float[] center =
        [
            ReadGeometry(CenterX, _original.Center.ElementAtOrDefault(0)),
            ReadGeometry(CenterY, _original.Center.ElementAtOrDefault(1)),
        ];
        float[] destination =
        [
            ReadGeometry(DestinationX, _original.Destination.ElementAtOrDefault(0)),
            ReadGeometry(DestinationY, _original.Destination.ElementAtOrDefault(1)),
        ];
        Result = ObjectEditorPolicy.BuildTeleporterPayload(
            _original,
            Mode.SelectedIndex switch
            {
                1 => "inbound",
                2 => "outbound",
                _ => "intrasim",
            },
            center,
            ReadGeometry(Radius, _original.Radius, fallback: 1m),
            destination,
            (uint)(PollInterval.Value ?? 1m),
            (int)(Capacity.Value ?? 1m),
            (uint)(BackflowLimit.Value ?? 0m),
            TeleportVegetables.IsChecked == true,
            TeleportCorpses.IsChecked == true,
            TeleportHeterotrophs.IsChecked == true,
            FixedPosition.IsChecked == true,
            RespectShapes.IsChecked == true,
            Path.Text?.Trim() ?? string.Empty,
            InboundPath.Text?.Trim() ?? string.Empty,
            OutboundPath.Text?.Trim() ?? string.Empty);
        Close();
    }

    // NumericUpDown uses decimals, which can round away bits of an imported Single.
    private static float ReadGeometry(NumericUpDown input, float original, decimal fallback = 0m) =>
        input.Value == (decimal)original ? original : (float)(input.Value ?? fallback);

    private void Cancel_Click(object? sender, RoutedEventArgs e) => Close();
}
