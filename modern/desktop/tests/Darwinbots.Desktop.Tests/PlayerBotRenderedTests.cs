using System.Collections.Concurrent;
using System.Globalization;
using System.Reflection;
using System.Runtime.InteropServices;
using Avalonia;
using Avalonia.Controls;
using Avalonia.Headless;
using Avalonia.Input;
using Avalonia.Interactivity;
using Avalonia.Media.Imaging;
using Avalonia.VisualTree;
using Darwinbots.Desktop.Controls;
using Darwinbots.Desktop.Core;
using Darwinbots.Desktop.Core.LegacyMenus;
using Darwinbots.Desktop.Services;
using Darwinbots.Desktop.ViewModels;
using Darwinbots.Desktop.Views;
using Xunit;

namespace Darwinbots.Desktop.Tests;

public sealed class PlayerBotRenderedTests
{
    [Fact]
    public async Task RealPlayerBotKeyMapRoutesVisibleKeysToTheNextAuthoritativeCycle()
    {
        var factory = new PlayerBotTestEngineFactory();

        await DesktopTestApplication.Session.Dispatch(async () =>
        {
            var window = new MainWindow([], new WorldSetupOptions(), factory, new AuditStorageService());
            window.Show();
            try
            {
                await WaitFor(() => TryField<SimulationSession>(window, "_session") is not null);
                var session = Field<SimulationSession>(window, "_session");
                await session.StepAsync(0);
                await WaitFor(() => session.LatestSnapshot.Organisms.Count == 1);

                var focus = factory.Client.Focus;
                var viewModel = Field<MainWindowViewModel>(window, "_viewModel");
                viewModel.Select(focus);
                var viewport = window.FindControl<WorldViewport>("Viewport")!;
                viewport.SetSnapshot(session.LatestSnapshot);
                viewport.SelectKey(focus);
                window.GetType().GetMethod("RefreshLegacyMenuStates", BindingFlags.Instance | BindingFlags.NonPublic)!
                    .Invoke(window, null);

                var menu = window.FindControl<LegacyMenuBar>("HistoricMenuBar")!;
                menu.FindCommandItem(LegacyCommandId.EditPlayerBotSettings)!
                    .RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));
                await WaitFor(() => window.OwnedWindows.OfType<PlayerBotSettingsWindow>().Any());
                var dialog = Assert.Single(window.OwnedWindows.OfType<PlayerBotSettingsWindow>());

                AssertKeyMapSurface(dialog, "KEY MAP · PRESSED XOR INVERT WRITES MEMORY AFTER SENSE CLEAR");

                var fields = dialog.GetVisualDescendants().OfType<TextBox>().ToArray();
                Assert.Equal(3, fields.Length);
                var capture = dialog.GetVisualDescendants().OfType<Button>()
                    .Single(button => button.Content?.ToString() == "CAPTURE KEY");
                var add = dialog.GetVisualDescendants().OfType<Button>()
                    .Single(button => button.Content?.ToString() == "ADD");
                var invert = dialog.GetVisualDescendants().OfType<CheckBox>().Single();
                var status = Field<TextBlock>(dialog, "_status");

                CaptureAndAdd(dialog, capture, add, fields, invert, Key.W, PhysicalKey.W, 50, 123, false);
                CaptureAndAdd(dialog, capture, add, fields, invert, Key.Space, PhysicalKey.Space, 51, -77, false);
                Assert.Equal(2, dialog.Bindings.Count);
                Assert.Equal("BINDING ADDED", status.Text);

                fields[0].Text = "not-a-key";
                fields[1].Text = "52";
                fields[2].Text = "9";
                add.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                Assert.Equal("ENTER KEY, MEMORY, AND VALUE", status.Text);
                Assert.Equal(2, dialog.Bindings.Count);

                fields[0].Text = ((int)Key.W).ToString(CultureInfo.InvariantCulture);
                fields[1].Text = "52";
                fields[2].Text = "9";
                add.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                Assert.Equal("KEY ALREADY MAPPED", status.Text);
                Assert.Equal(2, dialog.Bindings.Count);
                AssertKeyMapSurface(dialog, "KEY ALREADY MAPPED");

                dialog.GetVisualDescendants().OfType<Button>()
                    .Single(button => button.Content?.ToString() == "APPLY")
                    .RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                await WaitFor(() => !dialog.IsVisible && factory.Client.Configurations.Count == 1);
                Assert.Equal(
                    [
                        new PlayerBotBinding((int)Key.W, 50, 123),
                        new PlayerBotBinding((int)Key.Space, 51, -77),
                    ],
                    factory.Client.Configurations.Single().Bindings);

                menu.FindCommandItem(LegacyCommandId.EditEnablePlayerBot)!
                    .RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));
                await WaitFor(() => factory.Client.Configurations.Count == 2 && session.LatestSnapshot.PlayerBot.Enabled);
                Assert.Equal(focus, session.LatestSnapshot.PlayerBot.Focus);

