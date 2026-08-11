using System.Buffers.Binary;
using System.Globalization;
using System.Text;
using System.Text.Json;
using System.Text.Json.Serialization;

namespace Darwinbots.Desktop.Core;

public sealed record LegacySimulationSpecies(string Name, bool Vegetable, uint Color);

public sealed record LegacySimulationOrganism(
    string Dna,
    uint Species,
    bool Vegetable,
    bool Corpse,
    bool Repopulating,
    float[] Position,
    float[] Velocity,
    int Energy,
    ulong Age,
    ulong BirthCycle,
    int Body);

public sealed record LegacySimulationTeleporter(
    [property: JsonPropertyName("name")] string Name,
    [property: JsonPropertyName("entry")] float[] Entry,
    [property: JsonPropertyName("exit")] float[] Exit,
    [property: JsonPropertyName("enabled")] bool Enabled);

public sealed record LegacySimulationObstacle(
    [property: JsonPropertyName("name")] string Name,
    [property: JsonPropertyName("position")] float[] Position,
    [property: JsonPropertyName("size")] float[] Size,
    [property: JsonPropertyName("enabled")] bool Enabled);

public sealed record LegacySimulationFile(
    ushort Version,
    ulong Tick,
    float[] WorldSize,
    bool ToroidalWorld,
    IReadOnlyList<LegacySimulationSpecies> Species,
    IReadOnlyList<LegacySimulationOrganism> Organisms,
    DesktopSettings? Settings = null,
    IReadOnlyList<Db2OrganismTie>? Ties = null,
    IReadOnlyList<LegacySimulationTeleporter>? Teleporters = null,
    IReadOnlyList<LegacySimulationObstacle>? Obstacles = null);

public sealed record FileTeleporterPaths(string InboundDirectory, string OutboundDirectory)
{
    public static FileTeleporterPaths Create(string? inbound, string? outbound)
    {
        var inboundDirectory = Normalize(inbound, nameof(inbound));
        var outboundDirectory = Normalize(outbound, nameof(outbound));
        if (inboundDirectory.Length == 0 && outboundDirectory.Length == 0)
            throw new ArgumentException("A file teleporter needs an inbound or outbound directory.");
        return new(inboundDirectory, outboundDirectory);
    }

    public void ValidateExistingDirectories()
    {
        ValidateExistingDirectory(InboundDirectory, "inbound");
        ValidateExistingDirectory(OutboundDirectory, "outbound");
    }

    public static string RecordFileName(ulong tick, uint slot, uint generation) =>
        string.Create(
            CultureInfo.InvariantCulture,
            $"db2-{tick:x16}-{slot:x8}-{generation:x8}.dbo");

    private static string Normalize(string? value, string parameterName)
    {
        if (string.IsNullOrWhiteSpace(value)) return string.Empty;
        var trimmed = value.Trim().Trim('"');
        try
        {
            return Path.TrimEndingDirectorySeparator(Path.GetFullPath(trimmed));
        }
        catch (Exception error) when (error is ArgumentException or NotSupportedException or PathTooLongException)
        {
            throw new ArgumentException("The file teleporter directory is not a valid local path.", parameterName, error);
        }
    }

    private static void ValidateExistingDirectory(string directory, string label)
    {
        if (directory.Length > 0 && !Directory.Exists(directory))
            throw new DirectoryNotFoundException($"The {label} file teleporter directory does not exist: {directory}");
    }
}

public static class FileTeleporterCodec
{
    private static readonly byte[] Magic = "DB2S"u8.ToArray();
    private static readonly byte[] ExtensionMagic = "DB2X"u8.ToArray();
    private static readonly JsonSerializerOptions ExtensionJsonOptions = new()
    {
        PropertyNamingPolicy = JsonNamingPolicy.CamelCase,
        PropertyNameCaseInsensitive = true,
        WriteIndented = false,
    };
    private const ushort CurrentVersion = 2;
    private const int MaxItems = 1 << 20;
    private const int MaxStringBytes = 1 << 20;
    private const int MaxExtensionBytes = 16 << 20;

