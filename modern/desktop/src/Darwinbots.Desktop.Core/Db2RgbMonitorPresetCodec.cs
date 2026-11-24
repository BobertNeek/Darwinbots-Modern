using System.Buffers.Binary;

namespace Darwinbots.Desktop.Core;

public static class Db2RgbMonitorPresetCodec
{
    // VB6 writes addresses, floors and ceilings as three groups of signed Integers in RGB order.
    private const int GroupSize = 3 * sizeof(short);
    public const int ByteCount = 3 * GroupSize;

    public static RgbMemoryMonitorSettings Decode(ReadOnlySpan<byte> bytes)
    {
        if (bytes.Length < ByteCount)
            throw new InvalidDataException("The RGB monitor preset is truncated.");

        var settings = new RgbMemoryMonitorSettings(
            ReadChannel(bytes, 0), ReadChannel(bytes, sizeof(short)), ReadChannel(bytes, 2 * sizeof(short)));
        try { settings.Validate(); }
        catch (ArgumentException error)
        {
            throw new InvalidDataException("The RGB monitor preset contains invalid channel values.", error);
        }
        return settings;
    }

    public static byte[] Encode(RgbMemoryMonitorSettings settings)
    {
        ArgumentNullException.ThrowIfNull(settings);
        settings.Validate();

        var bytes = new byte[ByteCount];
        WriteChannel(bytes, 0, settings.Red);
        WriteChannel(bytes, sizeof(short), settings.Green);
        WriteChannel(bytes, 2 * sizeof(short), settings.Blue);
        return bytes;
    }

    private static RgbMemoryChannel ReadChannel(ReadOnlySpan<byte> bytes, int offset)
    {
        var address = BinaryPrimitives.ReadInt16LittleEndian(bytes[offset..]);
        if (address < 0)
            throw new InvalidDataException("RGB monitor addresses must be positive.");
        return new RgbMemoryChannel((ushort)address,
            BinaryPrimitives.ReadInt16LittleEndian(bytes[(offset + GroupSize)..]),
            BinaryPrimitives.ReadInt16LittleEndian(bytes[(offset + 2 * GroupSize)..]));
    }

    private static void WriteChannel(Span<byte> bytes, int offset, RgbMemoryChannel channel)
    {
        BinaryPrimitives.WriteInt16LittleEndian(bytes[offset..], (short)channel.Address);
        BinaryPrimitives.WriteInt16LittleEndian(bytes[(offset + GroupSize)..], (short)channel.Floor);
        BinaryPrimitives.WriteInt16LittleEndian(bytes[(offset + 2 * GroupSize)..], (short)channel.Ceiling);
    }
}
