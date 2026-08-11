using System.Text;
using Darwinbots.Desktop.Core;

namespace Darwinbots.Desktop.Services;

public static class SetupStorageService
{
    public static byte[] EncodeSettings(string path, DesktopSettings settings)
    {
        ArgumentException.ThrowIfNullOrWhiteSpace(path);
        ArgumentNullException.ThrowIfNull(settings);

        return IsModernJson(path)
            ? Encoding.UTF8.GetBytes(settings.ToJson())
            : Db2SettingsCodec.Encode(settings);
    }

    public static DesktopSettings DecodeSettings(string path, ReadOnlyMemory<byte> bytes)
    {
        ArgumentException.ThrowIfNullOrWhiteSpace(path);

        return IsModernJson(path)
            ? DesktopSettings.FromJson(Encoding.UTF8.GetString(bytes.Span))
            : Db2SettingsCodec.Decode(bytes.Span);
    }

    private static bool IsModernJson(string path) =>
        Path.GetExtension(path).Equals(".json", StringComparison.OrdinalIgnoreCase);
}
