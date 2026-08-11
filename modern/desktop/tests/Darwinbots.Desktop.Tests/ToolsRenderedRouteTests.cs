using Avalonia.Controls;
using Avalonia.Headless;
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

        await DesktopTestApplication.Session.Dispatch(async () =>
        {
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
                Assert.Contains("Rust owns collision, vision, shot, and drift behavior.", VisibleText(shape), StringComparison.Ordinal);
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

            return 0;
        }, CancellationToken.None);
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
}
