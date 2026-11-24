using Avalonia.Media;

namespace Darwinbots.Desktop;

public static class DesktopFonts
{
    public static FontFamily Body { get; } = new("Aptos, Noto Sans, DejaVu Sans, $Default");
    public static FontFamily Heading { get; } = new("Bahnschrift, DIN Alternate, Noto Sans, DejaVu Sans, $Default");
    public static FontFamily Inspector { get; } = new("Consolas, DejaVu Sans Mono, Menlo, $Default");
    public static FontFamily Editor { get; } = new("Cascadia Mono, Consolas, DejaVu Sans Mono, Menlo, $Default");
    public static FontFamily Document { get; } = new("Georgia, Noto Serif, DejaVu Serif, $Default");
}