    public static byte[] Encode(LegacySimulationFile file)
    {
        ArgumentNullException.ThrowIfNull(file);
        Validate(file);

        using var stream = new MemoryStream();
        stream.Write(Magic);
        WriteUInt16(stream, file.Version);
        WriteUInt64(stream, file.Tick);
        WriteSingle(stream, file.WorldSize[0]);
        WriteSingle(stream, file.WorldSize[1]);
        WriteBoolean(stream, file.ToroidalWorld);
        WriteCount(stream, file.Species.Count, "legacy species");
        foreach (var species in file.Species)
        {
            WriteString(stream, species.Name, "legacy species name");
            WriteBoolean(stream, species.Vegetable);
            WriteUInt32(stream, species.Color);
        }

        WriteCount(stream, file.Organisms.Count, "legacy organisms");
        foreach (var organism in file.Organisms)
        {
            WriteString(stream, organism.Dna, "legacy organism DNA");
            WriteUInt32(stream, organism.Species);
            WriteBoolean(stream, organism.Vegetable);
            if (file.Version >= 2)
            {
                WriteBoolean(stream, organism.Corpse);
                WriteBoolean(stream, organism.Repopulating);
            }
            WriteSingle(stream, organism.Position[0]);
            WriteSingle(stream, organism.Position[1]);
            WriteSingle(stream, organism.Velocity[0]);
            WriteSingle(stream, organism.Velocity[1]);
            WriteInt32(stream, organism.Energy);
            WriteUInt64(stream, organism.Age);
            WriteUInt64(stream, organism.BirthCycle);
            WriteInt32(stream, organism.Body);
        }

        if (file.Settings is not null || file.Ties is not null || file.Teleporters is not null || file.Obstacles is not null)
        {
            var extension = new LegacySimulationExtension(
                (file.Settings ?? new DesktopSettings
                {
                    WorldWidth = file.WorldSize[0],
                    WorldHeight = file.WorldSize[1],
                    ToroidalWorld = file.ToroidalWorld,
                    WrapX = file.ToroidalWorld,
                    WrapY = file.ToroidalWorld,
                }).ToJson(),
                file.Ties ?? [],
                file.Teleporters ?? [],
                file.Obstacles ?? []);
            var payload = JsonSerializer.SerializeToUtf8Bytes(extension, ExtensionJsonOptions);
            WriteBytes(stream, ExtensionMagic);
            WriteUInt32(stream, checked((uint)payload.Length));
            stream.Write(payload);
        }
        return stream.ToArray();
    }

    public static LegacySimulationFile Decode(ReadOnlySpan<byte> bytes)
    {
        var reader = new Reader(bytes);
        if (!reader.ReadBytes(4).SequenceEqual(Magic))
            throw new InvalidDataException("DB2 simulation header is invalid.");

        var version = reader.ReadUInt16("simulation version");
        if (version == 0 || version > CurrentVersion)
            throw new InvalidDataException($"Unsupported DB2 simulation version {version}.");

        var tick = reader.ReadUInt64("simulation tick");
        var worldSize = new[] { reader.ReadSingle("world width"), reader.ReadSingle("world height") };
        var toroidalWorld = reader.ReadBoolean("toroidal world");
        var species = new List<LegacySimulationSpecies>(reader.ReadCount("legacy species"));
        foreach (var _ in Enumerable.Range(0, species.Capacity))
        {
            species.Add(new LegacySimulationSpecies(
                reader.ReadString("legacy species name"),
                reader.ReadBoolean("legacy species vegetable flag"),
                reader.ReadUInt32("legacy species color")));
        }

        var organisms = new List<LegacySimulationOrganism>(reader.ReadCount("legacy organisms"));
        foreach (var _ in Enumerable.Range(0, organisms.Capacity))
        {
            organisms.Add(new LegacySimulationOrganism(
                reader.ReadString("legacy organism DNA"),
                reader.ReadUInt32("legacy organism species"),
                reader.ReadBoolean("legacy organism vegetable flag"),
                version >= 2 && reader.ReadBoolean("legacy organism corpse flag"),
                version >= 2 && reader.ReadBoolean("legacy organism repopulating flag"),
                [reader.ReadSingle("organism position x"), reader.ReadSingle("organism position y")],
                [reader.ReadSingle("organism velocity x"), reader.ReadSingle("organism velocity y")],
                reader.ReadInt32("organism energy"),
                reader.ReadUInt64("organism age"),
                reader.ReadUInt64("organism birth cycle"),
                reader.ReadInt32("organism body")));
        }

        DesktopSettings? settings = null;
        IReadOnlyList<Db2OrganismTie>? ties = null;
        IReadOnlyList<LegacySimulationTeleporter>? teleporters = null;
        IReadOnlyList<LegacySimulationObstacle>? obstacles = null;
        if (!reader.IsEmpty)
        {
            if (!reader.ReadBytes(4).SequenceEqual(ExtensionMagic))
                throw new InvalidDataException("DB2 simulation has trailing bytes.");
            var extensionLength = reader.ReadUInt32("simulation extension length");
            if (extensionLength > MaxExtensionBytes)
                throw new InvalidDataException("DB2 simulation extension is too large.");
            var extensionBytes = reader.ReadBytes(checked((int)extensionLength));
            LegacySimulationExtension extension;
            try
            {
                extension = JsonSerializer.Deserialize<LegacySimulationExtension>(extensionBytes, ExtensionJsonOptions)
                    ?? throw new InvalidDataException("DB2 simulation extension is empty.");
            }
            catch (JsonException error)
            {
                throw new InvalidDataException("DB2 simulation extension is invalid.", error);
            }

            settings = DesktopSettings.FromJson(extension.SettingsJson);
            ties = extension.Ties.ToArray();
            teleporters = extension.Teleporters.ToArray();
            obstacles = extension.Obstacles.ToArray();
        }

        if (!reader.IsEmpty)
            throw new InvalidDataException("DB2 simulation has trailing bytes.");
        return new(version, tick, worldSize, toroidalWorld, species, organisms, settings, ties, teleporters, obstacles);
    }