                viewport.Focus();
                window.KeyPress(Key.W, RawInputModifiers.None, PhysicalKey.W, null);
                await WaitFor(() => session.LatestSnapshot.PlayerBot.PressedKeys.SequenceEqual([(int)Key.W]));
                Assert.Equal(0, MemoryValue(session.LatestSnapshot, 50));
                Assert.Equal("PlayerBotKey:7:3:W:True", factory.Client.LastKeyCommand);

                await session.StepAsync(1);
                await WaitFor(() => MemoryValue(session.LatestSnapshot, 50) == 123);
                Assert.Equal("Tick:1", factory.Client.LastCommand);

                window.KeyRelease(Key.W, RawInputModifiers.None, PhysicalKey.W, null);
                await WaitFor(() => session.LatestSnapshot.PlayerBot.PressedKeys.Count == 0);
                Assert.Equal("PlayerBotKey:7:3:W:False", factory.Client.LastKeyCommand);
                await session.StepAsync(1);
                await WaitFor(() => session.LatestSnapshot.Tick == 2);
                Assert.Equal(123, MemoryValue(session.LatestSnapshot, 50));

                window.KeyPress(Key.Space, RawInputModifiers.None, PhysicalKey.Space, null);
                await WaitFor(() => session.LatestSnapshot.PlayerBot.PressedKeys.SequenceEqual([(int)Key.Space]));
                await session.StepAsync(1);
                await WaitFor(() => MemoryValue(session.LatestSnapshot, 51) == -77);
                Assert.Equal(-77, MemoryValue(session.LatestSnapshot, 51));
                window.KeyRelease(Key.Space, RawInputModifiers.None, PhysicalKey.Space, null);
                await WaitFor(() => session.LatestSnapshot.PlayerBot.PressedKeys.Count == 0);
                await session.StepAsync(1);
                await WaitFor(() => session.LatestSnapshot.Tick == 4);
                Assert.Equal(-77, MemoryValue(session.LatestSnapshot, 51));
            }
            finally
            {
                foreach (var ownedWindow in window.OwnedWindows.ToArray()) ownedWindow.Close();
                window.Close();
            }
        }, CancellationToken.None);
    }

    private static void CaptureAndAdd(
        PlayerBotSettingsWindow dialog,
        Button capture,
        Button add,
        TextBox[] fields,
        CheckBox invert,
        Key key,
        PhysicalKey physicalKey,
        ushort memory,
        int value,
        bool inverted)
    {
        invert.IsChecked = inverted;
        capture.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
        dialog.KeyPress(key, RawInputModifiers.None, physicalKey, null);
        dialog.KeyRelease(key, RawInputModifiers.None, physicalKey, null);
        Assert.Equal(((int)key).ToString(CultureInfo.InvariantCulture), fields[0].Text);
        fields[1].Text = memory.ToString(CultureInfo.InvariantCulture);
        fields[2].Text = value.ToString(CultureInfo.InvariantCulture);
        add.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
    }

    private static void AssertKeyMapSurface(PlayerBotSettingsWindow dialog, string statusText)
    {
        var surfaceText = string.Join("\n", dialog.GetVisualDescendants().OfType<TextBlock>().Select(block => block.Text));
        Assert.Contains("KEY MAP", surfaceText);
        Assert.Contains(statusText, surfaceText);
        AvaloniaHeadlessPlatform.ForceRenderTimerTick();
        Assert.True(dialog.Bounds.Width >= 520, $"Rendered width was {dialog.Bounds.Width}.");
        Assert.True(dialog.Bounds.Height >= 420, $"Rendered height was {dialog.Bounds.Height}.");
        using var frame = Assert.IsAssignableFrom<Bitmap>(dialog.CaptureRenderedFrame());
        Assert.True(frame.PixelSize.Width >= 520);
        Assert.True(frame.PixelSize.Height >= 420);

        var rect = new PixelRect(0, 0, frame.PixelSize.Width, frame.PixelSize.Height);
        var stride = rect.Width * 4;
        var bytes = new byte[stride * rect.Height];
        var pointer = Marshal.AllocHGlobal(bytes.Length);
        try
        {
            frame.CopyPixels(rect, pointer, bytes.Length, stride);
            Marshal.Copy(pointer, bytes, 0, bytes.Length);
        }
        finally
        {
            Marshal.FreeHGlobal(pointer);
        }

        Assert.True(ContainsApproximateColor(bytes, 0xF4, 0xF0, 0xE6));
        Assert.True(ContainsApproximateColor(bytes, 0x41, 0x6E, 0x35));
    }

    private static bool ContainsApproximateColor(byte[] pixels, byte red, byte green, byte blue)
    {
        for (var index = 0; index + 3 < pixels.Length; index += 4)
        {
            var bgra = Close(pixels[index], blue) && Close(pixels[index + 1], green) && Close(pixels[index + 2], red);
            var rgba = Close(pixels[index], red) && Close(pixels[index + 1], green) && Close(pixels[index + 2], blue);
            if (bgra || rgba) return true;
        }
        return false;

        static bool Close(byte left, byte right) => Math.Abs(left - right) <= 24;
    }

    private static int MemoryValue(EngineSnapshot snapshot, ushort address) =>
        snapshot.Organisms.Single().Memory.Single(cell => cell.Address == address).Value;

    private static T Field<T>(object target, string name) =>
        (T)target.GetType().GetField(name, BindingFlags.Instance | BindingFlags.NonPublic)!.GetValue(target)!;

    private static T? TryField<T>(object target, string name) where T : class =>
        target.GetType().GetField(name, BindingFlags.Instance | BindingFlags.NonPublic)?.GetValue(target) as T;

    private static async Task WaitFor(Func<bool> condition)
    {
        var deadline = DateTime.UtcNow + TimeSpan.FromSeconds(8);
        while (!condition() && DateTime.UtcNow < deadline) await Task.Delay(10);
        Assert.True(condition(), "Timed out waiting for PlayerBot UI or authoritative cycle state.");
    }
}

