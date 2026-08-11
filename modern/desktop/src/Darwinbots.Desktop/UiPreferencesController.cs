using System.Globalization;
using System.Runtime.CompilerServices;
using Avalonia;
using Avalonia.Controls;
using Avalonia.Data.Converters;
using Avalonia.VisualTree;
using Darwinbots.Desktop.Core;

namespace Darwinbots.Desktop;

public static class UiPreferencesController
{
    private static readonly ConditionalWeakTable<NumericUpDown, object> HookedNumericInputs = new();

    public static void Register(Window window)
    {
        ArgumentNullException.ThrowIfNull(window);
        void Apply() => ApplyNumericCulture(window);
        DesktopUiPreferences.Changed += Apply;
        window.Opened += (_, _) => Apply();
        window.Closed += (_, _) => DesktopUiPreferences.Changed -= Apply;
        Apply();
    }

    public static void ApplyNumericCulture(Visual root)
    {
        var numberFormat = (NumberFormatInfo)DesktopUiPreferences.Culture.NumberFormat.Clone();
        var numberStyle = NumberStyles.Float | NumberStyles.AllowThousands;
        foreach (var numeric in root.GetVisualDescendants().OfType<NumericUpDown>())
        {
            numeric.NumberFormat = numberFormat;
            numeric.ParsingNumberStyle = numberStyle;
            numeric.TextConverter = LocaleNumericTextConverter.Instance;
            if (!HookedNumericInputs.TryGetValue(numeric, out _))
            {
                HookedNumericInputs.Add(numeric, new object());
                var updating = false;

                void ApplyText()
                {
                    if (updating
                        || !UiResources.TryParseDecimal(numeric.Text, DesktopUiPreferences.Culture, out var parsed))
                        return;
                    var clamped = Math.Clamp(parsed, numeric.Minimum, numeric.Maximum);
                    if (numeric.Value == clamped) return;
                    updating = true;
                    try { numeric.Value = clamped; }
                    finally { updating = false; }
                }

                numeric.PropertyChanged += (_, args) =>
                {
                    if (args.Property == NumericUpDown.TextProperty) ApplyText();
                };

                var textBox = numeric.GetVisualDescendants().OfType<TextBox>().FirstOrDefault();
                if (textBox is not null) textBox.TextChanged += (_, _) => ApplyText();
            }
        }
    }

    private sealed class LocaleNumericTextConverter : IValueConverter
    {
        public static LocaleNumericTextConverter Instance { get; } = new();

        public object? Convert(object? value, Type targetType, object? parameter, CultureInfo culture) =>
            value is string text
                ? Parse(text)
                : value is IFormattable formattable
                    ? formattable.ToString("G", DesktopUiPreferences.Culture)
                    : value;

        public object? ConvertBack(object? value, Type targetType, object? parameter, CultureInfo culture) =>
            value is string text ? Parse(text) : value;

        private static object Parse(string text) =>
            UiResources.TryParseDecimal(text, DesktopUiPreferences.Culture, out var value)
                ? value
                : AvaloniaProperty.UnsetValue;
    }
}
