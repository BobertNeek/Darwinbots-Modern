using System.Text.Json;
using System.Text.Json.Serialization;

namespace Darwinbots.Desktop.Core;

public enum GlobalSettingsTab
{
    Main,
    AdvancedMutation,
    League,
    Evolution,
}

public enum EvolutionMode
{
    SimpleSurvival,
    Zerobot,
    Eco,
}

public enum GlobalMutationOscillationMode
{
    Disabled,
    Sine,
    Sawtooth,
}

public sealed record GlobalMainSettings
{
    public bool FixedRatio { get; init; }
    public bool OldColors { get; init; }
    public bool SuppressBuoyancy { get; init; }
    public bool VideoOff { get; init; }
    public int GraphUpdatesPerTick { get; init; } = 1;
    public bool HideTray { get; init; }
    public int BodyCutoff { get; init; }
    public bool GreedyEnforcement { get; init; }
    public bool NewSeed { get; init; }
    public string MainDirectory { get; init; } = string.Empty;
    public bool SafeMode { get; init; }
    public bool FindBestEnergyWeighting { get; init; }
    public int? FindBestWeight { get; init; }
    [JsonIgnore]
    public int EffectiveFindBestWeight => FindBestWeight ?? (FindBestEnergyWeighting ? 100 : 0);
    public bool EpigeneticReset { get; init; }
    public double EpigeneticAmplification { get; init; } = 1.0;
    public bool TemporaryGeneData { get; init; }
    public bool RandomizedInternetPictures { get; init; }
}

public sealed record GlobalAdvancedMutationSettings
{
    public bool Point2Enabled { get; init; }
    public bool CopyError2Enabled { get; init; }
    public bool AmplificationEnabled { get; init; }
    public bool TranslocationEnabled { get; init; }
    public bool Delta2Enabled { get; init; }
    public ulong Delta2Interval { get; init; } = 100;
    public double Delta2RateStep { get; init; } = 1.0;
    public double Delta2DistributionStep { get; init; }
    public int Delta2PreferenceStepPercent { get; init; }
    public bool NormalizationEnabled { get; init; }
    public int NormalizationDnaLength { get; init; } = 32_000;
    public GlobalMutationOscillationMode OscillationMode { get; init; }
    public ulong OscillationPeriod { get; init; }
    public double OscillationAmplitude { get; init; }
    public bool EpigeneticReset { get; init; }
    public double EpigeneticAmplification { get; init; } = 1.0;
    public ulong EpigeneticOverloadThreshold { get; init; } = 32;
    public bool SaveAsTemporaryGene { get; init; }
}

public sealed record GlobalSimulationCostSettings
{
    public double GlobalMultiplier { get; init; } = 1.0;
    public double DnaNumber { get; init; }
    public double DnaDereference { get; init; }
    public double DnaBasic { get; init; }
    public double DnaAdvanced { get; init; }
    public double DnaBitwise { get; init; }
    public double DnaCondition { get; init; }
    public double DnaLogic { get; init; }
    public double DnaStore { get; init; }
    public double DnaFlow { get; init; }
    public double Movement { get; init; }
    public double Rotation { get; init; }
    public double TieFormation { get; init; }
    public double ShotFormation { get; init; }
    public double Venom { get; init; }
    public double Poison { get; init; }
    public double Slime { get; init; }
    public double Shell { get; init; }
    public double BodyUpkeep { get; init; }
    public double DnaUpkeep { get; init; }
    public double DnaCopy { get; init; }
    public double ChloroplastConstruction { get; init; }
    public double Virus { get; init; }
    public double AgeBase { get; init; }
    public double AgeStart { get; init; }
    public double AgeLinear { get; init; }
    public double AgeLogarithmic { get; init; }
    public double DynamicMultiplier { get; init; }
    public bool DynamicAllRobots { get; init; } = true;
    public int DynamicTargetPopulation { get; init; }
    public int DynamicPopulationRange { get; init; }
    public double? DynamicUpperRangePercent { get; init; }
    public double? DynamicLowerRangePercent { get; init; }
    public double DynamicSensitivity { get; init; } = 1.0;
    public bool DynamicAllowNegative { get; init; }
    public double DynamicLowerZeroThreshold { get; init; }
    public double DynamicUpperZeroThreshold { get; init; }
    public double DynamicLowerReinstateThreshold { get; init; }
    public double DynamicUpperReinstateThreshold { get; init; }
}

