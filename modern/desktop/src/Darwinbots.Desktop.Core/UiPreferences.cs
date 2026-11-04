using System.Globalization;

namespace Darwinbots.Desktop.Core;

public enum DesktopUiTheme
{
    System,
    Light,
    Dark,
}

public sealed record UiCultureOption(string Name, string DisplayName, CultureInfo Culture);

public static class UiResources
{
    public static IReadOnlyList<UiCultureOption> Cultures { get; } =
    [
        new("en-US", "English (US)", CultureInfo.GetCultureInfo("en-US")),
        new("de-DE", "Deutsch", CultureInfo.GetCultureInfo("de-DE")),
        new("it-IT", "Italiano", CultureInfo.GetCultureInfo("it-IT")),
    ];

    public static CultureInfo ResolveCulture(string? name) =>
        Cultures.FirstOrDefault(option => string.Equals(option.Name, name, StringComparison.OrdinalIgnoreCase))?.Culture
        ?? Cultures[0].Culture;

    public static string Get(string key, CultureInfo culture) =>
        key switch
        {
            "interface" => culture.TwoLetterISOLanguageName switch
            {
                "de" => "OBERFLÄCHE",
                "it" => "INTERFACCIA",
                _ => "INTERFACE",
            },
            "theme" => culture.TwoLetterISOLanguageName switch
            {
                "de" => "THEMA",
                "it" => "TEMA",
                _ => "THEME",
            },
            "language" => culture.TwoLetterISOLanguageName switch
            {
                "de" => "SPRACHE",
                "it" => "LINGUA",
                _ => "LANGUAGE",
            },
            _ => key,
        };

    public static bool TryParseDecimal(string? text, CultureInfo culture, out decimal value)
    {
        value = 0;
        if (string.IsNullOrWhiteSpace(text)) return false;
        var trimmed = text.Trim();
        var comma = trimmed.LastIndexOf(',');
        var dot = trimmed.LastIndexOf('.');

        if (comma >= 0 && dot >= 0)
        {
            var decimalSeparator = comma > dot ? ',' : '.';
            var groupingSeparator = decimalSeparator == ',' ? '.' : ',';
            var normalized = trimmed.Replace(groupingSeparator.ToString(), string.Empty, StringComparison.Ordinal)
                .Replace(decimalSeparator, '.');
            return decimal.TryParse(normalized, NumberStyles.Float, CultureInfo.InvariantCulture, out value);
        }

        var cultureDecimal = culture.NumberFormat.NumberDecimalSeparator[0];
        if (comma >= 0 || dot >= 0)
        {
            var separator = comma >= 0 ? ',' : '.';
            if (separator == cultureDecimal)
                return decimal.TryParse(trimmed, NumberStyles.Number, culture, out value);
            if (string.Equals(separator.ToString(), culture.NumberFormat.NumberGroupSeparator, StringComparison.Ordinal))
                return decimal.TryParse(trimmed, NumberStyles.Number, culture, out value);

            var alternate = trimmed.Replace(separator, '.');
            return decimal.TryParse(alternate, NumberStyles.Float, CultureInfo.InvariantCulture, out value);
        }

        return decimal.TryParse(trimmed, NumberStyles.Number, culture, out value);
    }
}

public static class DesktopUiPreferences
{
    public static DesktopUiTheme Theme { get; private set; } = DesktopUiTheme.System;
    public static CultureInfo Culture { get; private set; } = UiResources.Cultures[0].Culture;

    public static event Action? Changed;

    public static void ApplyTheme(DesktopUiTheme theme)
    {
        Theme = theme;
        Changed?.Invoke();
    }

    public static void ApplyCulture(string? cultureName)
    {
        Culture = UiResources.ResolveCulture(cultureName);
        CultureInfo.CurrentCulture = Culture;
        CultureInfo.CurrentUICulture = Culture;
        Changed?.Invoke();
    }

    public static void Reset()
    {
        Theme = DesktopUiTheme.System;
        Culture = UiResources.Cultures[0].Culture;
        CultureInfo.CurrentCulture = Culture;
        CultureInfo.CurrentUICulture = Culture;
        Changed?.Invoke();
    }
}
