using System.Reflection;
using System.Runtime.CompilerServices;
using System.Text.RegularExpressions;
using Darwinbots.Desktop.Views;
using Xunit;

[assembly: CollectionBehavior(DisableTestParallelization = true)]

namespace Darwinbots.Desktop.Tests;

public sealed class ControlSurfaceBindingTests
{
    private static readonly Regex EventBinding = new(
        "\\b(?:Click|KeyDown|SelectionChanged|IsCheckedChanged|Opened|Closed|PointerPressed|PointerMoved|PointerReleased|DoubleTapped)=\"(?<handler>[A-Za-z_][A-Za-z0-9_]*)\"",
        RegexOptions.CultureInvariant);

    [Fact]
    public void EveryDeclaredControlEventResolvesToARealHandler()
    {
        var surfaces = new[]
        {
            new Surface("MainWindow.axaml", typeof(MainWindow), 26),
            new Surface("SetupWindow.axaml", typeof(SetupWindow), 11),
            new Surface("AdvancedSettingsWindow.axaml", typeof(AdvancedSettingsWindow), 2),
            new Surface("DnaEditorWindow.axaml", typeof(DnaEditorWindow), 3),
        };

        foreach (var surface in surfaces)
        {
            var xaml = File.ReadAllText(Path.Combine(ViewsDirectory(), surface.File));
            var handlers = EventBinding.Matches(xaml)
                .Select(match => match.Groups["handler"].Value)
                .Distinct(StringComparer.Ordinal)
                .OrderBy(value => value, StringComparer.Ordinal)
                .ToArray();

            Assert.Equal(surface.ExpectedHandlers, handlers.Length);
            foreach (var handler in handlers)
            {
                var method = surface.CodeBehind.GetMethod(
                    handler,
                    BindingFlags.Instance | BindingFlags.Public | BindingFlags.NonPublic);
                Assert.True(method is not null, $"{surface.File} binds missing handler {handler}");
            }

            foreach (Match control in Regex.Matches(xaml, "<(?:Button|RadioButton)\\b(?<attributes>[^>]*)>"))
                Assert.Contains("Click=\"", control.Groups["attributes"].Value, StringComparison.Ordinal);
            foreach (Match item in Regex.Matches(xaml, "<MenuItem\\b(?<attributes>[^>]*)/>"))
                Assert.Contains("Click=\"", item.Groups["attributes"].Value, StringComparison.Ordinal);
        }
    }

    private static string ViewsDirectory([CallerFilePath] string source = "") =>
        Path.GetFullPath(Path.Combine(Path.GetDirectoryName(source)!, "..", "..", "src", "Darwinbots.Desktop", "Views"));

    private sealed record Surface(string File, Type CodeBehind, int ExpectedHandlers);
}
