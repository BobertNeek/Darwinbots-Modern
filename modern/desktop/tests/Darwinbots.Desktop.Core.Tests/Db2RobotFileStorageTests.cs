using System.Text;
using Darwinbots.Desktop.Core;
using Xunit;

namespace Darwinbots.Desktop.Core.Tests;

public sealed class Db2RobotFileStorageTests
{
    private const string Dna = "'café €\r\ncond\n 1 .up store\rstop\r\n";
    private const string OriginalRates = "13\r\n27\r\n"
        + "101.25\r\n1.5\r\n0.25\r\n"
        + "202.5\r\n-2.5\r\n0.5\r\n"
        + "303.75\r\n3.5\r\n0.75\r\n"
        + "404\r\n4.5\r\n1\r\n"
        + "505.25\r\n5.5\r\n1.25\r\n"
        + "606.5\r\n6.5\r\n1.5\r\n"
        + "707.75\r\n7.5\r\n1.75\r\n"
        + "808\r\n8.5\r\n2\r\n"
        + "909.25\r\n9.5\r\n2.25\r\n"
        + "1010.5\r\n10.5\r\n2.5\r\n"
        + "1111.75\r\n11.5\r\n2.75\r\n";

    [Fact]
    public async Task LiteralOriginalFilesKeepCp1252BytesMixedDnaNewlinesAndAllSidecarFields()
    {
        var directory = TemporaryDirectory();
        var source = Path.Combine(directory, "original.txt");
        var target = Path.Combine(directory, "saved.txt");
        byte[] rawDna = [.. Encoding.ASCII.GetBytes("'caf"), 0xE9, 0x20, 0x80,
            .. Encoding.ASCII.GetBytes("\r\ncond\n 1 .up store\rstop\r\n")];
        var rates = Encoding.ASCII.GetBytes(OriginalRates);
        try
        {
            await File.WriteAllBytesAsync(source, rawDna);
            await File.WriteAllBytesAsync(Path.ChangeExtension(source, ".mrate"), rates);

            var loaded = await Db2RobotFileStorage.ReadAsync(source);
            Assert.Equal(Dna, loaded.Dna);
            Assert.Equal(AsymmetricProfile(), loaded.MutationProfile);
            await Db2RobotFileStorage.WriteAsync(target, loaded.Dna, loaded.MutationProfile);

            Assert.Equal(rawDna, await File.ReadAllBytesAsync(target));
            Assert.Equal(rates, await File.ReadAllBytesAsync(Path.ChangeExtension(target, ".mrate")));
            Assert.Equal(loaded, await Db2RobotFileStorage.ReadAsync(target));
            Assert.Equal(4, Directory.GetFiles(directory).Length);
        }
        finally { Directory.Delete(directory, true); }
    }

    [Fact]
    public async Task MissingSidecarIsNullExplicitDefaultIsPresentAndDnaOnlySaveLeavesItIntact()
    {
        var directory = TemporaryDirectory();
        var path = Path.Combine(directory, "robot.txt");
        var sidecar = Path.ChangeExtension(path, ".mrate");
        try
        {
            await Db2RobotFileStorage.WriteAsync(path, Dna);
            Assert.False(File.Exists(sidecar));
            Assert.Null((await Db2RobotFileStorage.ReadAsync(path)).MutationProfile);
            Assert.Null(await Db2RobotFileStorage.ReadMutationProfileAsync(path));

            await Db2RobotFileStorage.WriteAsync(path, Dna, MutationProfileUpdate.Default);
            Assert.Equal(MutationProfileUpdate.Default,
                (await Db2RobotFileStorage.ReadAsync(path)).MutationProfile);
            var originalSidecar = await File.ReadAllBytesAsync(sidecar);

            await Db2RobotFileStorage.WriteAsync(path, "'changed\r\n" + Dna);
            Assert.Equal(originalSidecar, await File.ReadAllBytesAsync(sidecar));
            var loaded = await Db2RobotFileStorage.ReadAsync(path);
            Assert.Equal("'changed\r\n" + Dna, loaded.Dna);
            Assert.Equal(MutationProfileUpdate.Default, loaded.MutationProfile);
            Assert.Equal(2, Directory.GetFiles(directory).Length);
        }
        finally { Directory.Delete(directory, true); }
    }

