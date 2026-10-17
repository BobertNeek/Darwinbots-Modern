using System.Text.Json;
using Darwinbots.Desktop.Core;
using Xunit;

namespace Darwinbots.Desktop.Tests;

public sealed class CommunityLeagueNativeTests
{
    private static NativeProcessCachesSnapshot EnergyCaches(int marker, double light, ulong shotsThisCycle = 17, bool deathPoffsEnabled = false, int repopulationCounter = 0) =>
        new(light, new(Enumerable.Range(0, 101).Select(value => value * 7).ToArray(), 4, marker), shotsThisCycle, deathPoffsEnabled)
        {
            RepopulationCounter = repopulationCounter,
        };

    private static void AssertProcessCaches(NativeProcessCachesSnapshot expected, NativeProcessCachesSnapshot actual)
    {
        Assert.Equal(expected.LightAvailability, actual.LightAvailability);
        Assert.Equal(expected.Energy.Ring, actual.Energy.Ring);
        Assert.Equal(expected.Energy.CurrentIndex, actual.Energy.CurrentIndex);
        Assert.Equal(expected.Energy.Displayed, actual.Energy.Displayed);
        Assert.Equal(expected.ShotsThisCycle, actual.ShotsThisCycle);
        Assert.Equal(expected.DeathPoffsEnabled, actual.DeathPoffsEnabled);
        Assert.Equal(expected.RepopulationCounter, actual.RepopulationCounter);
    }

    [Fact]
    public async Task PopulationSeedingScoresExtinctionThenAutomaticallyRunsTheClassicTournamentAndExportsStartupDna()
    {
        using var engine = new NativeEngineClient(new WorldSetupOptions
        {
            Backend = "Cpu", Seed = 42, PopulationCapacity = 16, BrownianMotion = 0,
            MutationSettings = MutationSettingsUpdate.Default with { Enabled = false },
        }, new TestClock(1125));
        engine.ImportSpecies(new("Gone.txt", "start\n1 .up store\nstop", false, 0xff123456, 1, 1000, 0, false, 0));
        engine.ImportSpecies(new("Next.txt", "start stop", false, 0xff654321, 2, 1000, 0, false, 0));
        engine.ImportSpecies(new("Plant.txt", "start stop", true, 0xff00ff00, 1, 1000, 0, false, 0));
        var species = engine.Snapshot().Species;
        var gone = (uint)species.Select((value, index) => (value, index)).Single(item => item.value.Name == "Gone.txt").index;
        var next = (uint)species.Select((value, index) => (value, index)).Single(item => item.value.Name == "Next.txt").index;
        var original = engine.ExportSpeciesTemplateDna(gone);
        engine.ConfigureLocalMode(LocalModeConfig.Default with
        {
            Workflow = LeagueWorkflow.PopulationSeeding, EntrantOrder = [gone, next],
            SeedingOpponent = next, MaximumCycles = 1, RestartOnExtinction = true,
            MinimumRounds = 1, MaximumRounds = 1,
        });
        engine.StartLocalMode();
        Assert.Equal(gone, engine.Snapshot().LocalMode.Series!.ActiveSeed);
        Assert.All(engine.Snapshot().Organisms.Where(robot => !robot.Vegetable), robot => Assert.Equal(gone, robot.Species));
        Assert.Contains(engine.Snapshot().Organisms, robot => robot.Vegetable);
        var candidates = engine.Snapshot().Organisms.Where(robot => !robot.Vegetable).ToArray();
        Assert.Equal(5, candidates.Length);
        var candidate = candidates[0];
        engine.ReplaceDna(candidate.Slot, candidate.Generation, "start 2 .up store stop");
        engine.RemoveMany(candidates.Select(robot => new OrganismKey(robot.Slot, robot.Generation)).ToArray());
        engine.Load(PopulationSeedCycle(engine.Save(), 1999));

        engine.Tick();

        var first = engine.Snapshot();
        Assert.Equal(new PopulationSeedSnapshot(gone, 0, 2000) { FileName = "0.txt" }, Assert.Single(first.LocalMode.Series!.PopulationSeeds));
        Assert.Equal(next, first.LocalMode.Series.ActiveSeed);
        Assert.Empty(first.LocalMode.Results);
        Assert.Empty(first.LocalMode.Series.ActiveMatch);
        Assert.Empty(first.LocalMode.Series.Matches);
        Assert.Empty(first.LocalMode.Series.Seeded);
        Assert.All(first.Organisms.Where(robot => !robot.Vegetable), robot => Assert.Equal(next, robot.Species));
        var saved = engine.Save();
        Assert.Equal(original, engine.ExportSpeciesTemplateDna(gone));
        Assert.Equal(saved, engine.Save());
        using var resumed = new NativeEngineClient(new WorldSetupOptions { Backend = "Cpu" }, new TestClock(1375));
        resumed.Load(saved);
        Assert.Equal(saved, resumed.Save());
        Assert.Equal(original, resumed.ExportSpeciesTemplateDna(gone));
        Assert.Equal(first.LocalMode.Series.PopulationSeeds, resumed.Snapshot().LocalMode.Series!.PopulationSeeds);
        Assert.Equal(LeagueWorkflow.PopulationSeeding, resumed.Snapshot().LocalMode.Configuration!.Workflow);
        Assert.Null(resumed.Snapshot().LocalMode.Configuration!.SeedingOpponent);
        // Jumping the public cycle does not run UpdateCounters. Carry the real
        // preceding displayed census instead of inventing it at cycle 2000.
        resumed.Load(PopulationSeedCycle(resumed.Save(), 1999, 5));

        resumed.Tick();

        var tournament = resumed.Snapshot().LocalMode;
        Assert.Equal(LocalModeStatus.Running, tournament.Status);
        Assert.Null(tournament.Series!.ActiveSeed);
        Assert.Equal(ClassicTournamentPhase.Tournament, tournament.Series.PopulationSeedingPhase);
        Assert.Equal(new PopulationSeedSnapshot(next, 5, 2000) { FileName = "5.txt" }, tournament.Series.PopulationSeeds[1]);
        Assert.Equal(new[] { gone, next }.Order(), tournament.Series.ActiveMatch.Order());
        Assert.Equal(2, tournament.Series.PopulationSeeds.Count);
        Assert.Empty(tournament.Results);
        Assert.Null(tournament.Adjudication);
        var boundary = resumed.Save();
        resumed.Load(boundary);
        Assert.Equal(boundary, resumed.Save());
        Assert.Equal(tournament.Series.ActiveMatch, resumed.Snapshot().LocalMode.Series!.ActiveMatch);
        resumed.AdjudicateLocalRound(next, "controlled classic winner");
        var complete = resumed.Snapshot().LocalMode;
        Assert.Equal(LocalModeStatus.Completed, complete.Status);
        Assert.Equal(ClassicTournamentPhase.Complete, complete.Series!.PopulationSeedingPhase);
        Assert.Equal(new[] { next }, complete.Series.Ladder);
        Assert.False(complete.Series.EnteredLadder);
        Assert.Equal(original, resumed.ExportSpeciesTemplateDna(gone));
        var directory = Path.Combine(Path.GetTempPath(), "db-native-classic-winner-" + Guid.NewGuid().ToString("N"));
        try
        {
            var paths = await ClassicTournamentDnaExport.WriteAsync(directory, complete,
                [new(next, resumed.Snapshot().Species[(int)next].Name, resumed.ExportSpeciesTemplateDna(next))]);
            Assert.Equal("Next.txt", Path.GetFileName(Assert.Single(paths)));
            Assert.Equal("start stop" + "\r\n" + "'#tag:Next.txt" + "\r\n",
                await File.ReadAllTextAsync(paths[0]));
            Assert.Equal(original, resumed.ExportSpeciesTemplateDna(gone));
        }
        finally { if (Directory.Exists(directory)) Directory.Delete(directory, true); }
    }

