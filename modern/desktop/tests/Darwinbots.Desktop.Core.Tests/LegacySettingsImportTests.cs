using System.Globalization;
using System.Text;
using Darwinbots.Desktop.Core;
using Xunit;

namespace Darwinbots.Desktop.Core.Tests;

public sealed class LegacySettingsImportTests
{
    [Theory]
    [InlineData(false, false)]
    [InlineData(true, false)]
    [InlineData(false, true)]
    [InlineData(true, true)]
    public void NativeSunOnRndExportsEitherWeatherAliasAndModernExtensionKeepsBoth(bool weatherEnabled, bool sunOnRandom)
    {
        var expected = new DesktopSettings
        {
            Vegetation = Db2VegetationOptions.Default with { WeatherEnabled = weatherEnabled, SunOnRandom = sunOnRandom },
        };
        byte[] complete = Db2SettingsCodec.Encode(expected);
        string native = Encoding.UTF8.GetString(complete).Split("#DARWINBOTS_MODERN_SETTINGS_V1#")[0];
        // Zero species/walls: global2 +90+8+71+18+9 reaches the native footer.
        int footer = 2 + 90 + 8 + 71 + 18 + 9;
        Assert.Equal(weatherEnabled || sunOnRandom ? "#TRUE#" : "#FALSE#", native.Replace("\r\n", "\n", StringComparison.Ordinal).Split('\n')[footer + 4]);

        var imported = Db2SettingsCodec.Decode(Encoding.UTF8.GetBytes(native));
        Assert.Equal(weatherEnabled || sunOnRandom, imported.Vegetation.WeatherEnabled || imported.Vegetation.SunOnRandom);
        string reexported = Encoding.UTF8.GetString(Db2SettingsCodec.Encode(imported)).Split("#DARWINBOTS_MODERN_SETTINGS_V1#")[0];
        Assert.Equal(weatherEnabled || sunOnRandom ? "#TRUE#" : "#FALSE#", reexported.Replace("\r\n", "\n", StringComparison.Ordinal).Split('\n')[footer + 4]);
        var modern = Db2SettingsCodec.Decode(complete);
        Assert.Equal(weatherEnabled, modern.Vegetation.WeatherEnabled);
        Assert.Equal(sunOnRandom, modern.Vegetation.SunOnRandom);
    }

    [Theory]
    [InlineData("Chloroplast Ecosystem.set", 1, 25)]
    [InlineData("Complex Simulation.set", 6, 300)]
    [InlineData("Large Sex Simulation.set", 2, 5000)]
    [InlineData("Ledge Island.set", 2, 100)]
    [InlineData("Simple Simulation.set", 2, 100)]
    [InlineData("Small Sex Simulation.set", 2, 25)]
    [InlineData("Walls and Waste.set", 2, 500)]
    public void ShippedSettingsRetainSpeciesAndUseThePostScriptPhysicsOffsets(string name, int count, int cap)
    {
        var path = Path.Combine(RepositoryRoot(), "Installer", "sets", name);
        var lines = File.ReadAllLines(path);
        var settings = Db2SettingsCodec.Decode(File.ReadAllBytes(path));
        int global = 2 + 40 * count;
        int extra = global + 90 + 8 * count;
        int tail = global + 169 + 56 * count;
        double Number(int i) => double.Parse(lines[i], CultureInfo.InvariantCulture);
        Assert.Equal(count, settings.Species.Count);
        Assert.Equal(32000, settings.OrganismCapacity);
        Assert.Equal(cap, settings.VegetablePopulationCap);
        Assert.Equal((float)Number(global + 18), settings.Gravity[1]);
        Assert.Equal((float)Number(global + 19), settings.Physics.SurfaceGravity);
        Assert.Equal(lines[global + 17].Contains("TRUE", StringComparison.OrdinalIgnoreCase) || lines[global + 17] == "1", settings.Shots.FeedingProportional);
        Assert.Equal((int)Number(global + 16), settings.Shots.FeedingFixedPower);
        Assert.Equal((float)Number(global + 15), settings.Shots.FeedingProportion);
        Assert.Equal((float)Number(extra + 2), settings.Physics.StaticFriction);
        Assert.Equal((float)Number(tail + 6), settings.Physics.MaxVelocity);
        Assert.Equal((int)Number(tail + 2), settings.Recording.SamplingInterval);
        Assert.Equal((ulong)Number(global + 89), settings.Seed);
        for (int i = 0; i < count; i++)
        {
            Assert.Equal((int)Number(2 + 40 * i + 6), settings.Species[i].InitialCount);
            Assert.Equal(Number(2 + 40 * i + 13), settings.Species[i].MutationProfile!.Point.OneIn);
            Assert.EndsWith(settings.Species[i].Name, settings.Species[i].DnaPath);
        }
    }