    [Fact]
    public async Task ProfileOnlyReadFindsUniqueCaseInsensitiveSidecarWithoutOpeningDna()
    {
        var directory = TemporaryDirectory();
        var path = Path.Combine(directory, "Robot.TXT");
        var sidecar = Path.Combine(directory, "rOBOT.MRATE");
        try
        {
            await File.WriteAllBytesAsync(sidecar, Encoding.ASCII.GetBytes(OriginalRates));

            Assert.Equal(AsymmetricProfile(), await Db2RobotFileStorage.ReadMutationProfileAsync(path));
            Assert.False(File.Exists(path));
            await Db2RobotFileStorage.WriteAsync(path, Dna, MutationProfileUpdate.Default);

            Assert.Equal(MutationProfileUpdate.Default,
                Db2MutationRateCodec.Decode(await File.ReadAllBytesAsync(sidecar)));
            Assert.Equal(2, Directory.GetFiles(directory).Length);
            Assert.Contains("rOBOT.MRATE", Directory.GetFiles(directory).Select(Path.GetFileName));
        }
        finally { Directory.Delete(directory, true); }
    }

    [Fact]
    public async Task CaseAmbiguityIsRejectedWhenTheFilesystemCanRepresentDistinctSidecars()
    {
        var directory = TemporaryDirectory();
        var path = Path.Combine(directory, "robot.txt");
        var lower = Path.Combine(directory, "robot.mrate");
        var upper = Path.Combine(directory, "ROBOT.MRATE");
        var originalDna = LegacyDnaText.Encode(Dna);
        var firstRates = Encoding.ASCII.GetBytes(OriginalRates);
        var secondRates = Db2MutationRateCodec.Encode(MutationProfileUpdate.Default);
        try
        {
            await File.WriteAllBytesAsync(path, originalDna);
            await File.WriteAllBytesAsync(lower, firstRates);
            await File.WriteAllBytesAsync(upper, secondRates);
            var sidecars = Directory.GetFiles(directory)
                .Count(file => string.Equals(Path.GetExtension(file), ".mrate", StringComparison.OrdinalIgnoreCase));
            if (sidecars == 2)
            {
                await Assert.ThrowsAsync<IOException>(() => Db2RobotFileStorage.ReadMutationProfileAsync(path));
                await Assert.ThrowsAsync<IOException>(() =>
                    Db2RobotFileStorage.WriteAsync(path, "'replacement", AsymmetricProfile()));
                Assert.Equal(originalDna, await File.ReadAllBytesAsync(path));
                Assert.Equal(firstRates, await File.ReadAllBytesAsync(lower));
                Assert.Equal(secondRates, await File.ReadAllBytesAsync(upper));
                Assert.Equal(3, Directory.GetFiles(directory).Length);
            }
            else
            {
                // Case-insensitive volumes have one file, so no ambiguity exists.
                Assert.Equal(1, sidecars);
                Assert.Equal(MutationProfileUpdate.Default,
                    await Db2RobotFileStorage.ReadMutationProfileAsync(path));
                await Db2RobotFileStorage.WriteAsync(path, Dna, AsymmetricProfile());
                Assert.Equal(AsymmetricProfile(), (await Db2RobotFileStorage.ReadAsync(path)).MutationProfile);
                Assert.Equal(2, Directory.GetFiles(directory).Length);
            }
        }
        finally { Directory.Delete(directory, true); }
    }