    [Fact]
    public async Task NativeClassicLadderHistorySurvivesSaveAndExportsFinalRankOrder()
    {
        using var engine = new NativeEngineClient(new WorldSetupOptions {
            Backend = "Cpu", Seed = 42, PopulationCapacity = 32, BrownianMotion = 0,
        }, new TestClock(1125));
        for (var index = 0; index < 4; index++)
            engine.ImportSpecies(new($"Original{index}.txt", $"' source {index}\nstart stop", false, 0xff123456, 1, 1000, 0, false, 0));
        var entrants = engine.Snapshot().Species.Select((value, index) => (value, index))
            .Where(item => item.value.Name.StartsWith("Original", StringComparison.Ordinal)).Select(item => (uint)item.index).ToArray();
        engine.ConfigureLocalMode(LocalModeConfig.Default with {
            Workflow = LeagueWorkflow.PopulationSeeding, EntrantOrder = entrants,
            FinishWithLadder = true, MinimumRounds = 1, MaximumRounds = 1,
        });
        engine.StartLocalMode();
        foreach (var score in new[] { 0, 5, 5, 9 })
        {
            engine.Load(PopulationSeedCycle(engine.Save(), 1999, score));
            engine.Tick();
        }
        Assert.Equal(new[] { "0.txt", "5.txt", "5a.txt", "9.txt" },
            engine.Snapshot().LocalMode.Series!.PopulationSeeds.Select(value => value.FileName));
        var observedLadder = false;
        for (var match = 0; match < 20 && engine.Snapshot().LocalMode.Status != LocalModeStatus.Completed; match++)
        {
            var mode = engine.Snapshot().LocalMode;
            if (mode.Series!.PopulationSeedingPhase == ClassicTournamentPhase.Ladder)
            {
                observedLadder = true;
                Assert.True(mode.Series.EnteredLadder);
                var save = engine.Save();
                engine.Load(save);
                Assert.Equal(save, engine.Save());
            }
            Assert.Equal(2, mode.Series.ActiveMatch.Count);
            engine.AdjudicateLocalRound(mode.Series.ActiveMatch[0], "controlled native schedule");
        }
        var complete = engine.Snapshot().LocalMode;
        Assert.True(observedLadder);
        Assert.Equal(LocalModeStatus.Completed, complete.Status);
        Assert.Equal(ClassicTournamentPhase.Complete, complete.Series!.PopulationSeedingPhase);
        Assert.True(complete.Series.EnteredLadder);
        Assert.Equal(2, complete.Series.Ladder.Count);
        Assert.Equal(new[] { 0, 5, 5, 9 }, complete.Series.PopulationSeeds.Select(value => value.Population));
        var directory = Path.Combine(Path.GetTempPath(), "db-native-classic-ladder-" + Guid.NewGuid().ToString("N"));
        try
        {
            var finalists = complete.Series.Ladder.Select(id => new ClassicTournamentDna(id,
                engine.Snapshot().Species[(int)id].Name, engine.ExportSpeciesTemplateDna(id))).ToArray();
            var paths = await ClassicTournamentDnaExport.WriteAsync(directory, complete, finalists);
            Assert.Equal(finalists.Select((value, index) => $"{index + 1}-{value.SourceName}"), paths.Select(Path.GetFileName));
            for (var index = 0; index < paths.Count; index++)
                Assert.Equal(finalists[index].Dna + "\r\n" + "'#tag:" + finalists[index].SourceName + "\r\n",
                    await File.ReadAllTextAsync(paths[index]));
        }
        finally { if (Directory.Exists(directory)) Directory.Delete(directory, true); }
    }

