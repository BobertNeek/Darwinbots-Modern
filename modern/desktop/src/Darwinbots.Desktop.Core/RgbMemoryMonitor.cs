namespace Darwinbots.Desktop.Core;

public sealed record RgbMemoryChannel(ushort Address, int Floor, int Ceiling)
{
    public void Validate()
    {
        if (Address is < 1 or > 999)
            throw new ArgumentOutOfRangeException(nameof(Address));
        if (Floor is < -32_000 or > 32_000)
            throw new ArgumentOutOfRangeException(nameof(Floor));
        if (Ceiling is < -32_000 or > 32_000)
            throw new ArgumentOutOfRangeException(nameof(Ceiling));
        if (Floor >= Ceiling)
            throw new ArgumentException("RGB monitor floors must be below their ceilings.");
    }
}

public sealed record RgbMemoryMonitorSettings(
    RgbMemoryChannel Red,
    RgbMemoryChannel Green,
    RgbMemoryChannel Blue)
{
    public static RgbMemoryMonitorSettings Default { get; } =
        new(new(310, 0, 32_000), new(311, 0, 32_000), new(312, 0, 32_000));

    public static IReadOnlyList<RgbMemoryMonitorPreset> Presets { get; } =
    [
        new("ENERGY / BODY / FDBODY", new(new(310, 0, 32_000), new(311, 0, 32_000), new(312, 0, 32_000))),
        new("WASTE / SHELL / SLIME", new(new(828, 0, 32_000), new(823, 0, 32_000), new(821, 0, 32_000))),
        new("EYE DIRECTION / WIDTH / VALUE", new(new(521, -1_256, 1_256), new(531, -1_256, 1_256), new(501, 0, 32_000))),
    ];

    public void Validate()
    {
        Red.Validate();
        Green.Validate();
        Blue.Validate();
    }

    public static RgbMemoryMonitorSettings Preset(string name) =>
        Presets.FirstOrDefault(value => value.Name.Equals(name, StringComparison.OrdinalIgnoreCase))?.Settings
        ?? throw new ArgumentException($"Unknown RGB monitor preset '{name}'.", nameof(name));
}

public sealed record RgbMemoryMonitorPreset(string Name, RgbMemoryMonitorSettings Settings);

public readonly record struct RgbColor(byte Red, byte Green, byte Blue)
{
    public uint ToArgb() => 0xff000000u | (uint)(Red << 16) | (uint)(Green << 8) | Blue;
}

public static class RgbMemoryMonitor
{
    public static RgbColor ColorFor(OrganismSnapshot organism, RgbMemoryMonitorSettings settings)
    {
        ArgumentNullException.ThrowIfNull(organism);
        ArgumentNullException.ThrowIfNull(settings);
        settings.Validate();
        var memory = organism.Memory.ToDictionary(cell => cell.Address, cell => cell.Value);
        return new(
            Normalize(memory.GetValueOrDefault(settings.Red.Address), settings.Red),
            Normalize(memory.GetValueOrDefault(settings.Green.Address), settings.Green),
            Normalize(memory.GetValueOrDefault(settings.Blue.Address), settings.Blue));
    }

    private static byte Normalize(int value, RgbMemoryChannel channel) =>
        (byte)Math.Clamp(
            Math.Round((value - channel.Floor) * 255d / (channel.Ceiling - channel.Floor), MidpointRounding.AwayFromZero),
            0,
            255);
}