    [Theory]
    [InlineData(0d, "0", 50d)]
    [InlineData(37.5d, "37.5", 37.5d)]
    public void BareNativeSettingsApplyTheOriginalDynamicSensitivityFallback(double configured, string value, double expected)
    {
        var fixture = Path.Combine(RepositoryRoot(), "Installer", "sets", "Simple Simulation.set");
        var original = Db2SettingsCodec.Decode(File.ReadAllBytes(fixture));
        var costs = original.GlobalSettings.Costs with { DynamicSensitivity = configured };
        var settings = original with { GlobalSettings = original.GlobalSettings with { Costs = costs } };
        var encoded = Db2SettingsCodec.Encode(settings);
        var native = LegacyDnaText.Decode(encoded).Split("#DARWINBOTS_MODERN_SETTINGS_V1#")[0];
        const int costsStart = 2 + 90 + 8 + 92 * 2;
        Assert.Equal(value, native.Split("\r\n")[costsStart + 55]);

        var imported = Db2SettingsCodec.Decode(LegacyDnaText.Encode(native));
        Assert.Equal(expected, imported.GlobalSettings.Costs.DynamicSensitivity);
        Assert.Equal(configured, Db2SettingsCodec.Decode(encoded).GlobalSettings.Costs.DynamicSensitivity);
    }

    [Fact]
    public void ExportedSettingsKeepNativeFieldsReadableWithoutTheModernExtension()
    {
        var expected = new DesktopSettings
        {
            Seed = 12345, VegetablePopulationCap = 25, Gravity = [0, 3.77f],
            Physics = Db2PhysicsOptions.Default with { MaxVelocity = 180, SurfaceGravity = 2, StaticFriction = 0.6f },
            Recording = new(SamplingInterval: 17),
            Shots = Db2ShotOptions.Default with { FeedingProportional = false, FeedingFixedPower = 37, FeedingProportion = 0.25f },
            Species = [new("Hunter.txt", "start stop", false, 0xff123456, 5, 900, 0, false, 0)
            {
                DnaPath = "Robots/Hunter.txt", MutationProfile = MutationProfileUpdate.Default with { Point = new(0, 3, 1), Delta = new(25, 500, 150) },
            }],
        };
        string native = Encoding.UTF8.GetString(Db2SettingsCodec.Encode(expected)).Split("#DARWINBOTS_MODERN_SETTINGS_V1#")[0];
        var actual = Db2SettingsCodec.Decode(Encoding.UTF8.GetBytes(native));
        Assert.Equal(expected.Seed, actual.Seed);
        Assert.Equal(expected.Physics, actual.Physics);
        Assert.Equal(expected.Shots, actual.Shots);
        Assert.Equal(25, actual.VegetablePopulationCap);
        Assert.Equal(17, actual.Recording.SamplingInterval);
        Assert.Equal(5, Assert.Single(actual.Species).InitialCount);
        Assert.Equal(0, actual.Species[0].MutationProfile!.Point.OneIn);
        Assert.Equal(25, actual.Species[0].MutationProfile!.Delta.OneIn);
        Assert.Equal(expected.Species[0].Color, actual.Species[0].Color);
    }

    [Theory]
    [InlineData("-2147483648", 0x80000000UL)]
    [InlineData("-42", 0xffffffd6UL)]
    [InlineData("-1", 0xffffffffUL)]
    [InlineData("0", 0UL)]
    [InlineData("2147483647", 0x7fffffffUL)]
    public void NativeSignedLongSeedKeepsItsLow32BitsThroughImportAndExport(string nativeSeed, ulong expectedSeed)
    {
        var lines = File.ReadAllLines(Path.Combine(RepositoryRoot(), "Installer", "sets", "Simple Simulation.set"));
        int count = int.Parse(lines[1], CultureInfo.InvariantCulture) + 1;
        int seedField = 2 + 40 * count + 89;
        lines[seedField] = nativeSeed;

        var settings = Db2SettingsCodec.Decode(Encoding.UTF8.GetBytes(string.Join("\r\n", lines)));

        Assert.Equal(expectedSeed, settings.Seed);
        Assert.Equal(expectedSeed, settings.ToWorldSetup().Seed);
        string native = Encoding.UTF8.GetString(Db2SettingsCodec.Encode(settings)).Split("#DARWINBOTS_MODERN_SETTINGS_V1#")[0];
        Assert.Equal(nativeSeed, native.Replace("\r\n", "\n", StringComparison.Ordinal).Split('\n')[seedField]);
        Assert.Equal(expectedSeed, Db2SettingsCodec.Decode(Encoding.UTF8.GetBytes(native)).Seed);
    }

