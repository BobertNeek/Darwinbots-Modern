using Darwinbots.Desktop.Core;
using Xunit;

namespace Darwinbots.Desktop.Core.Tests;

public sealed class StartupDnaWorkflowTests
{
    [Fact]
    public void StartupBotUsesDeterministicInsertionPointAndLegacyRobotsFallback()
    {
        var root = Directory.CreateTempSubdirectory("darwinbots-startup-");
        try
        {
            var legacyMainDirectory = Path.Combine(root.FullName, "legacy");
            var legacyRobotsDirectory = Path.Combine(legacyMainDirectory, "Robots");
            Directory.CreateDirectory(legacyRobotsDirectory);
            var legacyBot = Path.Combine(legacyRobotsDirectory, "Legacy Hunter.txt");
            File.WriteAllText(legacyBot, "start\nstop");

            var workflow = new StartupDnaWorkflow(
                worldWidth: 16_000f,
                worldHeight: 12_000f,
                legacyMainDirectory);

            var startupBot = Assert.Single(workflow.Resolve(["--bot", "Legacy Hunter.txt"]));

            Assert.Equal(Path.GetFullPath(legacyBot), startupBot.Path);
            Assert.Equal([8_000f, 6_000f], startupBot.Position);
        }
        finally
        {
            root.Delete(recursive: true);
        }
    }
}