internal sealed class PlayerBotTestEngineFactory : IEngineClientFactory
{
    public PlayerBotAuthorityEngineClient Client { get; } = new();

    public IEngineClient Create(WorldSetupOptions setup) => Client;
}

internal sealed class PlayerBotAuthorityEngineClient : IEngineClient
{
    private readonly OrganismKey _focus = new(7, 3);
    private readonly HashSet<int> _pressed = [];
    private EngineSnapshot _snapshot;
    private PlayerBotConfig _config = PlayerBotConfig.Disabled;
    private ulong _tick;

    public PlayerBotAuthorityEngineClient()
    {
        var organism = new OrganismSnapshot(7, 3, [100f, 200f], [0f, 0f], 1_000, 10, Species: 1)
        {
            Memory = [new MemoryCellSnapshot(50, 0), new MemoryCellSnapshot(51, 0)],
        };
        _snapshot = new EngineSnapshot(0, 1, "CPU", [organism])
        {
            Species = [
                new SpeciesSnapshot("Unassigned", false, 0xff858982, 0, false),
                new SpeciesSnapshot("Player Species", false, 0xff239ac0, 0, false),
            ],
        };
    }

    public OrganismKey Focus => _focus;
    public ConcurrentQueue<PlayerBotConfig> Configurations { get; } = [];
    public ConcurrentQueue<string> Commands { get; } = [];
    public string LastCommand => Commands.Last();
    public string LastKeyCommand => Commands.Last(command => command.StartsWith("PlayerBotKey:", StringComparison.Ordinal));
    public string Backend => "CPU";

    public EngineCapabilities Capabilities() => new(1, Backend, true, null);