    public static string Write(string directory, string fileName, LegacySimulationFile file)
    {
        ArgumentException.ThrowIfNullOrWhiteSpace(directory);
        ValidateFileName(fileName);
        var paths = FileTeleporterPaths.Create(directory, directory);
        if (!Directory.Exists(paths.OutboundDirectory))
            throw new DirectoryNotFoundException($"File teleporter directory does not exist: {paths.OutboundDirectory}");
        var path = Path.Combine(paths.OutboundDirectory, fileName);
        File.WriteAllBytes(path, Encode(file));
        return path;
    }

    public static LegacySimulationFile Read(string path)
    {
        ArgumentException.ThrowIfNullOrWhiteSpace(path);
        var fullPath = Path.GetFullPath(path);
        ValidateFileName(Path.GetFileName(fullPath));
        return Decode(File.ReadAllBytes(fullPath));
    }

    private static void Validate(LegacySimulationFile file)
    {
        if (file.Version == 0 || file.Version > CurrentVersion)
            throw new InvalidDataException($"Unsupported DB2 simulation version {file.Version}.");
        if (file.WorldSize is not { Length: 2 } || file.WorldSize.Any(value => !float.IsFinite(value) || value <= 0))
            throw new InvalidDataException("DB2 simulation world size is invalid.");
        if (file.Species.Count > MaxItems || file.Organisms.Count > MaxItems)
            throw new InvalidDataException("DB2 simulation contains too many records.");
        if (file.Ties is { Count: > MaxItems } || file.Teleporters is { Count: > MaxItems } || file.Obstacles is { Count: > MaxItems })
            throw new InvalidDataException("DB2 simulation extension contains too many records.");
        foreach (var species in file.Species)
            ValidateString(species.Name, "legacy species name");
        foreach (var organism in file.Organisms)
        {
            ValidateString(organism.Dna, "legacy organism DNA");
            if (organism.Position is not { Length: 2 } || organism.Position.Any(value => !float.IsFinite(value)))
                throw new InvalidDataException("DB2 organism position is invalid.");
            if (organism.Velocity is not { Length: 2 } || organism.Velocity.Any(value => !float.IsFinite(value)))
                throw new InvalidDataException("DB2 organism velocity is invalid.");
        }
        foreach (var tie in file.Ties ?? [])
        {
            if (!float.IsFinite(tie.RestLength) || tie.RestLength < 0)
                throw new InvalidDataException("DB2 organism tie rest length is invalid.");
        }
        foreach (var teleporter in file.Teleporters ?? [])
        {
            ValidateString(teleporter.Name, "legacy teleporter name");
            ValidateCoordinates(teleporter.Entry, "legacy teleporter entry");
            ValidateCoordinates(teleporter.Exit, "legacy teleporter exit");
        }
        foreach (var obstacle in file.Obstacles ?? [])
        {
            ValidateString(obstacle.Name, "legacy obstacle name");
            ValidateCoordinates(obstacle.Position, "legacy obstacle position");
            ValidateCoordinates(obstacle.Size, "legacy obstacle size");
        }
    }

    private static void ValidateCoordinates(float[] coordinates, string label)
    {
        if (coordinates is not { Length: 2 } || coordinates.Any(value => !float.IsFinite(value)))
            throw new InvalidDataException($"DB2 {label} is invalid.");
    }

    private static void ValidateString(string value, string label)
    {
        ArgumentNullException.ThrowIfNull(value);
        if (Encoding.UTF8.GetByteCount(value) > MaxStringBytes)
            throw new InvalidDataException($"{label} is too long.");
    }