    [Theory]
    [InlineData(0xffffffffUL, "-1", 0xffffffffUL)]
    [InlineData(0x100000000UL, "0", 0UL)]
    [InlineData(0x1234567880000000UL, "-2147483648", 0x80000000UL)]
    [InlineData(ulong.MaxValue, "-1", 0xffffffffUL)]
    public void ModernExtensionKeepsFullSeedWhileNativeFieldUsesSignedLow32Bits(ulong seed, string nativeSeed, ulong expectedNativeSeed)
    {
        var settings = new DesktopSettings { Seed = seed };
        byte[] encoded = Db2SettingsCodec.Encode(settings);
        string native = Encoding.UTF8.GetString(encoded).Split("#DARWINBOTS_MODERN_SETTINGS_V1#")[0];

        Assert.Equal(nativeSeed, native.Replace("\r\n", "\n", StringComparison.Ordinal).Split('\n')[2 + 89]);
        Assert.Equal(expectedNativeSeed, Db2SettingsCodec.Decode(Encoding.UTF8.GetBytes(native)).Seed);
        Assert.Equal(seed, Db2SettingsCodec.Decode(encoded).Seed);
        Assert.Equal(seed, DesktopSettings.FromJson(settings.ToJson()).Seed);
    }

    [Theory]
    [InlineData("-2147483649")]
    [InlineData("2147483648")]
    public void NativeSeedOutsideSignedLongRangeIsRejected(string nativeSeed)
    {
        var lines = File.ReadAllLines(Path.Combine(RepositoryRoot(), "Installer", "sets", "Simple Simulation.set"));
        int count = int.Parse(lines[1], CultureInfo.InvariantCulture) + 1;
        lines[2 + 40 * count + 89] = nativeSeed;

        var error = Assert.Throws<InvalidDataException>(() =>
            Db2SettingsCodec.Decode(Encoding.UTF8.GetBytes(string.Join("\r\n", lines))));

        Assert.Contains("seed", error.Message);
        Assert.Contains("signed 32-bit", error.Message);
    }

    [Theory]
    [InlineData(0d, false, false, "0", "0", "0")]
    [InlineData(2d, true, true, "-1", "1", "1")]
    public void NativeCostFlagsWriteOriginalSingleValues(double dynamicMultiplier, bool allRobots, bool allowNegative,
        string expectedDynamic, string expectedAllRobots, string expectedAllowNegative)
    {
        var path = Path.Combine(RepositoryRoot(), "Installer", "sets", "Walls and Waste.set");
        var original = File.ReadAllBytes(path);
        var originalFields = LegacyDnaText.Decode(original).Replace("\r\n", "\n", StringComparison.Ordinal).Split('\n');
        // Both species contribute 40 roster fields, 8 options and 44 mutation distribution fields.
        const int costsStart = 2 + 90 + 8 + 92 * 2;
        Assert.Equal("1", originalFields[costsStart + 61]);
        var settings = Db2SettingsCodec.Decode(original);
        Assert.True(settings.GlobalSettings.Costs.DynamicAllRobots);
        var costs = settings.GlobalSettings.Costs with
        {
            DynamicMultiplier = dynamicMultiplier,
            DynamicAllRobots = allRobots,
            DynamicAllowNegative = allowNegative,
        };
        byte[] encoded = Db2SettingsCodec.Encode(settings with { GlobalSettings = settings.GlobalSettings with { Costs = costs } });
        var native = LegacyDnaText.Decode(encoded).Split("#DARWINBOTS_MODERN_SETTINGS_V1#")[0];
        var fields = native.Split("\r\n");

        Assert.Equal(expectedDynamic, fields[costsStart + 56]);
        Assert.Equal(expectedAllRobots, fields[costsStart + 61]);
        Assert.Equal(expectedAllowNegative, fields[costsStart + 62]);
        var imported = Db2SettingsCodec.Decode(LegacyDnaText.Encode(native)).GlobalSettings.Costs;
        Assert.Equal(dynamicMultiplier == 0 ? 0d : 1d, imported.DynamicMultiplier);
        Assert.Equal(allRobots, imported.DynamicAllRobots);
        Assert.Equal(allowNegative, imported.DynamicAllowNegative);
        Assert.Equal(costs, Db2SettingsCodec.Decode(encoded).GlobalSettings.Costs);
    }

