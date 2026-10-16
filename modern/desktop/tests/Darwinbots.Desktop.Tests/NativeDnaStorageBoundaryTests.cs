using Avalonia.Platform.Storage;
using Darwinbots.Desktop.Core;
using Darwinbots.Desktop.Services;
using Xunit;

namespace Darwinbots.Desktop.Tests;

public sealed class NativeDnaStorageBoundaryTests
{
    [Fact]
    public async Task DefaultStorageWritesNativeBytesAndRejectsInvalidTextBeforeOpeningTheStream()
    {
        var file = new MemoryStorageFile(new Uri("memory://robot/export.txt"), [1, 2, 3]);
        const string dna = "' é €\r\nstart stop\n";
        await AvaloniaDesktopStorageService.WriteDnaFileAsync(file, dna, null);
        Assert.Equal(new byte[] { 39, 32, 0xe9, 32, 0x80, 13, 10, 115, 116, 97, 114, 116, 32, 115, 116, 111, 112, 10 }, file.Bytes);
        Assert.Equal(1, file.OpenWrites);

        var invalid = new MemoryStorageFile(new Uri("memory://robot/unchanged.txt"), [4, 5, 6]);
        await Assert.ThrowsAsync<InvalidDataException>(() =>
            AvaloniaDesktopStorageService.WriteDnaFileAsync(invalid, "' 🙂\r\nstart stop", null));
        Assert.Equal(0, invalid.OpenWrites);
        Assert.Equal(new byte[] { 4, 5, 6 }, invalid.Bytes);
    }

    [Fact]
    public async Task NonlocalDestinationsCannotSilentlyDropTheMutationSidecar()
    {
        var file = new MemoryStorageFile(new Uri("memory://robot/export.txt"), [4, 5, 6]);
        await Assert.ThrowsAsync<NotSupportedException>(() =>
            AvaloniaDesktopStorageService.WriteDnaFileAsync(file, "start stop", MutationProfileUpdate.Default));
        Assert.Equal(0, file.OpenWrites);
        Assert.Equal(new byte[] { 4, 5, 6 }, file.Bytes);
    }

    [Fact]
    public async Task DefaultLocalStorageWritesBothFilesAndValidatesBeforeChangingExistingFiles()
    {
        var directory = Path.Combine(Path.GetTempPath(), "db-native-storage-" + Guid.NewGuid().ToString("N"));
        try
        {
            Directory.CreateDirectory(directory);
            var path = Path.Combine(directory, "Café.txt");
            var file = new MemoryStorageFile(new Uri(path), []);
            const string raw = "' é\nstart stop\r\n";
            var profile = MutationProfileUpdate.Default with { Point = new(123.5, 7, 0.5), CopyErrorValueChangePercent = 17 };
            await AvaloniaDesktopStorageService.WriteDnaFileAsync(file, raw, profile);
            Assert.Equal(raw, LegacyDnaText.Decode(await File.ReadAllBytesAsync(path)));
            Assert.Equal(profile, Db2MutationRateCodec.Decode(await File.ReadAllBytesAsync(Path.ChangeExtension(path, ".mrate"))));
            Assert.Equal(0, file.OpenWrites);
            var dnaBefore = await File.ReadAllBytesAsync(path);
            var profileBefore = await File.ReadAllBytesAsync(Path.ChangeExtension(path, ".mrate"));
            await Assert.ThrowsAsync<InvalidDataException>(() =>
                AvaloniaDesktopStorageService.WriteDnaFileAsync(file, "start stop", profile with { PointValueChangePercent = 101 }));
            await Assert.ThrowsAsync<InvalidDataException>(() =>
                AvaloniaDesktopStorageService.WriteDnaFileAsync(file, "' 🙂\nstart stop", profile));
            Assert.Equal(dnaBefore, await File.ReadAllBytesAsync(path));
            Assert.Equal(profileBefore, await File.ReadAllBytesAsync(Path.ChangeExtension(path, ".mrate")));
        }
        finally { if (Directory.Exists(directory)) Directory.Delete(directory, true); }
    }

    private sealed class MemoryStorageFile(Uri path, byte[] initial) : IStorageFile
    {
        private readonly MemoryStream _stream = Create(initial);
        public int OpenWrites { get; private set; }
        public byte[] Bytes => _stream.ToArray();
        public string Name => Path.IsFile ? System.IO.Path.GetFileName(Path.LocalPath) : "export.txt";
        public Uri Path => path;
        public bool CanBookmark => false;
        public Task<Stream> OpenReadAsync() => Task.FromResult<Stream>(new MemoryStream(Bytes));
        public Task<Stream> OpenWriteAsync() { OpenWrites++; return Task.FromResult<Stream>(_stream); }
        public Task<StorageItemProperties> GetBasicPropertiesAsync() => Task.FromResult(new StorageItemProperties());
        public Task<string?> SaveBookmarkAsync() => Task.FromResult<string?>(null);
        public Task<IStorageFolder?> GetParentAsync() => Task.FromResult<IStorageFolder?>(null);
        public Task DeleteAsync() => Task.CompletedTask;
        public Task<IStorageItem?> MoveAsync(IStorageFolder destination) => Task.FromResult<IStorageItem?>(null);
        public void Dispose() { }
        private static MemoryStream Create(byte[] bytes)
        {
            var stream = new MemoryStream();
            stream.Write(bytes);
            stream.Position = 0;
            return stream;
        }
    }
}
