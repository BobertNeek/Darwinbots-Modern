using Avalonia.Controls;
using Avalonia.Headless;
using Avalonia.Interactivity;
using Avalonia.VisualTree;
using Darwinbots.Desktop.Core;
using Darwinbots.Desktop.Views;
using Xunit;

namespace Darwinbots.Desktop.Tests;

public sealed class ShapeEditorReflectPolicyTests
{
    [Theory]
    [InlineData("absorb", 0, 2, "reflect")]
    [InlineData("pass_through", 1, 1, "pass_through")]
    [InlineData("reflect", 2, 0, "absorb")]
    public async Task EditorRestoresAndAppliesThreeDistinctShotPolicies(
        string originalPolicy, int originalIndex, int selectedIndex, string expectedPolicy)
    {
        await DesktopTestApplication.Session.Dispatch(() =>
        {
            var original = new ObstacleSnapshot(17, [2f, 3f], [40f, 50f])
            {
                ShotPolicy = originalPolicy,
                Drift = [1f, -2f],
            };
            var editor = new ShapeEditorWindow(original);
            try
            {
                editor.Show();
                AvaloniaHeadlessPlatform.ForceRenderTimerTick();
                var selector = Assert.IsType<ComboBox>(editor.FindControl<ComboBox>("ShotPolicy"));
                Assert.Equal(["Absorb", "Pass through", "Reflect"],
                    selector.Items.Cast<ComboBoxItem>().Select(item => Assert.IsType<string>(item.Content)).ToArray());
                Assert.Equal(originalIndex, selector.SelectedIndex);
                selector.SelectedIndex = selectedIndex;
                Click(editor, "APPLY");
                var edited = Assert.IsType<ObstacleSnapshot>(editor.Result);
                Assert.Equal(expectedPolicy, edited.ShotPolicy);
                Assert.Equal(original.Id, edited.Id);
                Assert.Equal(original.Minimum, edited.Minimum);
                Assert.Equal(original.Maximum, edited.Maximum);
                Assert.Equal(original.Drift, edited.Drift);
                Assert.Equal(originalPolicy, original.ShotPolicy);
            }
            finally
            {
                editor.Close();
            }
        }, CancellationToken.None);
    }

    [Fact]
    public async Task CancelLeavesAnImportedReflectShapeUnchanged()
    {
        await DesktopTestApplication.Session.Dispatch(() =>
        {
            var original = new ObstacleSnapshot(17, [2f, 3f], [40f, 50f]) { ShotPolicy = "reflect" };
            var editor = new ShapeEditorWindow(original);
            try
            {
                editor.Show();
                AvaloniaHeadlessPlatform.ForceRenderTimerTick();
                editor.FindControl<ComboBox>("ShotPolicy")!.SelectedIndex = 1;
                Click(editor, "CANCEL");
                Assert.Null(editor.Result);
                Assert.Equal("reflect", original.ShotPolicy);
            }
            finally
            {
                editor.Close();
            }
        }, CancellationToken.None);
    }

    private static void Click(Window window, string label) => window.GetVisualDescendants()
        .OfType<Button>().Single(button => button.Content?.ToString() == label)
        .RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
}