    [Fact]
    public void LegacyCostFlagsStillAcceptModernBooleanTokens()
    {
        var path = Path.Combine(RepositoryRoot(), "Installer", "sets", "Simple Simulation.set");
        var fields = File.ReadAllLines(path);
        const int costsStart = 2 + 90 + 8 + 92 * 2;
        fields[costsStart + 56] = "#TRUE#";
        fields[costsStart + 61] = "#FALSE#";
        fields[costsStart + 62] = "#TRUE#";
        var imported = Db2SettingsCodec.Decode(LegacyDnaText.Encode(string.Join("\r\n", fields))).GlobalSettings.Costs;

        Assert.Equal(1d, imported.DynamicMultiplier);
        Assert.False(imported.DynamicAllRobots);
        Assert.True(imported.DynamicAllowNegative);
    }

    [Fact]
    public void NativeWallFooterMovesWhenImportedWallsAreEdited()
    {
        var settings = Db2SettingsCodec.Decode(File.ReadAllBytes(Path.Combine(RepositoryRoot(), "Installer", "sets", "Walls and Waste.set")));
        var edited = settings with
        {
            RobotFixingAllowed = false,
            Obstacles = [new(1, [25, 35], [250, 450])
            {
                Color = 0xff123456, Drift = [2, 3], VisionPolicy = "see_through", ShotPolicy = "absorb",
            }],
        };
        string native = Encoding.UTF8.GetString(Db2SettingsCodec.Encode(edited)).Split("#DARWINBOTS_MODERN_SETTINGS_V1#")[0];
        var actual = Db2SettingsCodec.Decode(Encoding.UTF8.GetBytes(native));
        var wall = Assert.Single(actual.Obstacles);
        Assert.Equal(new float[] { 25, 35 }, wall.Minimum);
        Assert.Equal(new float[] { 250, 450 }, wall.Maximum);
        Assert.Equal(0xff123456u, wall.Color);
        Assert.Equal(new float[] { 2, 3 }, wall.Drift);
        Assert.Equal("see_through", wall.VisionPolicy);
        Assert.False(actual.RobotFixingAllowed);
        Assert.Equal(settings.Vegetation.Tides, actual.Vegetation.Tides);
        Assert.Equal(settings.Species.Count, actual.Species.Count);
    }

    [Fact]
    public void OriginalWallsScaleToFieldUnitsAndKeepVelocityWhenDriftIsDisabled()
    {
        var fields = File.ReadAllLines(Path.Combine(RepositoryRoot(), "Installer", "sets", "Walls and Waste.set"));
        // Original settings have two species. Shape settings begin at field 391.
        fields[394] = "#FALSE#";
        fields[395] = "#FALSE#";
        fields[400] = "5649426"; // Stored color &H563412 while global Black is true.
        var imported = Db2SettingsCodec.Decode(LegacyDnaText.Encode(string.Join("\r\n", fields)));

        Assert.Equal(32_000f, imported.WorldWidth);
        Assert.Equal(24_000f, imported.WorldHeight);
        Assert.Equal(18, imported.Obstacles.Count);
        var first = imported.Obstacles[0];
        Assert.Equal(4005.00, first.Minimum[0], 2);
        Assert.Equal(3025.19, first.Minimum[1], 2);
        Assert.Equal(4505.00, first.Maximum[0], 2);
        Assert.Equal(14806.19, first.Maximum[1], 2);
        Assert.Equal(0xff123456u, first.Color);
        Assert.Equal([0.5954616f, 0.6160923f], first.Drift);
        Assert.True(imported.ShapeCreation!.Black);

        var exported = NativeFields(Db2SettingsCodec.Encode(imported with { Species = [] }));
        double Number(int i) => double.Parse(exported[i], CultureInfo.InvariantCulture);
        // Removing both species moves the shape settings to field 189 and the first row to 198.
        Assert.Equal("#FALSE#", exported[192]);
        Assert.Equal("#FALSE#", exported[193]);
        Assert.Equal("18", exported[197]);
        Assert.Equal(fields[400], exported[198]);
        Assert.Equal(0.015625, Number(199), 7);
        Assert.Equal(0.490875, Number(200), 7);
        Assert.Equal(0.5954616, Number(201), 7);
        Assert.Equal(0.6160923, Number(202), 7);
        Assert.Equal(0.1251562, Number(203), 7);
        Assert.Equal(0.1260494, Number(204), 7);
    }