    [Theory]
    [InlineData(false, false)]
    [InlineData(false, true)]
    [InlineData(true, false)]
    [InlineData(true, true)]
    public async Task InvalidDnaOrProfileDoesNotCreateOrChangeEitherTarget(bool existing, bool invalidProfile)
    {
        var directory = TemporaryDirectory();
        var path = Path.Combine(directory, "robot.txt");
        var sidecar = Path.ChangeExtension(path, ".mrate");
        byte[] oldDna = [0xE9, 0x80, 0x0D, 0x0A];
        byte[] oldRates = [0x00, 0x80, 0xE9];
        try
        {
            if (existing)
            {
                await File.WriteAllBytesAsync(path, oldDna);
                await File.WriteAllBytesAsync(sidecar, oldRates);
            }
            var dna = invalidProfile ? Dna : "'unrepresentable \u2603";
            var profile = invalidProfile
                ? AsymmetricProfile() with { CopyError2 = new(1, 0, double.NaN) }
                : AsymmetricProfile();

            await Assert.ThrowsAsync<InvalidDataException>(() =>
                Db2RobotFileStorage.WriteAsync(path, dna, profile));

            Assert.Equal(existing, File.Exists(path));
            Assert.Equal(existing, File.Exists(sidecar));
            if (existing)
            {
                Assert.Equal(oldDna, await File.ReadAllBytesAsync(path));
                Assert.Equal(oldRates, await File.ReadAllBytesAsync(sidecar));
            }
            Assert.Equal(existing ? 2 : 0, Directory.GetFiles(directory).Length);
        }
        finally { Directory.Delete(directory, true); }
    }

    [Fact]
    public async Task SidecarDirectoryIsRejectedBeforeDnaReplacementOrStaging()
    {
        var directory = TemporaryDirectory();
        var path = Path.Combine(directory, "robot.txt");
        var sidecar = Path.ChangeExtension(path, ".mrate");
        var original = LegacyDnaText.Encode(Dna);
        try
        {
            await File.WriteAllBytesAsync(path, original);
            Directory.CreateDirectory(sidecar);

            await Assert.ThrowsAsync<IOException>(() =>
                Db2RobotFileStorage.WriteAsync(path, "'replacement", AsymmetricProfile()));
            await Assert.ThrowsAsync<IOException>(() => Db2RobotFileStorage.ReadMutationProfileAsync(path));

            Assert.Equal(original, await File.ReadAllBytesAsync(path));
            Assert.True(Directory.Exists(sidecar));
            Assert.Equal(2, Directory.GetFileSystemEntries(directory).Length);
        }
        finally { Directory.Delete(directory, true); }
    }

    [Theory]
    [InlineData(false)]
    [InlineData(true)]
    public async Task DnaAndSidecarCannotNameTheSameWindowsFile(bool existing)
    {
        var directory = TemporaryDirectory();
        var path = Path.Combine(directory, "robot.MRATE");
        byte[] original = [0xE9, 0x80];
        try
        {
            if (existing) await File.WriteAllBytesAsync(path, original);

            await Assert.ThrowsAsync<ArgumentException>(() =>
                Db2RobotFileStorage.WriteAsync(path, Dna, AsymmetricProfile()));

            Assert.Equal(existing, File.Exists(path));
            if (existing) Assert.Equal(original, await File.ReadAllBytesAsync(path));
            Assert.Equal(existing ? 1 : 0, Directory.GetFiles(directory).Length);
        }
        finally { Directory.Delete(directory, true); }
    }

    [Fact]
    public async Task MalformedPresentSidecarRejectsWholeRobotAndProfileOnlyImports()
    {
        var directory = TemporaryDirectory();
        var path = Path.Combine(directory, "robot.txt");
        var sidecar = Path.ChangeExtension(path, ".mrate");
        var original = LegacyDnaText.Encode(Dna);
        var malformed = Encoding.ASCII.GetBytes("13\r\n27\r\n");
        try
        {
            await File.WriteAllBytesAsync(path, original);
            await File.WriteAllBytesAsync(sidecar, malformed);

            await Assert.ThrowsAsync<InvalidDataException>(() => Db2RobotFileStorage.ReadAsync(path));
            await Assert.ThrowsAsync<InvalidDataException>(() => Db2RobotFileStorage.ReadMutationProfileAsync(path));
            Assert.Equal(original, await File.ReadAllBytesAsync(path));
            Assert.Equal(malformed, await File.ReadAllBytesAsync(sidecar));
        }
        finally { Directory.Delete(directory, true); }
    }

