namespace Darwinbots.Desktop.Core;

public sealed record RobotConsoleCommand(string Name, IReadOnlyList<string> Arguments)
{
    public static RobotConsoleCommand Parse(string input)
    {
        ArgumentException.ThrowIfNullOrWhiteSpace(input);
        var parts = input.Split((char[]?)null, StringSplitOptions.RemoveEmptyEntries | StringSplitOptions.TrimEntries);
        return new RobotConsoleCommand(parts[0].ToLowerInvariant(), parts[1..]);
    }
}

public sealed class RobotConsoleController
{
    private readonly SimulationSession _session;
    private readonly OrganismKey _focus;
    private readonly Func<Task>? _play;
    private readonly Func<Task>? _pause;

    public RobotConsoleController(
        SimulationSession session,
        OrganismKey focus,
        Func<Task>? play = null,
        Func<Task>? pause = null)
    {
        _session = session;
        _focus = focus;
        _play = play;
        _pause = pause;
    }

    public async Task<string> ExecuteAsync(string input, CancellationToken cancellationToken = default)
    {
        var command = RobotConsoleCommand.Parse(input);
        switch (command.Name)
        {
            case "help":
                return "help | printmem [address or .sysvar] | set address-or-.sysvar value | energy value | cycle [count] | step | execrob | printeye | printtouch | printtaste | debug | showdna | play | pause";
            case "cycle":
            case "step":
                var count = command.Name == "step" ? 1u : ParseCount(command.Arguments, 1);
                await _session.StepAsync(count, cancellationToken).ConfigureAwait(false);
                return $"cycle {_session.LatestSnapshot.Tick} (+{count})";
            case "play":
                if (_play is null) return "play is not available from this console";
                await _play().ConfigureAwait(false);
                return "play";
            case "pause":
                if (_pause is null) return "pause is not available from this console";
                await _pause().ConfigureAwait(false);
                return "pause";
            case "set":
                RequireArguments(command, 2);
                var address = await _session.ResolveMemoryAddressAsync(command.Arguments[0], cancellationToken).ConfigureAwait(false);
                var value = ParseInt(command.Arguments[1], "value");
                await _session.SetOrganismMemoryAsync(_focus.Slot, _focus.Generation, address, value, cancellationToken)
                    .ConfigureAwait(false);
                return $"mem[{address}] = {value}";
            case "energy":
                RequireArguments(command, 1);
                var energy = ParseInt(command.Arguments[0], "energy");
                await _session.SetOrganismEnergyAsync(_focus.Slot, _focus.Generation, energy, cancellationToken)
                    .ConfigureAwait(false);
                return $"energy = {energy}";
            case "printmem":
            case "?":
                if (command.Arguments.Count == 0) return PrintMemory(command.Arguments);
                var resolved = await _session.ResolveMemoryAddressAsync(command.Arguments[0], cancellationToken).ConfigureAwait(false);
                return PrintMemory([resolved.ToString()]);
            case "execrob":
                await _session.ExecuteRobotsAsync(cancellationToken).ConfigureAwait(false);
                return $"DNA executed at cycle {_session.LatestSnapshot.Tick}";
            case "printeye":
            case "eyes":
                return PrintEyes();
            case "printtouch":
                return $"touch = {CurrentOrganism().Touch}";
            case "printtaste":
                return $"taste = {CurrentOrganism().Taste}";
            case "debug":
                return PrintDebug();
            case "showdna":
                return await _session.ExportDnaAsync(_focus.Slot, _focus.Generation, cancellationToken)
                    .ConfigureAwait(false);
            default:
                return $"unknown command '{command.Name}'. Type help.";
        }
    }

    private string PrintMemory(IReadOnlyList<string> arguments)
    {
        var organism = CurrentOrganism();
        if (arguments.Count > 0)
        {
            var address = ParseAddress(arguments[0]);
            var cell = organism.Memory.FirstOrDefault(cell => cell.Address == address);
            return $"mem[{address}] = {cell?.Value ?? 0}";
        }
        return string.Join(Environment.NewLine, organism.Memory.Select(cell => $"mem[{cell.Address}] = {cell.Value}"));
    }

    private string PrintEyes()
    {
        var vision = CurrentOrganism().Vision;
        var lines = new List<string> { $"focus eye: {vision.FocusEye}" };
        for (var index = 0; index < vision.Eyes.Length; index++)
        {
            var eye = vision.Eyes[index];
            lines.Add($"eye {index}: dir={eye.Direction} width={eye.Width} range={eye.Range:0.##} value={eye.Value}");
        }
        return string.Join(Environment.NewLine, lines);
    }

    private string PrintDebug()
    {
        var organism = CurrentOrganism();
        var active = organism.ActivationBitmap.Count(value => value);
        return $"gene={organism.CurrentGene} conditions={organism.ConditionCount} active={active} history={organism.ActivationHistory.Count}{Environment.NewLine}{organism.DebugText}".TrimEnd();
    }

    private OrganismSnapshot CurrentOrganism() =>
        _session.LatestSnapshot.Organisms.FirstOrDefault(organism =>
            organism.Slot == _focus.Slot && organism.Generation == _focus.Generation)
        ?? throw new InvalidOperationException("The focused robot is no longer live.");

    private static uint ParseCount(IReadOnlyList<string> arguments, uint defaultValue)
    {
        if (arguments.Count == 0) return defaultValue;
        var value = ParseInt(arguments[0], "cycle count");
        if (value is < 1 or > 1_000_000) throw new ArgumentOutOfRangeException(nameof(arguments));
        return (uint)value;
    }

    private static ushort ParseAddress(string value)
    {
        var address = ParseInt(value, "memory address");
        if (address is < 1 or > 1000) throw new ArgumentOutOfRangeException(nameof(value));
        return (ushort)address;
    }

    private static int ParseInt(string value, string name) =>
        int.TryParse(value, out var result)
            ? result
            : throw new FormatException($"Invalid {name} '{value}'.");

    private static void RequireArguments(RobotConsoleCommand command, int count)
    {
        if (command.Arguments.Count < count)
            throw new FormatException($"{command.Name} requires {count} argument(s).");
    }
}