    [Fact]
    public void EarlierModernSettingsTrailerRestoresAbsoluteShapes()
    {
        var settings = new DesktopSettings
        {
            Obstacles = [new(1, [-25, 35], [250, 450]) { Drift = [2, 3] }],
        };
        var fields = LegacyDnaText.Decode(Db2SettingsCodec.Encode(settings)).Split("\r\n");
        // Older Modern writers emitted absolute values in the native prefix and the trailer.
        fields[199] = "275";
        fields[200] = "415";
        fields[203] = "-25";
        fields[204] = "35";
        var bytes = LegacyDnaText.Encode(string.Join("\r\n", fields));
        var restored = Db2SettingsCodec.Decode(bytes);
        Assert.Equal(settings.ToJson(), restored.ToJson());
    }

    [Theory]
    [InlineData(null, -1)]
    [InlineData("0", 400)]
    public void OriginalSettingsUseLegacyDefaultsWhenTheWasteFieldIsAbsentOrZero(string? storedThreshold, int expected)
    {
        const int wasteField = 172; // The second field in SaveSett's zero-species tail.
        var imported = ImportWithTailField(wasteField, storedThreshold);

        Assert.Equal(expected, imported.Waste.AlzheimerThreshold);
    }

    [Theory]
    [InlineData(null, 40f)]
    [InlineData("0", 60f)]
    [InlineData("72.5", 72.5f)]
    public void OriginalSettingsUseTheLaterMaximumVelocityField(string? storedVelocity, float expected)
    {
        const int velocityField = 177; // The later of SaveSett's two velocity fields.
        var imported = ImportWithTailField(velocityField, storedVelocity);

        Assert.Equal(expected, imported.Physics.MaxVelocity);
    }

    [Fact]
    public void OriginalSettingsWithoutAnObstacleCountCanBeSavedAgain()
    {
        var fields = NativeFields(Db2SettingsCodec.Encode(new DesktopSettings()));
        const int obstacleCountField = 197; // The last field before shape records and the footer.
        const int obsoleteBlockedPlantsField = 8;
        fields[obsoleteBlockedPlantsField] = "17";
        var imported = Db2SettingsCodec.Decode(LegacyDnaText.Encode(string.Join("\r\n", fields[..obstacleCountField]) + "\r\n"));

        var resaved = NativeFields(Db2SettingsCodec.Encode(imported));

        Assert.Equal("17", resaved[obsoleteBlockedPlantsField]);
        Assert.Equal("0", resaved[obstacleCountField]);
        Assert.Equal("15000", resaved[obstacleCountField + 3]);
    }

    private static string[] NativeFields(byte[] bytes)
    {
        var text = LegacyDnaText.Decode(bytes);
        return text.Split("\r\n").TakeWhile(field => field != "#DARWINBOTS_MODERN_SETTINGS_V1#").ToArray();
    }

    private static DesktopSettings ImportWithTailField(int index, string? value)
    {
        var fields = NativeFields(Db2SettingsCodec.Encode(new DesktopSettings()));
        if (value is null)
            fields = fields[..index];
        else
            fields[index] = value;
        return Db2SettingsCodec.Decode(LegacyDnaText.Encode(string.Join("\r\n", fields) + "\r\n"));
    }

    internal static string RepositoryRoot()
    {
        var directory = new DirectoryInfo(AppContext.BaseDirectory);
        while (directory is not null && !File.Exists(Path.Combine(directory.FullName, "Installer", "sets", "Simple Simulation.set"))) directory = directory.Parent;
        return directory?.FullName ?? throw new DirectoryNotFoundException("Repository fixture directory was not found.");
    }
}
