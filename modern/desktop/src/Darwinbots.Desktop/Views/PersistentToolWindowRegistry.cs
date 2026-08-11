using Avalonia.Controls;
using Darwinbots.Desktop.Core;

namespace Darwinbots.Desktop.Views;

public interface IPersistentSnapshotToolWindow
{
    void SetSnapshot(EngineSnapshot snapshot);
}

public sealed class PersistentToolWindowRegistry
{
    private readonly Window _owner;
    private readonly Dictionary<string, Window> _windows = [];

    public PersistentToolWindowRegistry(Window owner)
    {
        _owner = owner ?? throw new ArgumentNullException(nameof(owner));
    }

    public int Count => _windows.Count;

    public T ShowOrActivate<T>(string key, Func<T> factory)
        where T : Window
    {
        ArgumentException.ThrowIfNullOrWhiteSpace(key);
        ArgumentNullException.ThrowIfNull(factory);

        if (_windows.TryGetValue(key, out var existing))
        {
            if (existing is not T typed)
                throw new InvalidOperationException($"Tool key '{key}' is already registered by {existing.GetType().Name}.");
            if (!typed.IsVisible)
                typed.Show(_owner);
            typed.Activate();
            return typed;
        }

        var window = factory();
        _windows.Add(key, window);
        window.Closed += (_, _) =>
        {
            if (_windows.TryGetValue(key, out var registered) && ReferenceEquals(registered, window))
                _windows.Remove(key);
        };
        window.Show(_owner);
        return window;
    }

    public void PublishSnapshot(EngineSnapshot snapshot)
    {
        ArgumentNullException.ThrowIfNull(snapshot);
        foreach (var window in _windows.Values.OfType<IPersistentSnapshotToolWindow>().ToArray())
            window.SetSnapshot(snapshot);
    }

    public bool Close(string key)
    {
        if (!_windows.TryGetValue(key, out var window)) return false;
        window.Close();
        return true;
    }

    public void CloseAll()
    {
        foreach (var window in _windows.Values.ToArray())
            window.Close();
        _windows.Clear();
    }
}