public sealed record GlobalLeagueSettings
{
    public bool OwnedByOfflineModes { get; init; } = true;
}

public sealed record GlobalEvolutionSettings
{
    public EvolutionMode Mode { get; init; } = EvolutionMode.SimpleSurvival;
    public string SourceBot { get; init; } = string.Empty;
    public int StageLength { get; init; } = 1;
    public int CycleLength { get; init; } = 100;
    public bool ReducePopulationOscillation { get; init; }
    public bool AutoGraphs { get; init; }
    public bool OptionalInternetStartup { get; init; }
    public bool NormalizeDnaSize { get; init; }
    public int ZerobotInitialDnaLength { get; init; } = 32_000;
    public bool RestrictMutation { get; init; }
    public bool RestrictSpeciesMixing { get; init; }
}

public sealed record GlobalSettings
{
    public GlobalMainSettings Main { get; init; } = new();
    public GlobalAdvancedMutationSettings AdvancedMutation { get; init; } = new();
    public GlobalSimulationCostSettings Costs { get; init; } = new();
    public GlobalLeagueSettings League { get; init; } = new();
    public GlobalEvolutionSettings Evolution { get; init; } = new();

    public string ToJson() => JsonSerializer.Serialize(this, SerializerOptions);

    public static GlobalSettings FromJson(string json)
    {
        ArgumentException.ThrowIfNullOrWhiteSpace(json);
        var settings = JsonSerializer.Deserialize<GlobalSettings>(json, SerializerOptions)
            ?? throw new InvalidDataException("Global settings document was empty.");
        settings.Validate();
        return settings;
    }

    private void Validate()
    {
        if (Main.EffectiveFindBestWeight is < 0 or > 200)
            throw new InvalidDataException("Find Best weighting must be between 0 and 200.");
        if (AdvancedMutation.NormalizationEnabled && AdvancedMutation.NormalizationDnaLength <= 0)
            throw new InvalidDataException("Mutation normalization DNA length must be positive.");
        if (AdvancedMutation.Delta2Enabled && AdvancedMutation.Delta2Interval == 0)
            throw new InvalidDataException("Delta2 interval must be positive when enabled.");
        if (Evolution.StageLength <= 0 || Evolution.CycleLength <= 0)
            throw new InvalidDataException("Evolution stage and cycle lengths must be positive.");
        if (Evolution.ZerobotInitialDnaLength <= 0)
            throw new InvalidDataException("Zerobot initial DNA length must be positive.");
    }

    private static readonly JsonSerializerOptions SerializerOptions = new()
    {
        PropertyNamingPolicy = JsonNamingPolicy.CamelCase,
        WriteIndented = true,
        Converters = { new JsonStringEnumConverter() },
    };
}

public sealed class GlobalSettingsTabViewModel
{
    public GlobalSettingsTabViewModel(
        GlobalSettingsTab tab,
        string title,
        bool isTypedSeam = false,
        bool ownedByOfflineModes = false)
    {
        Tab = tab;
        Title = title;
        IsTypedSeam = isTypedSeam;
        OwnedByOfflineModes = ownedByOfflineModes;
    }

    public GlobalSettingsTab Tab { get; }
    public string Title { get; }
    public bool IsTypedSeam { get; }
    public bool OwnedByOfflineModes { get; }
}

public sealed class GlobalSettingsViewModel
{
    public GlobalSettingsViewModel(GlobalSettings? settings = null)
    {
        Settings = settings ?? new GlobalSettings();
        Tabs =
        [
            new(GlobalSettingsTab.Main, "Main"),
            new(GlobalSettingsTab.AdvancedMutation, "Advanced Mutation"),
            new(GlobalSettingsTab.League, "League", isTypedSeam: true, ownedByOfflineModes: Settings.League.OwnedByOfflineModes),
            new(GlobalSettingsTab.Evolution, "Evolution"),
        ];
    }

    public GlobalSettings Settings { get; }
    public IReadOnlyList<GlobalSettingsTabViewModel> Tabs { get; }
    public GlobalSettingsTabViewModel League => Tabs.Single(tab => tab.Tab == GlobalSettingsTab.League);
}