    public void Tick(uint count = 1)
    {
        Commands.Enqueue($"Tick:{count}");
        for (var cycle = 0u; cycle < count; cycle++)
        {
            _tick++;
            var organism = _snapshot.Organisms.Single();
            var memory = organism.Memory.ToDictionary(cell => cell.Address, cell => cell.Value);
            foreach (var binding in _config.Bindings.Where(binding => _pressed.Contains(binding.KeyCode)))
                memory[binding.MemoryAddress] = binding.Value;
            _snapshot = _snapshot with
            {
                Tick = _tick,
                Organisms = [organism with
                {
                    Memory = memory.OrderBy(cell => cell.Key).Select(cell => new MemoryCellSnapshot(cell.Key, cell.Value)).ToArray(),
                }],
                PlayerBot = CurrentPlayerBot(),
            };
        }
    }

    public void ConfigurePlayerBot(PlayerBotConfig config)
    {
        _config = config;
        Configurations.Enqueue(config);
        Commands.Enqueue($"ConfigurePlayerBot:{config.Enabled}");
        _snapshot = _snapshot with { PlayerBot = CurrentPlayerBot() };
    }

    public void SetPlayerBotKey(uint slot, uint generation, int keyCode, bool pressed)
    {
        if (pressed) _pressed.Add(keyCode);
        else _pressed.Remove(keyCode);
        Commands.Enqueue($"PlayerBotKey:{slot}:{generation}:{(Key)keyCode}:{pressed}");
        _snapshot = _snapshot with { PlayerBot = CurrentPlayerBot() };
    }

    private PlayerBotSnapshot CurrentPlayerBot() =>
        new(_config.Enabled, _config.Focus, _config.Bindings.ToArray(), _pressed.OrderBy(key => key).ToArray());

    public DnaImportReport ImportDna(string dna, float[]? position = null) => DnaImportReport.Compatible;
    public DnaImportReport ImportSpecies(SpeciesImport species) => DnaImportReport.Compatible;
    public DnaImportReport InsertSpeciesAt(SpeciesImport species, float[] position) => DnaImportReport.Compatible;
    public void Remove(uint slot, uint generation) { }
    public void RemoveMany(IReadOnlyList<OrganismKey> organisms) { }
    public void Move(uint slot, uint generation, float[] position) { }
    public OrganismKey CloneOrganism(uint slot, uint generation, float[] position) => new(100, 0);
    public void ReplaceDna(uint slot, uint generation, string dna) { }
    public string ExportDna(uint slot, uint generation) => string.Empty;
    public OrganismKey Reproduce(uint firstSlot, uint firstGeneration, uint? secondSlot, uint? secondGeneration, float[] position) => new(101, 0);
    public void SwitchBackend(string backend) { }
    public void AddObstacle(ObstacleSnapshot obstacle) { }
    public void UpdateObstacle(ObstacleSnapshot obstacle) { }
    public void RemoveObstacle(uint id) { }
    public void AddTeleporter(TeleporterSnapshot teleporter) { }
    public void UpdateTeleporter(TeleporterSnapshot teleporter) { }
    public void RemoveTeleporter(uint id) { }
    public void AddFileTeleporter(FileTeleporterSnapshot teleporter) { }
    public void UpdateFileTeleporter(FileTeleporterSnapshot teleporter) { }
    public void RemoveFileTeleporter(uint id) { }
    public void SetBrownianMotion(float value) { }
    public void UpdateMutationSettings(MutationSettingsUpdate update) { }
    public void SetTiesEnabled(bool enabled) { }
    public void SetSpeciesColor(uint species, uint color) { }
    public void MakeNewSpecies(uint slot, uint generation, string name, uint color) { }
    public void UpdateLegacyRestrictions(bool robotFixingAllowed, bool asexualReproductionAllowed) { }
    public void SetOrganismMemory(uint slot, uint generation, ushort address, int value) { }
    public void SetOrganismEnergy(uint slot, uint generation, int energy) { }
    public void UpdateRestrictionOptions(RestrictionOptions options) { }
    public void ConfigureLocalMode(LocalModeConfig config) { }
    public void StartLocalMode() { }
    public void RestartLocalMode() { }
    public void AdjudicateLocalRound(uint? winner, string reason) { }
    public void SetOrganismAim(uint slot, uint generation, int aim) { }
    public void UpdateEnvironment(EnvironmentUpdate update) { }
    public byte[] Save() => [];
    public byte[] SaveWithoutMutations() => [];
    public void Load(byte[] save) { }
    public EngineSnapshot Snapshot() => _snapshot with { Tick = _tick };
    public void Dispose() { }
}
