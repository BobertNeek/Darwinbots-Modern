using System.Globalization;

namespace Darwinbots.Desktop.Core;

// Offsets follow OptionsForm.SaveSett (DB2 2.48.32), including variable species tails.
internal static class LegacySettingsAdapter
{
    internal static List<string> Encode(DesktopSettings settings)
    {
        int count = settings.Species.Count;
        int global = 2 + 40 * count;
        int extra = global + 90 + 8 * count;
        int distribution = extra + 8;
        int costs = distribution + 44 * count;
        int placement = costs + 71;
        int tail = placement + 4 * count;
        int shapes = tail + 18 + 5 * count;
        int footer = shapes + 9 + 7 * settings.Obstacles.Count;
        int end = footer + 13 + 3 * count;
        var fields = Enumerable.Repeat("0", end).ToList();
        if (settings.LegacySettingsFields is { Count: > 1 } original
            && original[1] == (count - 1).ToString(CultureInfo.InvariantCulture))
        {
            // The footer moves when walls are added or removed; retain unknown fields.
            int oldCount = int.Parse(original[shapes + 8], CultureInfo.InvariantCulture);
            int oldFooter = shapes + 9 + 7 * oldCount;
            for (int i = 0; i < Math.Min(shapes + 9, original.Count); i++) fields[i] = original[i];
            for (int i = oldFooter; i < original.Count; i++)
            {
                int target = footer + i - oldFooter;
                while (fields.Count <= target) fields.Add("0");
                fields[target] = original[i];
            }
        }
        while (fields.Count < end) fields.Add("0");
        void Number(int i, double value) => fields[i] = value.ToString("R", CultureInfo.InvariantCulture);
        void Flag(int i, bool value) => fields[i] = value ? "#TRUE#" : "#FALSE#";
        void Text(int i, string value) => fields[i] = "\"" + value.Replace("\"", "\"\"", StringComparison.Ordinal) + "\"";
        Number(0, -1); Number(1, count - 1);
        for (int index = 0; index < count; index++)
        {
            var species = settings.Species[index];
            int s = 2 + 40 * index;
            Number(s, settings.WorldWidth); Number(s + 1, settings.WorldHeight);
            Text(s + 5, Path.GetDirectoryName(species.DnaPath ?? "") ?? "Robots");
            Text(s + 7, Path.GetFileName(species.DnaPath ?? species.Name));
            Number(s + 6, species.InitialCount); Flag(s + 8, species.Vegetable);
            Flag(s + 9, species.Restrictions.FixedInPlace); Number(s + 10, ToVbColor(species.Color));
            Number(s + 12, species.InitialEnergy);
            for (int point = 0; point < 4; point++)
            {
                Number(s + 27 + point * 2, Math.Round(species.Skin[point].Radius * 60));
                Number(s + 28 + point * 2, ((314 - species.Skin[point].Angle) / 2 % 629 + 629) % 629);
            }
            var profile = species.MutationProfile ?? MutationProfileUpdate.Default;
            var rates = new[] { profile.Point, profile.MinorDeletion, profile.Reversal, profile.Insertion,
                profile.Amplification, profile.MajorDeletion, profile.CopyError, profile.Delta,
                profile.Translocation, profile.Point2, profile.CopyError2 };
            for (int kind = 0; kind < rates.Length; kind++)
            {
                Number(s + 13 + kind, rates[kind].OneIn);
                Number(distribution + 44 * index + 2 + 2 * kind, rates[kind].Mean);
                Number(distribution + 44 * index + 3 + 2 * kind, rates[kind].StdDev);
            }
            Number(s + 4, profile.Point.OneIn);
            Number(distribution + 44 * index, profile.CopyErrorValueChangePercent);
            Number(distribution + 44 * index + 1, profile.PointValueChangePercent);
            Flag(global + 90 + 8 * index + 7, !species.Restrictions.DisableMutations);
            Number(placement + 4 * index, species.Placement.Left); Number(placement + 4 * index + 1, species.Placement.Right);
            Number(placement + 4 * index + 2, species.Placement.Top); Number(placement + 4 * index + 3, species.Placement.Bottom);
            int r = tail + 17 + 5 * index;
            Flag(r, species.Restrictions.DisableVision); Flag(r + 1, species.Restrictions.DisableDna);
            Flag(r + 2, species.Restrictions.DisableMovement); Flag(r + 3, species.Restrictions.DisableReproduction);
            Flag(r + 4, species.Restrictions.VirusImmune);
        }
        Text(global, ""); Number(global + 1, count); Number(global + 3, settings.WorldWidth); Number(global + 4, settings.WorldHeight);
        Number(global + 5, settings.VegetablePopulationCap); Flag(global + 9, settings.ToroidalWorld);
        var vegetation = settings.Vegetation;
        var physics = settings.Physics;
        var cost = settings.GlobalSettings.Costs;
        Number(global + 10, vegetation.MaxEnergyPerTick); Number(global + 11, vegetation.MinimumChloroplastEquivalents);
        Number(global + 13, cost.DnaStore); Number(global + 14, cost.ShotFormation);
        Number(global + 15, settings.Shots.FeedingProportion);
        Number(global + 16, settings.Shots.FeedingFixedPower);
        Flag(global + 17, settings.Shots.FeedingProportional);
        Number(global + 18, settings.Gravity[1]); Number(global + 19, physics.SurfaceGravity);
        Number(global + 20, settings.BrownianMotion); Number(global + 21, physics.MovementEfficiency);
        Text(global + 23, "null"); Text(global + 25, "null"); Text(global + 31, "null");
        Number(global + 27, settings.MutationSettings.GlobalMultiplier);
        Flag(global + 32, settings.Recording.DeadSnapshotEnabled); Flag(global + 33, settings.Recording.ExcludeRepopulatingRobots);
        Flag(global + 34, vegetation.PondMode); Number(global + 36, vegetation.LightIntensity);
        Flag(global + 37, settings.CorpseDecay.Enabled); Number(global + 38, settings.CorpseDecay.PacketSize);
        Number(global + 39, vegetation.Gradient); Flag(global + 40, vegetation.DayNightEnabled);
        Number(global + 41, vegetation.CycleLength);
        Number(global + 42, settings.CorpseDecay.Mode switch { Db2CorpseDecayMode.WasteShot => 2, Db2CorpseDecayMode.EnergyShot => 3, _ => 1 });
        Number(global + 43, settings.CorpseDecay.Period); Number(global + 44, cost.Movement);
        for (int i = 0; i < 9 && settings.LegacySettingsFields is null; i++)
        {
            Text(global + 47 + 4 * i, ""); Text(global + 48 + 4 * i, "");
            Number(global + 49 + 4 * i, 0); Text(global + 50 + 4 * i, "");
        }
        Flag(global + 7, !settings.TiesEnabled);
        Flag(global + 83, settings.WrapX); Flag(global + 84, settings.WrapY);
        Number(global + 85, vegetation.RepopulationAmount); Number(global + 86, vegetation.RepopulationCooldown);
        Flag(global + 87, physics.ZeroMomentum); Flag(global + 88, true); Number(global + 89, settings.Seed ?? 1);
        Number(extra + 1, vegetation.FeedingToBody); Number(extra + 2, physics.StaticFriction); Number(extra + 3, physics.KineticFriction);
        Flag(extra + 4, physics.PlanetEaters); Number(extra + 5, physics.PlanetEatersG); Number(extra + 6, physics.Viscosity); Number(extra + 7, physics.Density);
        var indexedCosts = new (int, double)[] { (0,cost.DnaNumber), (1,cost.DnaDereference), (2,cost.DnaBasic),
            (3,cost.DnaAdvanced), (4,cost.DnaBitwise), (5,cost.DnaCondition), (6,cost.DnaLogic), (7,cost.DnaStore),
            (8,cost.ChloroplastConstruction), (9,cost.DnaFlow), (20,cost.Movement), (21,cost.Rotation), (22,cost.TieFormation),
            (23,cost.ShotFormation), (24,cost.DnaUpkeep), (25,cost.DnaCopy), (26,cost.Venom), (27,cost.Poison),
            (28,cost.Slime), (29,cost.Shell), (30,cost.BodyUpkeep), (31,cost.AgeBase), (32,cost.AgeStart), (33,cost.AgeLinear),
            (51,cost.AgeLogarithmic), (52,cost.DynamicLowerZeroThreshold), (53,cost.DynamicTargetPopulation), (54,cost.GlobalMultiplier), (55,cost.DynamicSensitivity),
            (57,cost.DynamicUpperRangePercent ?? cost.DynamicPopulationRange), (58,cost.DynamicLowerRangePercent ?? cost.DynamicUpperZeroThreshold),
            (59,cost.DynamicLowerReinstateThreshold), (60,cost.DynamicUpperReinstateThreshold) };
        foreach (var (index, value) in indexedCosts) Number(costs + index, value);
        Flag(costs + 56, cost.DynamicMultiplier != 0); Flag(costs + 61, cost.DynamicAllRobots); Flag(costs + 62, cost.DynamicAllowNegative);
        Number(tail, physics.MaxVelocity); Number(tail + 1, settings.Waste.AlzheimerThreshold); Number(tail + 2, settings.Recording.SamplingInterval);
        Number(tail + 3, 2); Number(tail + 4, 2); Number(tail + 5, physics.Elasticity); Number(tail + 6, physics.MaxVelocity);
        Flag(tail + 7, settings.Shots.EnergyShotsDoNotDecay); Number(tail + 8, vegetation.SunUpThreshold); Flag(tail + 9, vegetation.SunUpEnabled);
        Number(tail + 10, vegetation.SunDownThreshold); Flag(tail + 11, vegetation.SunDownEnabled); Number(tail + 15, vegetation.SunThresholdMode);
        Flag(tail + 17 + 5 * count, settings.Shots.WasteShotsDoNotDecay);
        Flag(tail + 14, settings.FixedBotRadii); Flag(tail + 16, !settings.AsexualReproductionAllowed);
        Flag(shapes, settings.Obstacles.All(o => o.Opacity == 0));
        Flag(shapes + 1, false); Flag(shapes + 3, settings.Obstacles.Any(o => o.Drift[0] != 0));
        Flag(shapes + 4, settings.Obstacles.Any(o => o.Drift[1] != 0));
        Flag(shapes + 5, settings.Obstacles.All(o => o.VisionPolicy == "see_through"));
        Flag(shapes + 6, settings.Obstacles.All(o => o.ShotPolicy == "absorb"));
        Flag(shapes + 7, settings.ShapesVisible); Number(shapes + 8, settings.Obstacles.Count);
        for (int i = 0; i < settings.Obstacles.Count; i++)
        {
            var obstacle = settings.Obstacles[i];
            int o = shapes + 9 + i * 7;
            Number(o, ToVbColor(obstacle.Color));
            Number(o + 1, obstacle.Maximum[0] - obstacle.Minimum[0]);
            Number(o + 2, obstacle.Maximum[1] - obstacle.Minimum[1]);
            Number(o + 3, obstacle.Drift[0]); Number(o + 4, obstacle.Drift[1]);
            Number(o + 5, obstacle.Minimum[0]); Number(o + 6, obstacle.Minimum[1]);
        }
        if (settings.LegacySettingsFields is null)
        {
            Number(footer, 5); Number(footer + 1, 1); Number(footer + 2, 15000); Number(footer + 3, 700);
        }
        for (int i = 0; i < count; i++)
        {
            Flag(footer + 4 + i, settings.Species[i].Restrictions.DisableChloroplasts);
            Flag(footer + 9 + count + i * 2, settings.Species[i].Restrictions.KillMultibot);
            Flag(footer + 10 + count + i * 2, settings.Species[i].Restrictions.KillDisqualified);
        }
        Flag(footer + 4 + count, vegetation.WeatherEnabled);
        Flag(footer + 9 + 3 * count, !settings.RobotFixingAllowed);
        Number(footer + 10 + 3 * count, vegetation.Tides); Number(footer + 11 + 3 * count, vegetation.TidesOffset);
        return fields;
    }

