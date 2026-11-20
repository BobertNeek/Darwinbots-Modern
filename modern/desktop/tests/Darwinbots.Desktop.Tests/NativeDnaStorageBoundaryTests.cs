using Darwinbots.Desktop.Core;
using Darwinbots.Desktop.Services;
using Xunit;

namespace Darwinbots.Desktop.Tests;

public sealed class NativeDnaStorageBoundaryTests
{
    [Fact]
    public async Task DefaultStorageWritesNativeBytesAndRejectsInvalidTextBeforeOpeningTheStream()
    {
        using var destination = new MemoryStream();
        destination.Write(new byte[] { 1, 2, 3 });
        var opens = 0;
        Func<Task<Stream>> open = () => { opens++; return Task.FromResult<Stream>(destination); };
        const string dna = "' é €\r\nstart stop\n";
        await AvaloniaDesktopStorageService.WriteDnaFileAsync(null, open, dna, null);
        Assert.Equal(new byte[] { 39, 32, 0xe9, 32, 0x80, 13, 10, 115, 116, 97, 114, 116, 32, 115, 116, 111, 112, 10 }, destination.ToArray());
        Assert.Equal(1, opens);

        using var unchanged = new MemoryStream();
        unchanged.Write(new byte[] { 4, 5, 6 });
        var invalidOpens = 0;
        Func<Task<Stream>> invalidOpen = () => { invalidOpens++; return Task.FromResult<Stream>(unchanged); };
        await Assert.ThrowsAsync<InvalidDataException>(() =>
            AvaloniaDesktopStorageService.WriteDnaFileAsync(null, invalidOpen, "' 🙂\r\nstart stop", null));
        Assert.Equal(0, invalidOpens);
        Assert.Equal(new byte[] { 4, 5, 6 }, unchanged.ToArray());
    }

    [Fact]
    public async Task NonlocalDestinationsCannotSilentlyDropTheMutationSidecar()
    {
        using var destination = new MemoryStream();
        destination.Write(new byte[] { 4, 5, 6 });
        var opens = 0;
        Func<Task<Stream>> open = () => { opens++; return Task.FromResult<Stream>(destination); };
        await Assert.ThrowsAsync<NotSupportedException>(() =>
            AvaloniaDesktopStorageService.WriteDnaFileAsync(null, open, "start stop", MutationProfileUpdate.Default));
        Assert.Equal(0, opens);
        Assert.Equal(new byte[] { 4, 5, 6 }, destination.ToArray());
    }

    [Fact]
    public async Task DefaultLocalStorageWritesBothFilesAndValidatesBeforeChangingExistingFiles()
    {
        var directory = Path.Combine(Path.GetTempPath(), "db-native-storage-" + Guid.NewGuid().ToString("N"));
        try
        {
            Directory.CreateDirectory(directory);
            var path = Path.Combine(directory, "Café.txt");
            var opens = 0;
            Func<Task<Stream>> open = () =>
            {
                opens++;
                return Task.FromResult<Stream>(new FileStream(path, FileMode.OpenOrCreate, FileAccess.Write));
            };
            const string raw = "' é\nstart stop\r\n";
            var profile = MutationProfileUpdate.Default with { Point = new(123.5, 7, 0.5), CopyErrorValueChangePercent = 17 };
            await AvaloniaDesktopStorageService.WriteDnaFileAsync(path, open, raw, profile);
            Assert.Equal(raw, LegacyDnaText.Decode(await File.ReadAllBytesAsync(path)));
            Assert.Equal(profile, Db2MutationRateCodec.Decode(await File.ReadAllBytesAsync(Path.ChangeExtension(path, ".mrate"))));
            Assert.Equal(0, opens);
            var dnaBefore = await File.ReadAllBytesAsync(path);
            var profileBefore = await File.ReadAllBytesAsync(Path.ChangeExtension(path, ".mrate"));
            await Assert.ThrowsAsync<InvalidDataException>(() =>
                AvaloniaDesktopStorageService.WriteDnaFileAsync(path, open, "start stop", profile with { PointValueChangePercent = 101 }));
            await Assert.ThrowsAsync<InvalidDataException>(() =>
                AvaloniaDesktopStorageService.WriteDnaFileAsync(path, open, "' 🙂\nstart stop", profile));
            Assert.Equal(dnaBefore, await File.ReadAllBytesAsync(path));
            Assert.Equal(profileBefore, await File.ReadAllBytesAsync(Path.ChangeExtension(path, ".mrate")));
            Assert.Equal(0, opens);
        }
        finally { if (Directory.Exists(directory)) Directory.Delete(directory, true); }
    }

    [Fact]
    public async Task CancellationIsObservedBeforeOpeningOrTruncatingTheDestination()
    {
        using var destination = new MemoryStream();
        destination.Write(new byte[] { 4, 5, 6 });
        var opens = 0;
        Func<Task<Stream>> open = () => { opens++; return Task.FromResult<Stream>(destination); };
        await Assert.ThrowsAnyAsync<OperationCanceledException>(() =>
            AvaloniaDesktopStorageService.WriteDnaFileAsync(null, open, "start stop", null, new CancellationToken(true)));
        Assert.Equal(0, opens);
        Assert.Equal(new byte[] { 4, 5, 6 }, destination.ToArray());
    }

    [Fact]
    public async Task NullArgumentsAreRejectedWithoutOpeningTheDestination()
    {
        await Assert.ThrowsAsync<ArgumentNullException>(() =>
            AvaloniaDesktopStorageService.WriteDnaFileAsync((Avalonia.Platform.Storage.IStorageFile)null!, "start stop", null));
        await Assert.ThrowsAsync<ArgumentNullException>(() =>
            AvaloniaDesktopStorageService.WriteDnaFileAsync(null, null!, "start stop", null));
        var opens = 0;
        Func<Task<Stream>> open = () => { opens++; return Task.FromResult<Stream>(new MemoryStream()); };
        await Assert.ThrowsAsync<ArgumentNullException>(() =>
            AvaloniaDesktopStorageService.WriteDnaFileAsync(null, open, null!, null));
        Assert.Equal(0, opens);
    }
}