    [Fact]
    public void ClassicSkinClockKeepsRawSingleSamplesAndModernSaveProvenance()
    {
        var clock = new AdvancingTestClock();
        using var engine = new NativeEngineClient(new WorldSetupOptions {
            Backend = "Cpu", Seed = 42, PopulationCapacity = 32, BrownianMotion = 0,
        }, clock);
        const string raw = "' Exact source comment\r\nstart 7 .up store stop\r\n";
        engine.ImportSpecies(new("SourceA.txt", raw, false, 0xff123456, 1, 1000, 0, false, 0));
        engine.ImportSpecies(new("SourceB.txt", "start stop", false, 0xff654321, 1, 1000, 0, false, 0));
        var entrants = engine.Snapshot().Species.Select((value, index) => (value, index))
            .Where(item => item.value.Name.StartsWith("Source", StringComparison.Ordinal)).Select(item => (uint)item.index).ToArray();
        engine.ConfigureLocalMode(LocalModeConfig.Default with {
            Workflow = LeagueWorkflow.PopulationSeeding, EntrantOrder = entrants, MinimumRounds = 1, MaximumRounds = 1,
        });
        engine.StartLocalMode();
        Assert.Equal(2, clock.Samples);
        Assert.Equal(112ul, SavedConfigurationValue(engine, "native_restart_seed"));
        Assert.Equal(raw, engine.ExportSpeciesTemplateDna(entrants[0]));
        using (var first = JsonDocument.Parse(engine.Save().AsMemory(10)))
        {
            var profile = Assert.Single(first.RootElement.GetProperty("local_mode").GetProperty("series")
                .GetProperty("classic_skin_profiles").EnumerateArray());
            Assert.Equal("Test.txt", profile.GetProperty("alias").GetString());
            Assert.Equal(1.125f, profile.GetProperty("timer_seconds").GetSingle());
            Assert.NotEqual(112 / 100f, profile.GetProperty("timer_seconds").GetSingle());
            Assert.False(profile.GetProperty("synthetic_timer").GetBoolean());
            Assert.False(profile.GetProperty("canonical_source").GetBoolean());
            Assert.False(first.RootElement.GetProperty("config").TryGetProperty("skin_timer_seconds", out _));
        }
        engine.Load(PopulationSeedCycle(engine.Save(), 1999, 5));
        engine.Tick();
        engine.Load(PopulationSeedCycle(engine.Save(), 1999, 5));
        engine.Tick();
        Assert.Equal(6, clock.Samples);
        Assert.Equal(212ul, SavedConfigurationValue(engine, "native_restart_seed"));
        var saved = engine.Save();
        using (var document = JsonDocument.Parse(saved.AsMemory(10)))
        {
            var profiles = document.RootElement.GetProperty("local_mode").GetProperty("series")
                .GetProperty("classic_skin_profiles").EnumerateArray().ToArray();
            Assert.Equal(2, profiles.Length);
            Assert.Equal(2.125f, profiles.Single(profile => profile.GetProperty("alias").GetString() == "robotA.txt")
                .GetProperty("timer_seconds").GetSingle());
            Assert.Equal(2.375f, profiles.Single(profile => profile.GetProperty("alias").GetString() == "robotB.txt")
                .GetProperty("timer_seconds").GetSingle());
            Assert.All(profiles, profile => Assert.Equal(8, profile.GetProperty("skin").GetArrayLength()));
        }
        var resumedClock = new AdvancingTestClock();
        using var resumed = new NativeEngineClient(new WorldSetupOptions { Backend = "Cpu" }, resumedClock);
        resumed.Load(saved);
        Assert.Equal(saved, resumed.Save());
        Assert.Equal(raw, resumed.ExportSpeciesTemplateDna(entrants[0]));
        resumed.Tick();
        Assert.Equal(2, resumedClock.Samples);
    }

    [Fact]
    public async Task SingleClassicEntrantCompletesBeforeOptionalLadderAndExportsThePlainChampion()
    {
        using var engine = new NativeEngineClient(new WorldSetupOptions {
            Backend = "Cpu", Seed = 42, PopulationCapacity = 16,
        }, new TestClock(1125));
        engine.ImportSpecies(new("Only.txt", "' only\nstart stop", false, 0xff123456, 1, 1000, 0, false, 0));
        var id = (uint)engine.Snapshot().Species.Select((value, index) => (value, index))
            .Single(item => item.value.Name == "Only.txt").index;
        engine.ConfigureLocalMode(LocalModeConfig.Default with {
            Workflow = LeagueWorkflow.PopulationSeeding, EntrantOrder = [id], FinishWithLadder = true,
        });
        engine.StartLocalMode();
        engine.Load(PopulationSeedCycle(engine.Save(), 1999, 5));
        engine.Tick();
        var complete = engine.Snapshot().LocalMode;
        Assert.Equal(LocalModeStatus.Completed, complete.Status);
        Assert.Equal(ClassicTournamentPhase.Complete, complete.Series!.PopulationSeedingPhase);
        Assert.False(complete.Series.EnteredLadder);
        Assert.True(complete.Configuration!.FinishWithLadder);
        var directory = Path.Combine(Path.GetTempPath(), "db-native-single-classic-" + Guid.NewGuid().ToString("N"));
        try
        {
            var paths = await ClassicTournamentDnaExport.WriteAsync(directory, complete,
                [new(id, engine.Snapshot().Species[(int)id].Name, engine.ExportSpeciesTemplateDna(id))]);
            Assert.Equal(Path.Combine(directory, "Tournament_Results", "Only.txt"), Assert.Single(paths));
            Assert.False(Directory.Exists(Path.Combine(directory, "stepladder")));
        }
        finally { if (Directory.Exists(directory)) Directory.Delete(directory, true); }
    }

    [Fact]
    public void ExistingSeedFilterKeepsItsCompetitivePairAndSavedToken()
    {
        using var engine = new NativeEngineClient(new WorldSetupOptions
        {
            Backend = "Cpu", Seed = 42, PopulationCapacity = 16,
        }, new TestClock(1125));
        engine.ImportSpecies(new("Candidate.txt", "start stop", false, 0xff123456, 1, 1000, 0, false, 0));
        engine.ImportSpecies(new("Reference.txt", "start stop", false, 0xff654321, 1, 1000, 0, false, 0));
        var animals = engine.Snapshot().Species.Select((species, index) => (species, index))
            .Where(item => item.species.Name is "Candidate.txt" or "Reference.txt").Select(item => (uint)item.index).ToArray();
        engine.ConfigureLocalMode(LocalModeConfig.Default with
        {
            Workflow = LeagueWorkflow.Seeding, EntrantOrder = [animals[0]], SeedingOpponent = animals[1],
        });
        engine.StartLocalMode();
        var before = engine.Snapshot().LocalMode;
        Assert.Equal(LeagueWorkflow.Seeding, before.Configuration!.Workflow);
        Assert.Equal(animals, before.Series!.ActiveMatch);
        Assert.Null(before.Series.ActiveSeed);
        Assert.Empty(before.Series.PopulationSeeds);
        using var resumed = new NativeEngineClient(new WorldSetupOptions { Backend = "Cpu" });
        resumed.Load(engine.Save());
        Assert.Equal(LeagueWorkflow.Seeding, resumed.Snapshot().LocalMode.Configuration!.Workflow);
        Assert.Equal(animals, resumed.Snapshot().LocalMode.Series!.ActiveMatch);
        using var saved = JsonDocument.Parse(resumed.Save().AsMemory(10));
        Assert.Equal("seeding", saved.RootElement.GetProperty("local_mode").GetProperty("config").GetProperty("workflow").GetString());
    }

