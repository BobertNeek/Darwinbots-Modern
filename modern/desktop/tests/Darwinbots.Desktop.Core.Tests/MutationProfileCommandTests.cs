using Darwinbots.Desktop.Core;
using Xunit;

namespace Darwinbots.Desktop.Core.Tests;

public sealed class MutationProfileCommandTests
{
    [Fact]
    public void GlobalMutationCommandCarriesEveryClassAndCategoryDistribution()
    {
        var profile = MutationProfileUpdate.Default with
        {
            Point = new MutationRateUpdate(11, 3, 1),
            PointCategoryDistribution = new MutationCategoryDistributionUpdate(40, 20, 10, 10, 10, 10),
        };

        var json = NativeCommandSerializer.SerializeMutationSettings(
            MutationSettingsUpdate.Default with { Profile = profile });

        foreach (var name in new[]
                 {
                     "point", "point2", "delta", "copy_error", "copy_error2", "insertion",
                     "reversal", "minor_deletion", "major_deletion", "amplification", "translocation",
                 })
        {
            Assert.Contains($"\"{name}\"", json);
        }

        Assert.Contains("\"point_category_distribution\"", json);
        Assert.Contains("\"copy_error_category_distribution\"", json);
    }

    [Fact]
    public void RobotMutationCommandTargetsTheSelectedRobot()
    {
        var json = NativeCommandSerializer.SerializeOrganismMutationProfile(
            new OrganismKey(7, 3), MutationProfileUpdate.Default);

        Assert.Contains("\"type\":\"update_organism_mutation_profile\"", json);
        Assert.Contains("\"slot\":7", json);
        Assert.Contains("\"generation\":3", json);
        Assert.Contains("\"copy_error\"", json);
    }
}
