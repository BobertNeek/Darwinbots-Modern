using Darwinbots.Desktop.Core;
using Xunit;

namespace Darwinbots.Desktop.Core.Tests;

public sealed class LegacyDnaTextTests
{
    [Fact]
    public void OriginalWindows1252BytesRetainAccentsEuroAndCrLfExactly()
    {
        byte[] bytes = [0x27, 0xe9, 0x80, 0x0d, 0x0a];
        Assert.Equal("'é€\r\n", LegacyDnaText.Decode(bytes));
        Assert.Equal(bytes, LegacyDnaText.Encode("'é€\r\n"));
    }

    [Fact]
    public void BomAwareUnicodeAndStrictUtf8RemainSupportedWithoutGuessingAmbiguousAnsi()
    {
        Assert.Equal("'é\r\n", LegacyDnaText.Decode([0x27, 0xc3, 0xa9, 0x0d, 0x0a]));
        Assert.Equal("'é\r\n", LegacyDnaText.Decode([0xef, 0xbb, 0xbf, 0x27, 0xc3, 0xa9, 0x0d, 0x0a]));
        Assert.Equal("'é\r\n", LegacyDnaText.Decode([0xff, 0xfe, 0x27, 0, 0xe9, 0, 0x0d, 0, 0x0a, 0]));
        Assert.Equal("'é\r\n", LegacyDnaText.Decode([0xfe, 0xff, 0, 0x27, 0, 0xe9, 0, 0x0d, 0, 0x0a]));
        Assert.Equal("é", LegacyDnaText.Decode([0xff, 0xfe, 0, 0, 0xe9, 0, 0, 0]));
        Assert.Equal("é", LegacyDnaText.Decode([0, 0, 0xfe, 0xff, 0, 0, 0, 0xe9]));
        // C3 A9 could also mean Windows-1252 "Ã©". The documented rule prefers
        // valid UTF-8; there is no claim to infer the author's intent.
        Assert.Equal("é", LegacyDnaText.Decode([0xc3, 0xa9]));
    }

    [Fact]
    public void ExplicitUnicodeBomIsStrictAndOriginalWritesRejectUnrepresentableCharacters()
    {
        Assert.Throws<System.Text.DecoderFallbackException>(() => LegacyDnaText.Decode([0xef, 0xbb, 0xbf, 0xe9]));
        Assert.Throws<InvalidDataException>(() => LegacyDnaText.Encode("' unsupported 🙂\nstart stop"));
    }
}