    private static byte[] PopulationSeedCycle(byte[] save, int cycle, int census = 0)
    {
        var payload = System.Text.Json.Nodes.JsonNode.Parse(System.Text.Encoding.UTF8.GetString(save, 10, save.Length - 10))!;
        payload["native_cycle"] = cycle;
        payload["native_population_census"] = new System.Text.Json.Nodes.JsonArray(census, census);
        var json = JsonSerializer.SerializeToUtf8Bytes(payload);
        var framed = new byte[10 + json.Length];
        save.AsSpan(0, 6).CopyTo(framed);
        System.Buffers.Binary.BinaryPrimitives.WriteUInt32LittleEndian(framed.AsSpan(6, 4), (uint)json.Length);
        json.CopyTo(framed.AsSpan(10));
        return framed;
    }

    [Fact]
    public void NativeFreshExplicitAndContinuationBoundariesKeepDistinctCyclesAndMonotonicTicks()
    {
        using var engine = new NativeEngineClient(new WorldSetupOptions
        {
            Backend = "Cpu", Seed = 42, PopulationCapacity = 16,
            BrownianMotion = 0, MutationSettings = MutationSettingsUpdate.Default with { Enabled = false },
        }, new TestClock(1125));
        Assert.Equal(0ul, engine.Snapshot().Tick);
        Assert.Equal(-1, engine.Snapshot().NativeCycle);
        engine.ImportSpecies(new("Alpha.txt", "start stop", false, 0xff123456, 1, 1000, 0, false, 0));
        engine.ImportSpecies(new("Beta.txt", "start stop", false, 0xff654321, 1, 1000, 0, false, 0));
        engine.Tick();
        Assert.Equal(1ul, engine.Snapshot().Tick);
        Assert.Equal(0, engine.Snapshot().NativeCycle);
        engine.ConfigureLocalMode(LocalModeConfig.Default with
        {
            Kind = LocalModeKind.Contest, MinimumRounds = 10, MaximumRounds = 10,
        });

        engine.StartLocalMode();
        Assert.Equal(1ul, engine.Snapshot().Tick);
        Assert.Equal(-1, engine.Snapshot().NativeCycle);
        engine.Tick();
        Assert.Equal(2ul, engine.Snapshot().Tick);
        Assert.Equal(0, engine.Snapshot().NativeCycle);
        engine.AdjudicateLocalRound(1, "native continuation cycle boundary");
        Assert.Equal(2ul, engine.Snapshot().Tick);
        Assert.Equal(0, engine.Snapshot().NativeCycle);
        Assert.Equal(2u, engine.Snapshot().LocalMode.CurrentRound);
        engine.Tick();
        Assert.Equal(3ul, engine.Snapshot().Tick);
        Assert.Equal(1, engine.Snapshot().NativeCycle);

        var saved = engine.Save();
        using var resumed = new NativeEngineClient(new WorldSetupOptions { Backend = "Cpu" });
        resumed.Load(saved);
        Assert.Equal(saved, resumed.Save());
        Assert.Equal(3ul, resumed.Snapshot().Tick);
        Assert.Equal(1, resumed.Snapshot().NativeCycle);
        resumed.RestartLocalMode();
        Assert.Equal(3ul, resumed.Snapshot().Tick);
        Assert.Equal(-1, resumed.Snapshot().NativeCycle);
        resumed.Tick();
        Assert.Equal(4ul, resumed.Snapshot().Tick);
        Assert.Equal(0, resumed.Snapshot().NativeCycle);
    }

    [Theory]
    [InlineData(0u, -1, 0)]
    [InlineData(1u, 0, 1)]
    public void NativeLoadedCycleAdvancesWithoutReapplyingTheFreshWorldOffset(uint initialTicks, int initialCycle, int nextCycle)
    {
        using var source = new NativeEngineClient(new WorldSetupOptions
        {
            Backend = "Cpu", Seed = 42, PopulationCapacity = 16, BrownianMotion = 0,
        });
        source.ImportDna("start stop", [500, 500]);
        source.Tick(initialTicks);
        Assert.Equal(initialCycle, source.Snapshot().NativeCycle);
        var native = source.SaveLegacy();
        using var loaded = new NativeEngineClient(new WorldSetupOptions { Backend = "Cpu" });
        loaded.Load(native);
        var before = loaded.Snapshot();
        Assert.Equal(initialCycle, before.NativeCycle);

        loaded.Tick();

        Assert.Equal(before.Tick + 1, loaded.Snapshot().Tick);
        Assert.Equal(nextCycle, loaded.Snapshot().NativeCycle);
    }

