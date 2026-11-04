using Avalonia.Controls;
using Avalonia.Headless;
using Avalonia.Interactivity;
using Avalonia.Media.Imaging;
using Avalonia.VisualTree;
using Darwinbots.Desktop.Core;
using Darwinbots.Desktop.ViewModels;
using Darwinbots.Desktop.Views;
using Xunit;

namespace Darwinbots.Desktop.Tests;

public sealed class ToolsRenderedRouteTests
{
    [Fact]
    public async Task FiveParityToolRoutesRenderTheirPlayerVisibleControls()
    {
        var focus = new OrganismKey(7, 3);
        var organism = new OrganismSnapshot(
            focus.Slot,
            focus.Generation,
            [100f, 200f],
            [1f, -1f],
            875,
            19,
            Species: 1)
        {
            NewMutations = 2,
            TotalMutations = 9,
            LastMutationKind = "POINT",
            LastMutationCycle = 11,
            MutationLog = ["POINT @ 11", "DELTA @ 8"],
            Memory = [new MemoryCellSnapshot(310, 875)],
        };
        var snapshot = new EngineSnapshot(12, 1, "CPU", [organism])
        {
            Species =
            [
                new SpeciesSnapshot("Unassigned", false, 0xff858982, 0, false),
                new SpeciesSnapshot("Audit Bot", false, 0xff239ac0, 0, false),
            ],
        };

        var receipt = await DesktopTestApplication.Session.Dispatch(async () =>
        {
            byte[] shapeReceipt = [];
            var shape = new ShapeEditorWindow(new ObstacleSnapshot(17, [2f, 3f], [40f, 50f]));
            try
            {
                Show(shape, 430, 650);
                AssertRendered(shape, 390, 600);
                Assert.Equal(2m, Named<NumericUpDown>(shape, "MinimumX").Value);
                Assert.Equal(3m, Named<NumericUpDown>(shape, "MinimumY").Value);
                Assert.Equal(40m, Named<NumericUpDown>(shape, "MaximumX").Value);
                Assert.Equal(50m, Named<NumericUpDown>(shape, "MaximumY").Value);
                Assert.Contains("SHAPE PROPERTIES", VisibleText(shape), StringComparison.Ordinal);
                Assert.Contains("Edit shape movement, appearance and how bots see it.", VisibleText(shape), StringComparison.Ordinal);

                using var frame = shape.CaptureRenderedFrame();
                Assert.NotNull(frame);
                Assert.True(frame.PixelSize.Width >= 390, $"Rendered width was {frame.PixelSize.Width}.");
                Assert.True(frame.PixelSize.Height >= 600, $"Rendered height was {frame.PixelSize.Height}.");
                using var png = new MemoryStream();
                frame.Save(png, new PngBitmapEncoderOptions());
                var bytes = png.ToArray();
                Assert.True(bytes.Length > 1_000, $"Rendered PNG was only {bytes.Length} bytes.");
                Assert.Equal(new byte[] { 137, 80, 78, 71, 13, 10, 26, 10 }, bytes.Take(8));
                shapeReceipt = bytes;

                Named<NumericUpDown>(shape, "MinimumX").Value = -5m;
                Named<NumericUpDown>(shape, "MinimumY").Value = -6m;
                Named<NumericUpDown>(shape, "MaximumX").Value = 70m;
                Named<NumericUpDown>(shape, "MaximumY").Value = 80m;
                Named<TextBox>(shape, "ColorText").Text = "#FF010203";
                Named<NumericUpDown>(shape, "OpacityValue").Value = 0.75m;
                Named<NumericUpDown>(shape, "DriftX").Value = 1.5m;
                Named<NumericUpDown>(shape, "DriftY").Value = -2.5m;
                Named<ComboBox>(shape, "VisionPolicy").SelectedIndex = 1;
                Named<ComboBox>(shape, "ShotPolicy").SelectedIndex = 1;
                shape.GetVisualDescendants().OfType<Button>()
                    .Single(button => button.Content?.ToString() == "APPLY")
                    .RaiseEvent(new RoutedEventArgs(Button.ClickEvent));

                var edited = Assert.IsType<ObstacleSnapshot>(shape.Result);
                Assert.Equal(17u, edited.Id);
                Assert.Equal([-5f, -6f], edited.Minimum);
                Assert.Equal([70f, 80f], edited.Maximum);
                Assert.Equal(0xFF010203u, edited.Color);
                Assert.Equal(0.75f, edited.Opacity);
                Assert.Equal([1.5f, -2.5f], edited.Drift);
                Assert.Equal("see_through", edited.VisionPolicy);
                Assert.Equal("pass_through", edited.ShotPolicy);
            }
            finally
            {
                shape.Close();
            }

            var main = new MainWindow(
                [],
                new WorldSetupOptions
                {
                    TicksPerUpdate = 1,
                    Species = [new SpeciesImport("Rendered Species", "start\nstop", false, 0xff239ac0, 1, 1_000, 0, false, 1)],
                },
                new AuditEngineClientFactory(),
                new AuditStorageService());
            try
            {
                Show(main, 1500, 1000);
                var viewModel = Assert.IsType<MainWindowViewModel>(main.DataContext);
                Assert.True(viewModel.StatusPanels.Count >= 13);
                var mainText = main.GetVisualDescendants().OfType<TextBlock>().Select(value => value.Text).ToArray();
                Assert.All(viewModel.StatusPanels, panel => Assert.Contains(panel.Label, mainText));
                Assert.True(mainText.Count(value => viewModel.StatusPanels.Any(panel => panel.Label == value)) >= 13);
                AssertRendered(main, 900, 600);
            }
            finally
            {
                main.Close();
            }

            var energy = EnvironmentUpdate.Default with
            {
                Vegetation = EnvironmentUpdate.Default.Vegetation with
                {
                    DayNightEnabled = true,
                    CycleLength = 2_400,
                    WeatherEnabled = true,
                    SunUpEnabled = true,
                    SunUpThreshold = 123,
                    SunDownEnabled = true,
                    SunDownThreshold = 456,
                    SunThresholdMode = 2,
                },
            };
            var settings = new AdvancedSettingsWindow(energy, snapshot);
            try
            {
                Show(settings, 900, 760);
                var tabs = Named<TabControl>(settings, "SettingsTabs");
                tabs.SelectedIndex = 1;
                AvaloniaHeadlessPlatform.ForceRenderTimerTick();
                Assert.Equal(2_400m, Named<NumericUpDown>(settings, "CycleLength").Value);
                Assert.True(Named<CheckBox>(settings, "WeatherEnabled").IsChecked);
                Assert.True(Named<CheckBox>(settings, "SunUpEnabled").IsChecked);
                Assert.True(Named<CheckBox>(settings, "SunDownEnabled").IsChecked);
                Assert.Equal(123m, Named<NumericUpDown>(settings, "SunUpThreshold").Value);
                Assert.Equal(456m, Named<NumericUpDown>(settings, "SunDownThreshold").Value);
                Assert.Equal(2, Named<ComboBox>(settings, "SunThresholdMode").SelectedIndex);
                AssertRendered(settings, 760, 620);
            }
            finally
            {
                settings.Close();
            }

            var rates = new MutationRatesWindow(MutationProfileUpdate.Default);
            try
            {
                Show(rates, 980, 720);
                Assert.Equal(11, Named<ComboBox>(rates, "ClassSelector").ItemCount);
                Assert.Equal("Point", Named<ComboBox>(rates, "ClassSelector").SelectedItem?.ToString());
                Assert.True(Named<CheckBox>(rates, "Enabled").IsChecked);
                var classDescription = Named<TextBlock>(rates, "ClassDescription").Text;
                Assert.False(string.IsNullOrWhiteSpace(classDescription));
                Assert.Contains("mutation", classDescription, StringComparison.OrdinalIgnoreCase);
                Assert.Contains("TOTAL", Named<TextBlock>(rates, "CategoryTotal").Text, StringComparison.Ordinal);
                Assert.Contains("Point", Named<TextBlock>(rates, "Summary").Text, StringComparison.Ordinal);
                AssertRendered(rates, 820, 600);
            }
            finally
            {
                rates.Close();
            }

            var robot = new RobotDetailsWindow(snapshot, focus, "start\nstop");
            try
            {
                Show(robot, 1_080, 680);
                var robotText = VisibleText(robot);
                Assert.Contains("NEW / TOTAL M.", robotText, StringComparison.Ordinal);
                Assert.Contains("2 / 9", robotText, StringComparison.Ordinal);
                Assert.Contains("LAST MUTATION", robotText, StringComparison.Ordinal);
                Assert.Contains("POINT", robotText, StringComparison.Ordinal);
                Assert.Contains("MUTATION LOG", robotText, StringComparison.Ordinal);
                AssertRendered(robot, 820, 520);
            }
            finally
            {
                robot.Close();
            }

            return shapeReceipt;
        }, CancellationToken.None);
        WriteReceipt(receipt);
    }

