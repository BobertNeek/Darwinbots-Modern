namespace Darwinbots.Desktop.Core;

public interface IEngineClientFactory
{
    IEngineClient Create(WorldSetupOptions setup);
}

public sealed class NativeEngineClientFactory : IEngineClientFactory
{
    public static NativeEngineClientFactory Instance { get; } = new();

    private NativeEngineClientFactory()
    {
    }

    public IEngineClient Create(WorldSetupOptions setup) => new NativeEngineClient(setup);
}
