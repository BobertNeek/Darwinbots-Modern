using Avalonia;
using Darwinbots.Desktop.Core;
using System.Security.Cryptography;
using System.Text.Json;
using System.Text.Json.Serialization;

namespace Darwinbots.Desktop;

internal static class Program
{
    [STAThread]
    public static int Main(string[] args)
    {
        if (args.Contains("--smoke-test", StringComparer.Ordinal)) return SmokeTest(args);
        return BuildAvaloniaApp().StartWithClassicDesktopLifetime(args);
    }

    private static int SmokeTest(string[] args)
    {
        try
        {
            var fixtureIndex = Array.IndexOf(args, "--native-smoke-fixture");
            if (fixtureIndex >= 0 && fixtureIndex + 1 == args.Length)
                throw new ArgumentException("--native-smoke-fixture requires a simulation path.");
            var bots = Path.Combine(AppContext.BaseDirectory, "Bots");
            foreach (var name in new[] { "Alga_Minimalis_Chloroplastus.txt", "Animal_Minimalis.txt",
                "Historical/Alga_Minimalis.txt", "Historical/league/21-Efficient Bot (F1)(Shen)-01.09.04.txt" })
                if (!File.Exists(Path.Combine(bots, name))) throw new InvalidDataException($"Packaged bot is missing: {name}");
            var options = new WorldSetupOptions
            {
                Backend = "Cpu", Seed = 24832, PopulationCapacity = 64,
                WorldWidth = 3000, WorldHeight = 2000, MetabolismCost = 0,
                VegetableEnergyPerTick = 0, SunlightEnergy = 0,
                GlobalSettings = new GlobalSettings { Main = new GlobalMainSettings { NewSeed = false } },
            };
            var clock = new SmokeClock();
            using var engine = new NativeEngineClient(options, clock);
            engine.ImportDna("start 1 .up store stop", [500, 500]);
            engine.Tick(10);
            if (engine.Snapshot().Population != 1 || engine.Snapshot().Organisms[0].Position[0] <= 500)
                throw new InvalidDataException("Packaged engine did not move the imported bot.");
            var roster = engine.ReadConfiguration()?.Species ?? throw new InvalidDataException("Packaged species configuration is unavailable.");
            if (roster.Count == 0 || roster.Any(species => species.SpeciesId is null))
                throw new InvalidDataException("Packaged species configuration omitted stable identities.");
            engine.UpdateSpeciesSettings(roster);
            if (!engine.ReadConfiguration()!.Species.Select(species => species.SpeciesId).SequenceEqual(roster.Select(species => species.SpeciesId)))
                throw new InvalidDataException("Packaged species editing changed stable identities.");
            var historyBeforeSave = engine.Snapshot().Organisms[0].ActivationHistory;
            if (!historyBeforeSave.Select(frame => frame.Tick).SequenceEqual(Enumerable.Range(1, 10).Select(tick => (ulong)tick)))
                throw new InvalidDataException("Packaged inspector did not retain the current session's activation history.");
            using var resumed = new NativeEngineClient(options, clock);
            resumed.Load(engine.Save());
            // DB3S intentionally omits activation bitmap/history and debug text.
            // Loading clears inspection history; an ordinary command must retain it.
            if (resumed.Snapshot().Organisms[0].ActivationHistory.Count != 0 ||
                engine.Snapshot().Organisms[0].ActivationHistory.Count != 10)
                throw new InvalidDataException("Packaged save/load changed the activation-history boundary contract.");
            engine.Tick(3);
            resumed.Tick(3);
            static string State(EngineSnapshot snapshot) => JsonSerializer.Serialize(new
            {
                snapshot.Tick, snapshot.Population,
                Organisms = snapshot.Organisms.Select(organism => organism with { ActivationHistory = [] }), snapshot.Stats,
                snapshot.Ties, snapshot.Shots, snapshot.Species, snapshot.History,
                snapshot.RenderInstances,
            });
            if (State(engine.Snapshot()) != State(resumed.Snapshot()))
                throw new InvalidDataException("Packaged save/resume diverged from continuous execution.");
            var continuousHistory = engine.Snapshot().Organisms[0].ActivationHistory;
            var resumedHistory = resumed.Snapshot().Organisms[0].ActivationHistory;
            if (!continuousHistory.Select(frame => frame.Tick).SequenceEqual(Enumerable.Range(1, 13).Select(tick => (ulong)tick)) ||
                !resumedHistory.Select(frame => frame.Tick).SequenceEqual(new ulong[] { 11, 12, 13 }) ||
                JsonSerializer.Serialize(continuousHistory.TakeLast(3)) != JsonSerializer.Serialize(resumedHistory))
                throw new InvalidDataException("Packaged resumed inspector history diverged from the continuous session's matching tail.");
            if (!engine.Save().AsSpan().SequenceEqual(resumed.Save()))
                throw new InvalidDataException("Packaged save/resume produced different persistent world bytes.");
            var nativeFiles = fixtureIndex < 0 ? null : SmokeNativeFiles(args[fixtureIndex + 1], options, clock);
            Console.WriteLine(JsonSerializer.Serialize(new
            {
                status = "passed", backend = engine.Backend, ticks = engine.Snapshot().Tick,
                population = engine.Snapshot().Population, movement = true, save_resume = true,
                persistent_save_bytes_equal = true, inspection_history_boundary = true, species_settings = true,
                bundled_bots = Directory.GetFiles(bots, "*.txt", SearchOption.AllDirectories).Length,
                native_files = nativeFiles,
            }, new JsonSerializerOptions { DefaultIgnoreCondition = JsonIgnoreCondition.WhenWritingNull }));
            return 0;
        }
        catch (Exception error)
        {
            Console.Error.WriteLine(error.Message);
            return 1;
        }
    }

