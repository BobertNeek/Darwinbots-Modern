using System.Text;

namespace Darwinbots.Desktop.Core;

/// <summary>
/// Original DNA uses Windows-1252. Imports prefer an explicit Unicode BOM, then
/// strict UTF-8, and fall back to Windows-1252 only for invalid UTF-8. A BOM-less
/// byte sequence valid in both encodings is interpreted as UTF-8, not detected.
/// Line endings and source text are retained without normalization.
/// </summary>
public static class LegacyDnaText
{
    private const string C1 = "€\u0081‚ƒ„…†‡ˆ‰Š‹Œ\u008dŽ\u008f\u0090‘’“”•–—˜™š›œ\u009džŸ";
    private static readonly UTF8Encoding StrictUtf8 = new(false, true);

    public static string Decode(ReadOnlySpan<byte> bytes)
    {
        if (bytes.StartsWith(new byte[] { 0xef, 0xbb, 0xbf })) return StrictUtf8.GetString(bytes[3..]);
        if (bytes.StartsWith(new byte[] { 0xff, 0xfe, 0x00, 0x00 })) return new UTF32Encoding(false, false, true).GetString(bytes[4..]);
        if (bytes.StartsWith(new byte[] { 0x00, 0x00, 0xfe, 0xff })) return new UTF32Encoding(true, false, true).GetString(bytes[4..]);
        if (bytes.StartsWith(new byte[] { 0xff, 0xfe })) return new UnicodeEncoding(false, false, true).GetString(bytes[2..]);
        if (bytes.StartsWith(new byte[] { 0xfe, 0xff })) return new UnicodeEncoding(true, false, true).GetString(bytes[2..]);
        try { return StrictUtf8.GetString(bytes); }
        catch (DecoderFallbackException)
        {
            var characters = new char[bytes.Length];
            for (var index = 0; index < bytes.Length; index++)
                characters[index] = bytes[index] is >= 0x80 and <= 0x9f ? C1[bytes[index] - 0x80] : (char)bytes[index];
            return new string(characters);
        }
    }

    public static byte[] Encode(string source)
    {
        ArgumentNullException.ThrowIfNull(source);
        var bytes = new byte[source.Length];
        for (var index = 0; index < source.Length; index++)
        {
            var value = source[index];
            if (value < 128 || value is >= (char)160 and <= (char)255) bytes[index] = (byte)value;
            else
            {
                var mapped = C1.IndexOf(value);
                if (mapped < 0) throw new InvalidDataException("Original Darwinbots DNA contains a character outside Windows-1252.");
                bytes[index] = (byte)(mapped + 0x80);
            }
        }
        return bytes;
    }
}