    [Fact]
    public void DeathParticlePolicyRoundTripsAndOnlyConnectedDeletionSuppressesItsOwnParticles()
    {
        var options = new WorldSetupOptions { Backend = "Cpu", Seed = 42, PopulationCapacity = 16 };
        using var engine = new NativeEngineClient(options);
        for (var index = 0; index < 4; index++)
            engine.ImportDna("start stop", [1_000f + index * 500, 1_000f]);
        var before = engine.Snapshot();
        var actors = before.Organisms.ToArray();
        Assert.Equal(4, actors.Length);
        Assert.True(before.DeathPoffsEnabled);
        var originalSave = engine.Save();
        using var original = JsonDocument.Parse(originalSave.AsMemory(10));
        engine.SetDeathPoffsEnabled(false);
        Assert.False(engine.Snapshot().DeathPoffsEnabled);
        Assert.False(engine.ReadNativeProcessCaches().DeathPoffsEnabled);
        var disabledSave = engine.Save();
        Assert.NotEqual(Convert.ToBase64String(originalSave), Convert.ToBase64String(disabledSave));
        using (var disabled = JsonDocument.Parse(disabledSave.AsMemory(10)))
        {
            Assert.Equal(original.RootElement.GetProperty("tick").GetRawText(), disabled.RootElement.GetProperty("tick").GetRawText());
            Assert.Equal(original.RootElement.GetProperty("native_random").GetRawText(), disabled.RootElement.GetProperty("native_random").GetRawText());
            Assert.Equal(original.RootElement.GetProperty("config").GetRawText(), disabled.RootElement.GetProperty("config").GetRawText());
        }
        engine.Remove(actors[0].Slot, actors[0].Generation);
        Assert.Empty(engine.Snapshot().Shots.Where(shot => shot.Kind == -100));
        engine.SetDeathPoffsEnabled(true);
        engine.Remove(actors[1].Slot, actors[1].Generation);
        var particles = engine.Snapshot().Shots.Where(shot => shot.Kind == -100).ToArray();
        Assert.Equal(20, particles.Length);
        const uint originalColor = 0xFF62A844;
        // Source DBrite rounds each (channel + 255) / 2 to Long, ties to even:
        // (98, 168, 68) -> (176, 212, 162). Both source choices are valid.
        const uint brighterColor = 0xFFB0D4A2;
        Assert.Equal(originalColor, actors[1].Phenotype.Color);
        Assert.All(particles, shot => Assert.True(
            shot.Color == originalColor || shot.Color == brighterColor,
            $"Unexpected native death-particle color {shot.Color:X8}."));
        engine.SetDeathPoffsEnabled(false);
        Assert.Equal(JsonSerializer.Serialize(particles), JsonSerializer.Serialize(engine.Snapshot().Shots.Where(shot => shot.Kind == -100).ToArray()));
        var saved = engine.Save();
        using var restored = new NativeEngineClient(options);
        restored.Load(saved);
        Assert.False(restored.Snapshot().DeathPoffsEnabled);
        Assert.False(restored.ReadNativeProcessCaches().DeathPoffsEnabled);
        Assert.Equal(saved, restored.Save());
        Assert.Equal(JsonSerializer.Serialize(particles), JsonSerializer.Serialize(restored.Snapshot().Shots.Where(shot => shot.Kind == -100).ToArray()));
        engine.SetDeathPoffsEnabled(true);
        engine.RemoveMany([new(actors[2].Slot, actors[2].Generation)], suppressDeathPoffs: true);
        Assert.Equal(particles.Length, engine.Snapshot().Shots.Count(shot => shot.Kind == -100));
        Assert.True(engine.Snapshot().DeathPoffsEnabled);
        engine.RemoveMany([new(actors[3].Slot, actors[3].Generation)]);
        Assert.Equal(particles.Length * 2, engine.Snapshot().Shots.Count(shot => shot.Kind == -100));
        Assert.Equal(before.Tick, engine.Snapshot().Tick);
        Assert.Equal(before.NativeCycle, engine.Snapshot().NativeCycle);
    }

    [Theory]
    [InlineData(0f)]
    [InlineData(1.25f)]
    [InlineData(-2.5f)]
    public void NewClientInheritsOnlyItsExplicitGaussianSpareWithoutDrawingOrCouplingClients(float spare)
    {
        var options = new WorldSetupOptions { Backend = "Cpu", Seed = 42 };
        using var independent = new NativeEngineClient(options);
        var caches = EnergyCaches(123, 0.125, repopulationCounter: -17);
        using var inherited = new NativeEngineClient(options with { NativeGaussianSpare = spare, NativeSunChange = 12, NativeProcessCaches = caches });
        using var baseline = JsonDocument.Parse(independent.Save().AsMemory(10));
        using var handoff = JsonDocument.Parse(inherited.Save().AsMemory(10));
        Assert.Equal(baseline.RootElement.GetProperty("native_random").GetProperty("state").GetUInt32(),
            handoff.RootElement.GetProperty("native_random").GetProperty("state").GetUInt32());
        var saved = inherited.Save();

        Assert.Equal(spare, inherited.ReadNativeGaussianSpare());
        Assert.Equal(spare, inherited.ReadNativeGaussianSpare());
        Assert.Null(independent.ReadNativeGaussianSpare());
        Assert.Equal((byte)12, inherited.ReadNativeSunChange());
        Assert.Equal((byte)0, independent.ReadNativeSunChange());
        AssertProcessCaches(caches, inherited.ReadNativeProcessCaches());
        AssertProcessCaches(caches, inherited.ReadNativeProcessCaches());
        AssertProcessCaches(NativeProcessCachesSnapshot.Empty with { RepopulationCounter = 0 }, independent.ReadNativeProcessCaches());
        Assert.Equal(saved, inherited.Save());
        Assert.Equal(0ul, inherited.Snapshot().Tick);
    }

    [Theory]
    [InlineData(false)]
    [InlineData(true)]
    public void NativeLoadRetainsTheProcessHandoffButModernLoadRestoresItsOwnGaussianState(bool savedHasSpare)
    {
        using var inherited = new NativeEngineClient(new WorldSetupOptions
        {
            Backend = "Cpu", Seed = 42, NativeGaussianSpare = 1.25f, NativeSunChange = 12,
            NativeProcessCaches = EnergyCaches(123, 0.125, repopulationCounter: int.MinValue),
        });
        const string relative = "modern/engine/tests/fixtures/db24832-original-user.sim";
        var directory = new DirectoryInfo(AppContext.BaseDirectory);
        while (directory is not null && !File.Exists(Path.Combine(directory.FullName, relative))) directory = directory.Parent;
        Assert.NotNull(directory);
        var original = File.ReadAllBytes(Path.Combine(directory.FullName, relative));
        using var reference = new NativeEngineClient(new WorldSetupOptions { Backend = "Cpu" });
        reference.Load(original);
        inherited.Load(original);
        Assert.Equal(1.25f, inherited.ReadNativeGaussianSpare());
        Assert.Equal(reference.ReadNativeSunChange(), inherited.ReadNativeSunChange());
        // startloaded resets this source process global to -RepopCooldown.
        // Other process caches still inherit; the incoming Long minimum must not win.
        var loadedCounter = -checked((int)reference.ReadConfiguration().Environment.Vegetation.RepopulationCooldown);
        Assert.NotEqual(int.MinValue, loadedCounter);
        AssertProcessCaches(EnergyCaches(123, 0.125, repopulationCounter: loadedCounter), inherited.ReadNativeProcessCaches());
        AssertProcessCaches(NativeProcessCachesSnapshot.Empty with { RepopulationCounter = loadedCounter }, reference.ReadNativeProcessCaches());
        using var source = new NativeEngineClient(new WorldSetupOptions
        {
            Backend = "Cpu", Seed = 17, NativeGaussianSpare = savedHasSpare ? 2.5f : null, NativeSunChange = 7,
            NativeProcessCaches = EnergyCaches(-456, 0.375, 21, true, repopulationCounter: 23),
        });
        var modern = source.Save();

        inherited.Load(modern);

        Assert.Equal(savedHasSpare ? 2.5f : (float?)null, inherited.ReadNativeGaussianSpare());
        Assert.Equal((byte)7, inherited.ReadNativeSunChange());
        AssertProcessCaches(EnergyCaches(-456, 0.375, 21, true, repopulationCounter: 23), inherited.ReadNativeProcessCaches());
        Assert.Equal(modern, inherited.Save());
        Assert.Equal(0ul, inherited.Snapshot().Tick);
    }