    [Theory]
    [InlineData(false)]
    [InlineData(true)]
    public async Task CanceledWriteLeavesBothTargetsUntouched(bool existing)
    {
        var directory = TemporaryDirectory();
        var path = Path.Combine(directory, "robot.txt");
        var sidecar = Path.ChangeExtension(path, ".mrate");
        var original = LegacyDnaText.Encode(Dna);
        var rates = Encoding.ASCII.GetBytes(OriginalRates);
        try
        {
            if (existing)
            {
                await File.WriteAllBytesAsync(path, original);
                await File.WriteAllBytesAsync(sidecar, rates);
            }
            using var canceled = new CancellationTokenSource();
            canceled.Cancel();

            await Assert.ThrowsAnyAsync<OperationCanceledException>(() =>
                Db2RobotFileStorage.WriteAsync(path, "'replacement", MutationProfileUpdate.Default, canceled.Token));

            Assert.Equal(existing, File.Exists(path));
            Assert.Equal(existing, File.Exists(sidecar));
            if (existing)
            {
                Assert.Equal(original, await File.ReadAllBytesAsync(path));
                Assert.Equal(rates, await File.ReadAllBytesAsync(sidecar));
            }
            Assert.Equal(existing ? 2 : 0, Directory.GetFiles(directory).Length);
        }
        finally { Directory.Delete(directory, true); }
    }

    [Fact]
    public async Task SharedSidecarReadHandleExercisesWindowsSecondReplaceRollbackAndUnixReplacement()
    {
        var directory = TemporaryDirectory();
        var path = Path.Combine(directory, "robot.txt");
        var sidecar = Path.ChangeExtension(path, ".mrate");
        var original = LegacyDnaText.Encode(Dna);
        var rates = Encoding.ASCII.GetBytes(OriginalRates);
        const string replacement = "'replacement café\r\nstart 2 .up store stop\r\n";
        try
        {
            await File.WriteAllBytesAsync(path, original);
            await File.WriteAllBytesAsync(sidecar, rates);
            await using (var reader = new FileStream(sidecar, FileMode.Open, FileAccess.Read, FileShare.Read))
            {
                if (OperatingSystem.IsWindows())
                {
                    // Backup reads are allowed; replacing this second target is denied.
                    await Assert.ThrowsAsync<IOException>(() =>
                        Db2RobotFileStorage.WriteAsync(path, replacement, MutationProfileUpdate.Default));
                    Assert.Equal(original, await File.ReadAllBytesAsync(path));
                    Assert.Equal(rates, await File.ReadAllBytesAsync(sidecar));
                }
                else
                {
                    // POSIX permits rename over an open file; the old handle still sees old bytes.
                    await Db2RobotFileStorage.WriteAsync(path, replacement, MutationProfileUpdate.Default);
                    Assert.Equal(replacement, (await Db2RobotFileStorage.ReadAsync(path)).Dna);
                    Assert.Equal(MutationProfileUpdate.Default,
                        await Db2RobotFileStorage.ReadMutationProfileAsync(path));
                }
                using var retained = new MemoryStream();
                await reader.CopyToAsync(retained);
                Assert.Equal(rates, retained.ToArray());
            }
            Assert.Equal(2, Directory.GetFiles(directory).Length);
        }
        finally { Directory.Delete(directory, true); }
    }

    private static string TemporaryDirectory()
    {
        var directory = Path.Combine(Path.GetTempPath(), $"darwinbots-robot-file-{Guid.NewGuid():N}");
        Directory.CreateDirectory(directory);
        return directory;
    }

    private static MutationProfileUpdate AsymmetricProfile() => MutationProfileUpdate.Default with
    {
        PointValueChangePercent = 13, CopyErrorValueChangePercent = 27,
        Point = new(101.25, 1.5, 0.25),
        MinorDeletion = new(202.5, -2.5, 0.5),
        Reversal = new(303.75, 3.5, 0.75),
        Insertion = new(404, 4.5, 1),
        Amplification = new(505.25, 5.5, 1.25),
        MajorDeletion = new(606.5, 6.5, 1.5),
        CopyError = new(707.75, 7.5, 1.75),
        Delta = new(808, 8.5, 2),
        Translocation = new(909.25, 9.5, 2.25),
        Point2 = new(1010.5, 10.5, 2.5),
        CopyError2 = new(1111.75, 11.5, 2.75),
    };
}
