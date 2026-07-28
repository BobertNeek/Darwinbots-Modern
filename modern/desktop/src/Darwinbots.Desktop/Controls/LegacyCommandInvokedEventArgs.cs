using Darwinbots.Desktop.Core.LegacyMenus;

namespace Darwinbots.Desktop.Controls;

public sealed class LegacyCommandInvokedEventArgs(LegacyCommandId command) : EventArgs
{
    public LegacyCommandId Command { get; } = command;
}
