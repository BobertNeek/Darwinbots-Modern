using System.Diagnostics;
using System.Reflection;
using System.Security.Cryptography;
using System.Text;
using System.Text.Json;
using Darwinbots.Desktop.Core;

if (args.Length != 2)
    throw new ArgumentException("Usage: SnapshotParseProfile <native-snapshot-utf8.json> <result.json>");
var input = Path.GetFullPath(args[0]);
var output = Path.GetFullPath(args[1]);
if (StringComparer.OrdinalIgnoreCase.Equals(input, output))
    throw new ArgumentException("The result must not overwrite the input snapshot.");
var bytes = File.ReadAllBytes(input);
var text = Encoding.UTF8.GetString(bytes);
const string backend = "CPU";
const int warmups = 5;
const int samples = 15;

var stringSnapshot = NativeSnapshotParser.Parse(text, backend);
var utf8Snapshot = NativeSnapshotParser.Parse(bytes, backend);
var stringState = JsonSerializer.SerializeToUtf8Bytes(stringSnapshot);
var utf8State = JsonSerializer.SerializeToUtf8Bytes(utf8Snapshot);
if (!stringState.AsSpan().SequenceEqual(utf8State))
    throw new InvalidDataException("UTF-8 and string parsing produced different complete snapshots.");

// Diagnostic reflection isolates phases without adding DTOs or profiler hooks
// to the production API. Type-based deserialization includes its dispatch cost.
var parser = typeof(NativeSnapshotParser);
var nativeType = parser.GetNestedType("NativeSnapshot", BindingFlags.NonPublic)!;
var options = (JsonSerializerOptions)parser.GetField("Options", BindingFlags.NonPublic | BindingFlags.Static)!.GetValue(null)!;
var project = parser.GetMethod("Project", BindingFlags.NonPublic | BindingFlags.Static)!;
var native = JsonSerializer.Deserialize(bytes, nativeType, options)!;
object[] projectionArguments = [native, backend];

Func<object?> baseline = () => NativeSnapshotParser.Parse(Encoding.UTF8.GetString(bytes), backend);
Func<object?> candidate = () => NativeSnapshotParser.Parse(bytes, backend);
Func<object?> deserializeString = () => JsonSerializer.Deserialize(text, nativeType, options);
Func<object?> deserializeUtf8 = () => JsonSerializer.Deserialize(bytes, nativeType, options);
Func<object?> projection = () => project.Invoke(null, projectionArguments);
for (var index = 0; index < warmups; index++)
{
    _ = baseline();
    _ = candidate();
    _ = deserializeString();
    _ = deserializeUtf8();
    _ = projection();
}

var baselineSamples = new List<Sample>();
var candidateSamples = new List<Sample>();
var stringSamples = new List<Sample>();
var utf8Samples = new List<Sample>();
var projectionSamples = new List<Sample>();
for (var index = 0; index < samples; index++)
{
    // Alternate order within each matched pair to reduce order/GC bias.
    if (index % 2 == 0)
    {
        baselineSamples.Add(Measure(baseline));
        candidateSamples.Add(Measure(candidate));
        stringSamples.Add(Measure(deserializeString));
        utf8Samples.Add(Measure(deserializeUtf8));
    }
    else
    {
        candidateSamples.Add(Measure(candidate));
        baselineSamples.Add(Measure(baseline));
        utf8Samples.Add(Measure(deserializeUtf8));
        stringSamples.Add(Measure(deserializeString));
    }
    projectionSamples.Add(Measure(projection));
}

var result = new
{
    scope = "Isolated managed parsing of identical preserved UTF-8 bytes; no engine ticks, UI rendering or live CPS measurement",
    input_bytes = bytes.Length,
    input_sha256 = Convert.ToHexString(SHA256.HashData(bytes)),
    complete_snapshot_bytes_equal = true,
    complete_snapshot_sha256 = Convert.ToHexString(SHA256.HashData(stringState)),
    stringSnapshot.Tick, stringSnapshot.Population, warmups, samples,
    core_dll_sha256 = Convert.ToHexString(SHA256.HashData(File.ReadAllBytes(parser.Assembly.Location))),
    runtime = System.Runtime.InteropServices.RuntimeInformation.FrameworkDescription,
    baseline_decode_and_string_parse = Summarize(baselineSamples),
    candidate_utf8_parse = Summarize(candidateSamples),
    json_string_deserialization_type_dispatch = Summarize(stringSamples),
    json_utf8_deserialization_type_dispatch = Summarize(utf8Samples),
    dto_projection_reflection = Summarize(projectionSamples),
};
File.WriteAllText(output, JsonSerializer.Serialize(result, new JsonSerializerOptions { WriteIndented = true }));
Console.WriteLine(JsonSerializer.Serialize(result));

static Sample Measure(Func<object?> action)
{
    var allocated = GC.GetAllocatedBytesForCurrentThread();
    var started = Stopwatch.GetTimestamp();
    var value = action();
    var elapsed = Stopwatch.GetElapsedTime(started).TotalMilliseconds;
    var bytes = GC.GetAllocatedBytesForCurrentThread() - allocated;
    GC.KeepAlive(value);
    return new(elapsed, bytes);
}

static object Summarize(List<Sample> values) => new
{
    median_ms = values.Select(value => value.ElapsedMs).Order().ElementAt(values.Count / 2),
    median_allocated_bytes = values.Select(value => value.AllocatedBytes).Order().ElementAt(values.Count / 2),
    samples = values,
};

sealed record Sample(double ElapsedMs, long AllocatedBytes);
