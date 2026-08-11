using System.Globalization;
using Darwinbots.Desktop.Core;
using Xunit;

namespace Darwinbots.Desktop.Core.Tests;

public sealed class UiPreferencesTests
{
    [Theory]
    [InlineData("de-DE", "1,25", 1.25)]
    [InlineData("de-DE", "1.234", 1234)]
    [InlineData("de-DE", "1.234,5", 1234.5)]
    [InlineData("en-US", "1,234.5", 1234.5)]
    public void LocaleNumericParserAcceptsCultureDecimalAndGrouping(string cultureName, string text, double expected)
    {
        var culture = CultureInfo.GetCultureInfo(cultureName);

        Assert.True(UiResources.TryParseDecimal(text, culture, out var actual));
        Assert.Equal((decimal)expected, actual);
    }

    [Fact]
    public void UiResourcesTranslateInterfaceLabelForSelectedCulture()
    {
        Assert.Equal("OBERFLÄCHE", UiResources.Get("interface", CultureInfo.GetCultureInfo("de-DE")));
    }
}
