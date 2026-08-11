using Avalonia.Controls;
using Darwinbots.Desktop.Views;
using Xunit;

namespace Darwinbots.Desktop.Tests;

public sealed class PersistentToolWindowRegistryTests
{
    [Fact]
    public async Task ReopeningARegisteredToolReusesTheNonmodalWindowAndLeavesOwnerUsable()
    {
        await DesktopTestApplication.Session.Dispatch(() =>
        {
            var owner = new Window { Title = "Playfield" };
            owner.Show();
            var registry = new PersistentToolWindowRegistry(owner);
            var factoryCalls = 0;

            try
            {
                var first = registry.ShowOrActivate("status", () =>
                {
                    factoryCalls++;
                    return new Window { Title = "Status" };
                });
                var second = registry.ShowOrActivate("status", () =>
                {
                    factoryCalls++;
                    return new Window { Title = "Duplicate" };
                });

                Assert.Same(first, second);
                Assert.Equal(1, factoryCalls);
                Assert.True(owner.IsVisible);
                Assert.True(first.IsVisible);
                Assert.Single(owner.OwnedWindows);
            }
            finally
            {
                registry.CloseAll();
                owner.Close();
            }
        }, CancellationToken.None);
    }
}