    private static void ValidateFileName(string? fileName)
    {
        if (string.IsNullOrWhiteSpace(fileName)
            || !string.Equals(Path.GetExtension(fileName), ".dbo", StringComparison.OrdinalIgnoreCase)
            || Path.GetFileName(fileName) != fileName
            || fileName.IndexOfAny(Path.GetInvalidFileNameChars()) >= 0)
        {
            throw new ArgumentException("File teleporter records must be local .dbo files.", nameof(fileName));
        }
    }

    private static void WriteCount(Stream stream, int count, string label)
    {
        if (count < 0 || count > MaxItems) throw new InvalidDataException($"{label} count is invalid.");
        WriteUInt32(stream, (uint)count);
    }

    private static void WriteString(Stream stream, string value, string label)
    {
        ValidateString(value, label);
        var bytes = Encoding.UTF8.GetBytes(value);
        WriteUInt32(stream, (uint)bytes.Length);
        stream.Write(bytes);
    }

    private static void WriteBoolean(Stream stream, bool value) => stream.WriteByte(value ? (byte)1 : (byte)0);

    private static void WriteBytes(Stream stream, ReadOnlySpan<byte> value) => stream.Write(value);

    private static void WriteUInt16(Stream stream, ushort value)
    {
        Span<byte> buffer = stackalloc byte[2];
        BinaryPrimitives.WriteUInt16LittleEndian(buffer, value);
        stream.Write(buffer);
    }

    private static void WriteUInt32(Stream stream, uint value)
    {
        Span<byte> buffer = stackalloc byte[4];
        BinaryPrimitives.WriteUInt32LittleEndian(buffer, value);
        stream.Write(buffer);
    }

    private static void WriteUInt64(Stream stream, ulong value)
    {
        Span<byte> buffer = stackalloc byte[8];
        BinaryPrimitives.WriteUInt64LittleEndian(buffer, value);
        stream.Write(buffer);
    }

    private static void WriteInt32(Stream stream, int value)
    {
        Span<byte> buffer = stackalloc byte[4];
        BinaryPrimitives.WriteInt32LittleEndian(buffer, value);
        stream.Write(buffer);
    }

    private static void WriteSingle(Stream stream, float value) => WriteInt32(stream, BitConverter.SingleToInt32Bits(value));

    private ref struct Reader(ReadOnlySpan<byte> bytes)
    {
        private ReadOnlySpan<byte> _bytes = bytes;
        private int _offset;

        public bool IsEmpty => _offset == _bytes.Length;

        public ReadOnlySpan<byte> ReadBytes(int count)
        {
            Ensure(count, "bytes");
            var result = _bytes.Slice(_offset, count);
            _offset += count;
            return result;
        }

        public ushort ReadUInt16(string label)
        {
            var value = BinaryPrimitives.ReadUInt16LittleEndian(ReadBytes(2));
            return value;
        }

        public uint ReadUInt32(string label) => BinaryPrimitives.ReadUInt32LittleEndian(ReadBytes(4));

        public ulong ReadUInt64(string label) => BinaryPrimitives.ReadUInt64LittleEndian(ReadBytes(8));

        public int ReadInt32(string label) => BinaryPrimitives.ReadInt32LittleEndian(ReadBytes(4));

        public float ReadSingle(string label) => BitConverter.Int32BitsToSingle(ReadInt32(label));

        public bool ReadBoolean(string label)
        {
            var value = ReadBytes(1)[0];
            if (value > 1) throw new InvalidDataException($"DB2 {label} is invalid.");
            return value == 1;
        }

        public int ReadCount(string label)
        {
            var value = ReadUInt32(label);
            if (value > MaxItems) throw new InvalidDataException($"DB2 {label} count is invalid.");
            return (int)value;
        }

        public string ReadString(string label)
        {
            var length = ReadUInt32($"{label} length");
            if (length > MaxStringBytes) throw new InvalidDataException($"DB2 {label} is too long.");
            var bytes = ReadBytes(checked((int)length));
            try
            {
                return new UTF8Encoding(false, true).GetString(bytes);
            }
            catch (DecoderFallbackException error)
            {
                throw new InvalidDataException($"DB2 {label} is not valid UTF-8.", error);
            }
        }

        private void Ensure(int count, string label)
        {
            if (count < 0 || _bytes.Length - _offset < count)
                throw new InvalidDataException($"DB2 simulation {label} is truncated.");
        }
    }

    private sealed record LegacySimulationExtension(
        [property: JsonPropertyName("settings")] string SettingsJson,
        [property: JsonPropertyName("ties")] IReadOnlyList<Db2OrganismTie> Ties,
        [property: JsonPropertyName("teleporters")] IReadOnlyList<LegacySimulationTeleporter> Teleporters,
        [property: JsonPropertyName("obstacles")] IReadOnlyList<LegacySimulationObstacle> Obstacles);
}