    [Theory]
    [InlineData(true, false)]
    [InlineData(false, true)]
    [InlineData(true, true)]
    public void WeatherOnNativeHeaderOverridesIncomingProcessSunChange(bool weatherEnabled, bool sunOnRandom)
    {
        using var engine = new NativeEngineClient(new WorldSetupOptions
        {
            Backend = "Cpu", Seed = 42, NativeSunChange = 12,
            Vegetation = Db2VegetationOptions.Default with { WeatherEnabled = weatherEnabled, SunOnRandom = sunOnRandom },
        });
        Assert.Equal((byte)10, engine.ReadNativeSunChange());
        using var save = JsonDocument.Parse(engine.Save().AsMemory(10));
        Assert.Equal(0x008fc6e7u, save.RootElement.GetProperty("native_random").GetProperty("state").GetUInt32());
        Assert.Equal(0ul, engine.Snapshot().Tick);
    }

    [Fact]
    public void DirectModeConfiguredStartAndExplicitRestartKeepTheChosenSeedAndGaussianSpare()
    {
        var clock = new TestClock(1125);
        using var engine = new NativeEngineClient(new WorldSetupOptions
        {
            Backend = "Cpu", Seed = 42, NativeGaussianSpare = 1.25f,
        }, clock);
        engine.ImportSpecies(new("Alpha.txt", "start stop", false, 0xff123456, 1, 1000, 0, false, 0));
        engine.ImportSpecies(new("Beta.txt", "start stop", false, 0xff654321, 1, 1000, 0, false, 0));
        engine.ConfigureLocalMode(LocalModeConfig.Default with { Kind = LocalModeKind.Contest });
        engine.StartLocalMode();
        Assert.Equal(42ul, SavedConfigurationValue(engine, "seed"));
        Assert.Equal(112ul, SavedConfigurationValue(engine, "native_restart_seed"));
        Assert.Equal(1.25f, engine.ReadNativeGaussianSpare());
        clock.Milliseconds = 1375;

        engine.RestartLocalMode();

        Assert.Equal(42ul, SavedConfigurationValue(engine, "seed"));
        Assert.Equal(138ul, SavedConfigurationValue(engine, "native_restart_seed"));
        Assert.Equal(1.25f, engine.ReadNativeGaussianSpare());
        Assert.Equal(0ul, engine.Snapshot().Tick);
    }

    [Theory]
    [InlineData(1125, 112UL)]
    [InlineData(1375, 138UL)]
    public void NewSimulationSeedUsesTimerSingleAndLongTiesToEven(int milliseconds, ulong expectedSeed)
    {
        var clock = new TestClock(milliseconds);
        using var engine = new NativeEngineClient(new WorldSetupOptions
        {
            Backend = "Cpu", Seed = ulong.MaxValue,
            GlobalSettings = new GlobalSettings { Main = new GlobalMainSettings { NewSeed = true } },
        }, clock);

        Assert.Equal(expectedSeed, SavedConfigurationValue(engine, "seed"));
        Assert.Equal(0ul, engine.Snapshot().Tick);
    }

    [Fact]
    public void ExplicitFullWidthSeedAndModernResumeIgnoreTheHostClockUntilALiveBoundary()
    {
        var clock = new TestClock(1125);
        using var engine = new NativeEngineClient(new WorldSetupOptions
        {
            Backend = "Cpu", Seed = 0x1234567880000000UL,
            GlobalSettings = new GlobalSettings { Main = new GlobalMainSettings { NewSeed = false } },
        }, clock);
        var save = engine.Save();
        Assert.Equal(0x1234567880000000UL, SavedConfigurationValue(engine, "seed"));
        clock.Milliseconds = 1375;
        Assert.Equal(save, engine.Save());

        using var resumed = new NativeEngineClient(new WorldSetupOptions
        {
            Backend = "Cpu",
            GlobalSettings = new GlobalSettings { Main = new GlobalMainSettings { NewSeed = true } },
        }, clock);
        Assert.Equal(138ul, SavedConfigurationValue(resumed, "seed"));
        resumed.Load(save);

        Assert.Equal(save, resumed.Save());
        Assert.Equal(0x1234567880000000UL, SavedConfigurationValue(resumed, "seed"));
    }

    [Fact]
    public void LiveModeBoundariesCaptureCurrentTimerAndRetainTheInputThroughModernResume()
    {
        var clock = new TestClock(1125);
        using var engine = new NativeEngineClient(new WorldSetupOptions { Backend = "Cpu", Seed = 42 }, clock);
        engine.ImportSpecies(new("Alpha.txt", "start stop", false, 0xff123456, 1, 1000, 0, false, 0));
        engine.ImportSpecies(new("Beta.txt", "start stop", false, 0xff654321, 1, 1000, 0, false, 0));
        engine.ConfigureLocalMode(LocalModeConfig.Default with
        {
            Kind = LocalModeKind.Contest, MinimumRounds = 10, MaximumRounds = 10,
        });

        engine.StartLocalMode();
        Assert.Equal(42ul, SavedConfigurationValue(engine, "seed"));
        Assert.Equal(112ul, SavedConfigurationValue(engine, "native_restart_seed"));

        clock.Milliseconds = 1375;
        engine.RestartLocalMode();
        Assert.Equal(42ul, SavedConfigurationValue(engine, "seed"));
        Assert.Equal(138ul, SavedConfigurationValue(engine, "native_restart_seed"));

        clock.Milliseconds = 2125;
        engine.Tick();
        Assert.Equal(42ul, SavedConfigurationValue(engine, "seed"));
        Assert.Equal(212ul, SavedConfigurationValue(engine, "native_restart_seed"));

        clock.Milliseconds = 2375;
        engine.AdjudicateLocalRound(1, "controlled clock round");
        Assert.Equal(238ul, SavedConfigurationValue(engine, "native_restart_seed"));
        var result = Assert.Single(engine.Snapshot().LocalMode.Results);
        Assert.Equal(1u, result.Winner);
        Assert.Equal("controlled clock round", result.Reason);
        Assert.Equal(2u, engine.Snapshot().LocalMode.CurrentRound);
        var currentRoundSeed = SavedConfigurationValue(engine, "seed");

        var save = engine.Save();
        using var resumed = new NativeEngineClient(new WorldSetupOptions { Backend = "Cpu" }, new TestClock(3375));
        resumed.Load(save);
        Assert.Equal(save, resumed.Save());
        Assert.Equal(238ul, SavedConfigurationValue(resumed, "native_restart_seed"));
        resumed.RestartLocalMode();
        Assert.Equal(338ul, SavedConfigurationValue(resumed, "native_restart_seed"));
        Assert.Equal(currentRoundSeed, SavedConfigurationValue(resumed, "seed"));
    }