    private static object SmokeNativeFiles(string fixture, WorldSetupOptions options, TimeProvider clock)
    {
        var fixtureBytes = File.ReadAllBytes(fixture);
        var workingDirectory = Directory.CreateTempSubdirectory("darwinbots-native-smoke-");
        try
        {
            using var engine = new NativeEngineClient(options, clock);
            engine.Load(fixtureBytes);
            var input = engine.Snapshot();
            if (input.NativeCycle != 1 || input.Population != 1)
                throw new InvalidDataException("Expected the original one-robot movement fixture at cycle 1.");

            engine.Tick(1);
            var stepped = engine.Snapshot();
            if (stepped.NativeCycle != 2 || stepped.Population != 1)
                throw new InvalidDataException("The packaged engine did not advance the native fixture to cycle 2.");
            var original = stepped.Organisms.Single();
            var dna = engine.ExportDna(original.Slot, original.Generation);

            var simulationPath = Path.Combine(workingDirectory.FullName, "roundtrip.sim");
            File.WriteAllBytes(simulationPath, engine.SaveLegacy());
            var simulationBytes = File.ReadAllBytes(simulationPath);
            if (simulationBytes.AsSpan().StartsWith("DB3S"u8))
                throw new InvalidDataException("The packaged native simulation writer returned a Modern checkpoint.");
            using var reopened = new NativeEngineClient(options, clock);
            reopened.Load(simulationBytes);
            var simulation = reopened.Snapshot();
            var restored = simulation.Organisms.Single();
            if (simulation.NativeCycle != stepped.NativeCycle || simulation.Population != stepped.Population
                || restored.UniqueId != original.UniqueId || reopened.ExportDna(restored.Slot, restored.Generation) != dna)
                throw new InvalidDataException("The packaged native simulation round trip changed the cycle, population, identity or DNA.");

            var organismPath = Path.Combine(workingDirectory.FullName, "roundtrip.dbo");
            File.WriteAllBytes(organismPath, reopened.ExportOrganismGroup([new(restored.Slot, restored.Generation)]));
            using var receiver = new NativeEngineClient(options, clock);
            var count = receiver.ImportOrganism(File.ReadAllBytes(organismPath));
            var imported = receiver.Snapshot().Organisms.Single();
            if (count != 1 || receiver.ExportDna(imported.Slot, imported.Generation) != dna)
                throw new InvalidDataException("The packaged organism round trip changed the robot count or DNA.");

            return new
            {
                fixture_sha256 = Convert.ToHexStringLower(SHA256.HashData(fixtureBytes)),
                sim_cycle = simulation.NativeCycle,
                sim_population = simulation.Population,
                dbo_count = count,
            };
        }
        finally { workingDirectory.Delete(recursive: true); }
    }

    private sealed class SmokeClock : TimeProvider
    {
        public override DateTimeOffset GetUtcNow() => new(2026, 10, 25, 18, 0, 0, TimeSpan.Zero);
        public override TimeZoneInfo LocalTimeZone => TimeZoneInfo.Utc;
    }

    public static AppBuilder BuildAvaloniaApp() =>
        AppBuilder.Configure<App>()
            .UsePlatformDetect()
            .LogToTrace();
}