    private static uint ToVbColor(uint color) => ((color & 0xff) << 16) | (color & 0xff00) | ((color >> 16) & 0xff);
    private static uint FromVbColor(int color) => 0xff000000 | ToVbColor(unchecked((uint)color));

    internal static DesktopSettings Decode(string[] lines, int count, int global)
    {
        string Text(int i, string fallback = "") => i < lines.Length ? lines[i].Trim().Trim('"') : fallback;
        double Number(int i, double fallback = 0)
        {
            if (i >= lines.Length || string.IsNullOrWhiteSpace(Text(i))) return fallback;
            return double.TryParse(Text(i), NumberStyles.Float, CultureInfo.InvariantCulture, out var value)
                && double.IsFinite(value) ? value : throw new InvalidDataException($"Legacy settings field {i} is not a finite number.");
        }
        int Integer(int i, int fallback = 0) => (int)Math.Clamp(Number(i, fallback), int.MinValue, int.MaxValue);
        bool Flag(int i, bool fallback = false)
        {
            if (i >= lines.Length || string.IsNullOrWhiteSpace(Text(i))) return fallback;
            if (bool.TryParse(Text(i).Trim('#'), out var value)) return value;
            return Number(i) != 0;
        }
        float Float(int offset, float fallback = 0) => (float)Number(global + offset, fallback);
        // Nine four-field script records precede the later species and physics fields.
        int extra = global + 90 + 8 * count;
        int distribution = extra + 8;
        int costs = distribution + 44 * count;
        int placement = costs + 71;
        int tail = placement + 4 * count;
        var species = new List<SpeciesImport>();
        int shapes = tail + 18 + 5 * count;
        int obstacleCount = Integer(shapes + 8);
        if (obstacleCount is < 0 or > 32000 || obstacleCount > 0 && shapes + 9 + 7 * obstacleCount > lines.Length)
            throw new InvalidDataException("Legacy settings obstacles are invalid or truncated.");
        var obstacles = new List<ObstacleSnapshot>();
        for (int index = 0; index < obstacleCount; index++)
        {
            int o = shapes + 9 + 7 * index;
            float x = (float)Number(o + 5), y = (float)Number(o + 6);
            obstacles.Add(new((uint)(index + 1), [x, y], [x + (float)Number(o + 1), y + (float)Number(o + 2)])
            {
                Color = Flag(shapes + 1) ? 0xff000000 : FromVbColor(Integer(o)),
                Opacity = Flag(shapes) ? 0 : 1,
                Drift = [Flag(shapes + 3) ? (float)Number(o + 3) : 0, Flag(shapes + 4) ? (float)Number(o + 4) : 0],
                VisionPolicy = Flag(shapes + 5) ? "see_through" : "opaque",
                ShotPolicy = Flag(shapes + 6) ? "absorb" : "pass_through",
            });
        }
        int footer = shapes + 9 + 7 * obstacleCount;
        for (var index = 0; index < count; index++)
        {
            int s = 2 + 40 * index;
            int rates = global + 90 + 8 * index;
            int distributions = distribution + 44 * index;
            MutationRateUpdate Rate(int kind) => new(Math.Max(0, Number(s + 13 + kind, 5000)),
                Number(distributions + 2 + 2 * kind, 1), Math.Max(0, Number(distributions + 3 + 2 * kind)));
            var profile = MutationProfileUpdate.Default with
            {
                Point = Rate(0), MinorDeletion = Rate(1), Reversal = Rate(2), Insertion = Rate(3),
                Amplification = Rate(4), MajorDeletion = Rate(5), CopyError = Rate(6), Delta = Rate(7),
                Translocation = Rate(8), Point2 = Rate(9), CopyError2 = Rate(10),
                PointValueChangePercent = (byte)Math.Clamp(Integer(distributions + 1, 80), 0, 100),
                CopyErrorValueChangePercent = (byte)Math.Clamp(Integer(distributions, 80), 0, 100),
            };
            int restrictions = tail + 17 + index * 5;
            var name = Text(s + 7);
            if (string.IsNullOrWhiteSpace(name)) throw new InvalidDataException($"Legacy species {index} has no DNA filename.");
            species.Add(new SpeciesImport(name, "", Flag(s + 8), FromVbColor(Integer(s + 10)),
                Integer(s + 6), Integer(s + 12, 1000), 0, Flag(s + 8), 0)
            {
                DnaPath = Path.Combine(Text(s + 5), name),
                MutationProfile = profile,
                Skin = Enumerable.Range(0, 4).Select(point => new SkinPointSnapshot(
                    (float)Number(s + 27 + point * 2) / 60, 314 - Integer(s + 28 + point * 2) * 2)).ToArray(),
                Placement = new SpeciesPlacement(SpeciesPlacementMode.Random)
                {
                    Left = (float)Number(placement + 4 * index),
                    Right = (float)Number(placement + 4 * index + 1, 1),
                    Top = (float)Number(placement + 4 * index + 2),
                    Bottom = (float)Number(placement + 4 * index + 3, 1),
                },
                Restrictions = new SpeciesRestrictionSettings
                {
                    FixedInPlace = Flag(s + 9), DisableMutations = !Flag(rates + 7, true),
                    DisableVision = Flag(restrictions), DisableDna = Flag(restrictions + 1),
                    DisableMovement = Flag(restrictions + 2), DisableReproduction = Flag(restrictions + 3),
                    VirusImmune = Flag(restrictions + 4),
                    DisableChloroplasts = Flag(footer + 4 + index),
                    KillMultibot = Flag(footer + 9 + count + index * 2),
                    KillDisqualified = Flag(footer + 10 + count + index * 2),
                },
            });
        }
        double Cost(int index) => Number(costs + index);
        var costSettings = new GlobalSimulationCostSettings
        {
            DnaNumber = Cost(0), DnaDereference = Cost(1), DnaBasic = Cost(2), DnaAdvanced = Cost(3),
            DnaBitwise = Cost(4), DnaCondition = Cost(5), DnaLogic = Cost(6), DnaStore = Cost(7),
            ChloroplastConstruction = Cost(8), DnaFlow = Cost(9), Movement = Cost(20), Rotation = Cost(21),
            TieFormation = Cost(22), ShotFormation = Cost(23), DnaUpkeep = Cost(24), DnaCopy = Cost(25),
            Venom = Cost(26), Poison = Cost(27), Slime = Cost(28), Shell = Cost(29),
            BodyUpkeep = Cost(30), AgeBase = Cost(31), AgeStart = Cost(32), AgeLinear = Cost(33),
            AgeLogarithmic = Cost(51), GlobalMultiplier = Number(costs + 54, 1),
            DynamicTargetPopulation = Integer(costs + 53), DynamicSensitivity = Number(costs + 55, 1),
            DynamicMultiplier = Flag(costs + 56) ? 1 : 0, DynamicPopulationRange = Integer(costs + 57),
            DynamicAllRobots = Flag(costs + 61, true), DynamicAllowNegative = Flag(costs + 62),
            DynamicLowerZeroThreshold = Cost(52), DynamicUpperZeroThreshold = Cost(58),
            DynamicUpperRangePercent = Cost(57), DynamicLowerRangePercent = Cost(58),
            DynamicLowerReinstateThreshold = Cost(59), DynamicUpperReinstateThreshold = Cost(60),
        };
        var vegetation = Db2VegetationOptions.Default with
        {
            MaxEnergyPerTick = Integer(global + 10, 100), MinimumChloroplastEquivalents = Integer(global + 11),
            PondMode = Flag(global + 34), LightIntensity = Float(36, 100), Gradient = Float(39, 1.02f),
            DayNightEnabled = Flag(global + 40), CycleLength = (ulong)Math.Max(1, Number(global + 41, 10000)),
            RepopulationAmount = Integer(global + 85, 10), RepopulationCooldown = (ulong)Math.Max(0, Number(global + 86, 10)),
            FeedingToBody = (float)Number(extra + 1, 0.75), SunUpThreshold = Integer(tail + 8, 500000),
            SunUpEnabled = Flag(tail + 9), SunDownThreshold = Integer(tail + 10, 1000000),
            SunDownEnabled = Flag(tail + 11), SunThresholdMode = Integer(tail + 15),
            WeatherEnabled = Flag(footer + 4 + count),
            Tides = (uint)Math.Max(0, Number(footer + 10 + 3 * count)),
            TidesOffset = Integer(footer + 11 + 3 * count),
        };
        return new DesktopSettings
        {
            OrganismCapacity = 32000, WorldWidth = Float(3), WorldHeight = Float(4),
            Seed = (ulong)Math.Max(0, Number(global + 89, 1)), LegacySettingsFields = lines.TakeWhile(line => !line.StartsWith("#DARWINBOTS_MODERN_SETTINGS", StringComparison.Ordinal)).ToArray(),
            MutationSettings = MutationSettingsUpdate.Default with { GlobalMultiplier = Float(27, 1) },
            VegetablePopulationCap = Math.Max(0, Integer(global + 5)), SunlightEnergy = vegetation.MaxEnergyPerTick,
            Gravity = [0, Float(18)], BrownianMotion = Float(20), Species = species,
            FixedBotRadii = Flag(tail + 14), TiesEnabled = !Flag(global + 7),
            AsexualReproductionAllowed = !Flag(tail + 16), RobotFixingAllowed = !Flag(footer + 9 + 3 * count),
            Obstacles = obstacles, ShapesVisible = Flag(shapes + 7),
            ToroidalWorld = Flag(global + 9), WrapX = Flag(global + 83, Flag(global + 9)),
            WrapY = Flag(global + 84, Flag(global + 9)), Vegetation = vegetation,
            Physics = Db2PhysicsOptions.Default with
            {
                SurfaceGravity = Float(19), MaxVelocity = (float)Number(tail + 6, 60),
                MovementEfficiency = Float(21, 0.66f), StaticFriction = (float)Number(extra + 2),
                KineticFriction = (float)Number(extra + 3), PlanetEaters = Flag(extra + 4),
                PlanetEatersG = (float)Number(extra + 5), Viscosity = Number(extra + 6), Density = Number(extra + 7),
                Elasticity = (float)Number(tail + 5), ZeroMomentum = Flag(global + 87),
            },
            Waste = new(Integer(tail + 1, 400), 32000),
            Recording = new(DeadSnapshotEnabled: Flag(global + 32), ExcludeRepopulatingRobots: Flag(global + 33), SamplingInterval: Math.Max(1, Integer(tail + 2, 200))),
            Shots = Db2ShotOptions.Default with { EnergyShotsDoNotDecay = Flag(tail + 7), WasteShotsDoNotDecay = Flag(tail + 17 + 5 * count),
                FeedingProportional = Flag(global + 17, true), FeedingFixedPower = Integer(global + 16, 200), FeedingProportion = Float(15, 1) },
            GlobalSettings = new() { Costs = costSettings },
        };
    }
}
