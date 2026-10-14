using Avalonia;
using Darwinbots.Desktop.Core;
using System.Text.Json;

namespace Darwinbots.Desktop;

internal static class Program
{
    [STAThread]
    public static int Main(string[] args)
    {
        if (args.Contains("--smoke-test", StringComparer.Ordinal)) return SmokeTest();
        return BuildAvaloniaApp().StartWithClassicDesktopLifetime(args);
    }

    private static int SmokeTest()
    {
        try
        {
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
            using var engine = new NativeEngineClient(options);
            engine.ImportDna("start 1 .up store stop", [500, 500]);
            engine.Tick(10);
            if (engine.Snapshot().Population != 1 || engine.Snapshot().Organisms[0].Position[0] <= 500)
                throw new InvalidDataException("Packaged engine did not move the imported bot.");
            using var resumed = new NativeEngineClient(options);
            resumed.Load(engine.Save());
            engine.Tick(3);
            resumed.Tick(3);
            static string State(EngineSnapshot snapshot) => JsonSerializer.Serialize(new
            {
                snapshot.Tick, snapshot.Population, snapshot.Organisms, snapshot.Stats,
                snapshot.Ties, snapshot.Shots, snapshot.Species, snapshot.History,
                snapshot.RenderInstances,
            });
            if (State(engine.Snapshot()) != State(resumed.Snapshot()))
                throw new InvalidDataException("Packaged save/resume diverged from continuous execution.");
            Console.WriteLine(JsonSerializer.Serialize(new
            {
                status = "passed", backend = engine.Backend, ticks = engine.Snapshot().Tick,
                population = engine.Snapshot().Population, movement = true, save_resume = true,
                bundled_bots = Directory.GetFiles(bots, "*.txt", SearchOption.AllDirectories).Length,
            }));
            return 0;
        }
        catch (Exception error)
        {
            Console.Error.WriteLine(error.Message);
            return 1;
        }
    }

    public static AppBuilder BuildAvaloniaApp() =>
        AppBuilder.Configure<App>()
            .UsePlatformDetect()
            .LogToTrace();
}