    private static void Show(Window window, double width, double height)
    {
        window.WindowState = WindowState.Normal;
        window.Width = width;
        window.Height = height;
        window.Show();
        AvaloniaHeadlessPlatform.ForceRenderTimerTick();
    }

    private static void AssertRendered(Window window, int minimumWidth, int minimumHeight)
    {
        AvaloniaHeadlessPlatform.ForceRenderTimerTick();
        using var frame = window.CaptureRenderedFrame();
        Assert.NotNull(frame);
        Assert.True(frame.PixelSize.Width >= minimumWidth, $"Rendered width was {frame.PixelSize.Width}.");
        Assert.True(frame.PixelSize.Height >= minimumHeight, $"Rendered height was {frame.PixelSize.Height}.");
    }

    private static string VisibleText(Control root) => string.Join(
        "\n",
        root.GetVisualDescendants().OfType<TextBlock>().Select(value => value.Text).Where(value => !string.IsNullOrWhiteSpace(value)));

    private static T Named<T>(Control root, string name) where T : Control =>
        root.FindControl<T>(name) ?? throw new Xunit.Sdk.XunitException($"Missing {typeof(T).Name} named {name}.");

    private static void WriteReceipt(byte[] bytes)
    {
        Assert.True(bytes.Length > 1_000, $"Rendered PNG was only {bytes.Length} bytes.");
        Assert.Equal(new byte[] { 137, 80, 78, 71, 13, 10, 26, 10 }, bytes.Take(8));
        var receiptPath = ReceiptPath();
        Directory.CreateDirectory(Path.GetDirectoryName(receiptPath)!);
        File.WriteAllBytes(receiptPath, bytes);
        Assert.True(File.Exists(receiptPath), $"Rendered receipt was not written: {receiptPath}");
    }

    private static string ReceiptPath()
    {
        // Automated output must not overwrite manually maintained comparison images.
        var directory = Environment.GetEnvironmentVariable("DARWINBOTS_GUI_AUDIT_DIR");
        if (string.IsNullOrWhiteSpace(directory))
            directory = Path.Combine(AppContext.BaseDirectory, "artifacts", "gui-audit");
        return Path.Combine(directory, "db24832-world06-shape-editor.png");
    }
}