    [Fact]
    public void AutomaticStagedRestartUsesTheTimerCapturedByTheTickBatch()
    {
        var clock = new TestClock(1125);
        using var engine = new NativeEngineClient(new WorldSetupOptions { Backend = "Cpu", Seed = 42 }, clock);
        engine.ConfigureLocalMode(LocalModeConfig.Default with
        {
            Kind = LocalModeKind.SimpleSurvival,
            StagedEvolution = new(EvolutionMode.SimpleSurvival, "start stop", 2, 150, false, 255, false),
        });
        engine.StartLocalMode();
        Assert.Equal(112ul, SavedConfigurationValue(engine, "seed"));
        var before = engine.Snapshot();
        var mutatorSpecies = before.LocalMode.Evolution!.MutatorSpecies;
        var mutators = before.Organisms.Where(robot => mutatorSpecies.Contains(robot.Species))
            .Select(robot => new OrganismKey(robot.Slot, robot.Generation)).ToArray();
        Assert.NotEmpty(mutators);
        engine.RemoveMany(mutators);

        clock.Milliseconds = 1375;
        engine.Tick();

        Assert.Equal(138ul, SavedConfigurationValue(engine, "seed"));
        var after = engine.Snapshot();
        var evolution = Assert.IsType<StagedEvolutionSnapshot>(after.LocalMode.Evolution);
        Assert.Equal("evolving", evolution.Phase);
        Assert.Contains(after.Organisms, robot => evolution.MutatorSpecies.Contains(robot.Species));
        Assert.True(after.LocalMode.RestartCount > before.LocalMode.RestartCount);
    }

    [Fact]
    public void ManualFindBestUsesLiveZerobotWeightWithoutChangingTheReadOnlyQueryContract()
    {
        var clock = new AdvancingTestClock();
        using var engine = new NativeEngineClient(new WorldSetupOptions
        {
            Backend = "Cpu", Seed = 42, PopulationCapacity = 2,
        }, clock);
        engine.ImportSpecies(new("Animal.txt", "start stop", false, 0xff123456, 1, 1000, 0, false, 0));
        var ordinary = engine.Save();
        var queried = engine.FindBestRobot(137);
        var selected = engine.SelectBestRobot(137);
        Assert.Equal(queried, selected.Robot);
        Assert.False(selected.Restarted);
        Assert.Equal((ushort)137, selected.EffectiveWeight);
        Assert.Equal(0, clock.Samples);
        Assert.Equal(ordinary, engine.Save());
        engine.ConfigureLocalMode(LocalModeConfig.Default with
        {
            Kind = LocalModeKind.Zerobot,
            StagedEvolution = new(EvolutionMode.Zerobot, "", 10, 150, false, 32, false) { FitnessWeight = 100 },
        });
        Assert.Equal(LocalModeStatus.Configured, engine.Snapshot().LocalMode.Status);
        var configured = engine.Save();
        selected = engine.SelectBestRobot(137);
        Assert.Equal(queried, selected.Robot);
        Assert.False(selected.Restarted);
        Assert.Equal((ushort)137, selected.EffectiveWeight);
        Assert.Equal(0, clock.Samples);
        Assert.Equal(configured, engine.Save());
        engine.StartLocalMode();
        var before = engine.Save();
        var snapshot = engine.Snapshot();
        Assert.Equal((ushort)29, snapshot.LocalMode.Evolution!.FitnessWeight);
        Assert.Equal((ushort)100, snapshot.LocalMode.Configuration!.StagedEvolution!.FitnessWeight);
        var samples = clock.Samples;
        Assert.NotNull(engine.FindBestRobot(200));
        Assert.Equal(samples, clock.Samples);
        Assert.Equal(before, engine.Save());
        selected = engine.SelectBestRobot(200);
        Assert.Equal((ushort)29, selected.EffectiveWeight);
        Assert.False(selected.Restarted, "an unmutated starter cannot advance Zerobot");
        Assert.NotNull(selected.Robot);
        Assert.Equal(samples + 1, clock.Samples);
        Assert.Equal(snapshot.Tick, engine.Snapshot().Tick);
        Assert.Equal(snapshot.NativeCycle, engine.Snapshot().NativeCycle);
        // The unused host clock sample must not alter the world or pending seed.
        Assert.Equal(before, engine.Save());
        var after = engine.Save();
        Assert.Throws<ArgumentOutOfRangeException>(() => engine.SelectBestRobot(201));
        Assert.Equal(samples + 1, clock.Samples);
        Assert.Equal(after, engine.Save());
        engine.Dispose();
        Assert.Throws<ObjectDisposedException>(() => engine.SelectBestRobot(100));
        Assert.Equal(samples + 1, clock.Samples);
    }

    [Fact]
    public void ZeroTicksAndRejectedModeActionDoNotCommitThePrependedClockInput()
    {
        var clock = new TestClock(1125);
        using var engine = new NativeEngineClient(new WorldSetupOptions { Backend = "Cpu", Seed = 42 }, clock);
        var before = engine.Save();
        clock.Milliseconds = 1375;

        engine.Tick(0);
        Assert.Equal(before, engine.Save());
        Assert.Throws<InvalidOperationException>(() => engine.StartLocalMode());
        Assert.Equal(before, engine.Save());
    }

