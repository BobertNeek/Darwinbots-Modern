using System.ComponentModel;

namespace Darwinbots.Desktop.ViewModels;

public sealed class StatusPanelViewModel : INotifyPropertyChanged
{
    internal StatusPanelViewModel(string label)
    {
        Label = label;
    }

    public string Label { get; }
    public string Value { get; private set; } = "0";
    public string Display => $"{Label} {Value}";

    public event PropertyChangedEventHandler? PropertyChanged;

    internal void SetValue(string value)
    {
        if (Value == value) return;
        Value = value;
        PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(nameof(Value)));
        PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(nameof(Display)));
    }
}