    [Fact]
    public void StagedBatchSamplesEachExternalRestartAndLoadedModesKeepThatPolicy()
    {
        var clock = new AdvancingTestClock();
        // Base fills the five available slots, so each attempt starts without
        // Mutate and takes the real staged-loss/restart path on its next cycle.
        using var engine = new NativeEngineClient(new WorldSetupOptions
        {
            Backend = "Cpu", Seed = 42, PopulationCapacity = 5,
        }, clock);
        engine.ConfigureLocalMode(LocalModeConfig.Default with
        {
            Kind = LocalModeKind.SimpleSurvival,
            StagedEvolution = new(EvolutionMode.SimpleSurvival, "start stop", 2, 150, false, 255, false),
        });
        engine.StartLocalMode();
        var before = engine.Snapshot();
        var evolution = Assert.IsType<StagedEvolutionSnapshot>(before.LocalMode.Evolution);
        Assert.Equal(5, before.Population);
        Assert.DoesNotContain(before.Organisms, robot => evolution.MutatorSpecies.Contains(robot.Species));
        Assert.Equal(112ul, SavedConfigurationValue(engine, "seed"));
        Assert.Equal(1, clock.Samples);
        var beforeZero = engine.Save();

        engine.Tick(0);
        Assert.Equal(beforeZero, engine.Save());
        Assert.Equal(1, clock.Samples);
        engine.Tick(2);

        Assert.Equal(before.LocalMode.RestartCount + 2u, engine.Snapshot().LocalMode.RestartCount);
        Assert.Equal(3, clock.Samples);
        Assert.Equal(162ul, SavedConfigurationValue(engine, "seed"));
        Assert.Equal(162ul, SavedConfigurationValue(engine, "native_restart_seed"));

        var save = engine.Save();
        var resumedClock = new AdvancingTestClock();
        using var resumed = new NativeEngineClient(new WorldSetupOptions { Backend = "Cpu" }, resumedClock);
        resumed.Load(save);
        Assert.Equal(save, resumed.Save());
        resumed.Tick(2);

        Assert.Equal(2, resumedClock.Samples);
        Assert.Equal(138ul, SavedConfigurationValue(resumed, "seed"));
        Assert.Equal(before.LocalMode.RestartCount + 4u, resumed.Snapshot().LocalMode.RestartCount);
    }

    [Theory]
    [InlineData(false)]
    [InlineData(true)]
    public void OrdinaryAndDirectContestTickBatchesTakeOneClockSample(bool directContest)
    {
        var clock = new AdvancingTestClock();
        using var engine = new NativeEngineClient(new WorldSetupOptions { Backend = "Cpu", Seed = 42 }, clock);
        if (directContest)
        {
            engine.ImportSpecies(new("Alpha.txt", "start stop", false, 0xff123456, 1, 1000, 0, false, 0));
            engine.ImportSpecies(new("Beta.txt", "start stop", false, 0xff654321, 1, 1000, 0, false, 0));
            engine.ConfigureLocalMode(LocalModeConfig.Default with
            {
                Kind = LocalModeKind.Contest, MinimumRounds = 10, MaximumRounds = 10,
            });
            engine.StartLocalMode();
        }
        int samplesBefore = clock.Samples;

        engine.Tick(2);

        Assert.Equal(samplesBefore + 1, clock.Samples);
        Assert.Equal(2ul, engine.Snapshot().Tick);
    }

    private static ulong SavedConfigurationValue(NativeEngineClient engine, string field)
    {
        using var document = JsonDocument.Parse(engine.Save().AsMemory(10));
        return document.RootElement.GetProperty("config").GetProperty(field).GetUInt64();
    }

    private sealed class TestClock(int milliseconds) : TimeProvider
    {
        public int Milliseconds { get; set; } = milliseconds;
        public override DateTimeOffset GetUtcNow() => new DateTimeOffset(2026, 10, 15, 0, 0, 0, TimeSpan.Zero)
            .AddMilliseconds(Milliseconds);
        public override TimeZoneInfo LocalTimeZone => TimeZoneInfo.Utc;
    }

    private sealed class AdvancingTestClock : TimeProvider
    {
        public int Samples { get; private set; }
        public override DateTimeOffset GetUtcNow() => new DateTimeOffset(2026, 10, 15, 0, 0, 0, TimeSpan.Zero)
            .AddMilliseconds(1125 + 250 * Samples++);
        public override TimeZoneInfo LocalTimeZone => TimeZoneInfo.Utc;
    }

    [Theory]
    [InlineData(LeaguePreset.ShortBot, "at most 6 genes")]
    [InlineData(LeaguePreset.SingleGene, "at most 6 genes")]
    [InlineData(LeaguePreset.SingleStore, "exactly 1 store")]
    public void InvalidEntrantsReachTheDesktopAsActionableErrorsWithoutChangingTheWorld(
        LeaguePreset preset, string explanation)
    {
        using var engine = new NativeEngineClient(backend: "Cpu");
        var dna = preset == LeaguePreset.SingleStore
            ? "start .up inc .dn dec stop"
            : string.Concat(Enumerable.Repeat("start stop\n", 7));
        engine.ImportDna(dna, [500, 500]);
        var before = engine.Save();
        var error = Assert.Throws<InvalidOperationException>(() =>
            engine.ConfigureLocalMode(LocalModeConfig.Default with { Preset = preset }));
        Assert.Contains(explanation, error.Message);
        Assert.Equal(before, engine.Save());
    }

    [Fact]
    public void SgConfigurationKeepsItsLegacySaveTokenAndShortBotArenaAfterResume()
    {
        using var engine = new NativeEngineClient(backend: "Cpu");
        engine.ImportDna(string.Concat(Enumerable.Repeat("start stop\n", 6)), [500, 500]);
        engine.ConfigureLocalMode(LocalModeConfig.Default with { Preset = LeaguePreset.SingleGene });
        using var resumed = new NativeEngineClient(backend: "Cpu");
        resumed.Load(engine.Save());
        resumed.StartLocalMode();
        Assert.Equal(new float[] { 16000, 12000 }, resumed.Snapshot().WorldSize);
        Assert.Equal(LeaguePreset.SingleGene, resumed.Snapshot().LocalMode.Preset);
    }
}
