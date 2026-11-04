using Tower.Launcher;
using Xunit;

namespace Tower.Update.Tests;

public class LobbyLaunchTests
{
    [Theory]
    [InlineData(true, false, false, false)]
    [InlineData(false, true, false, false)]
    [InlineData(false, false, true, false)]
    [InlineData(false, false, false, true)]
    public void Busy_launch_rejects_configuration_changes(bool game, bool launching, bool worker, bool updating)
    {
        Assert.NotNull(LobbyLaunch.BlockReason(game, launching, worker, updating));
    }

    [Fact]
    public void Idle_launch_is_allowed_and_password_fingerprint_is_exact()
    {
        Assert.Null(LobbyLaunch.BlockReason(false, false, false, false));
        Assert.Equal("ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad", LobbyLaunch.Fingerprint("abc"));
        Assert.NotEqual(LobbyLaunch.Fingerprint("abc"), LobbyLaunch.Fingerprint("Abc"));
    }
}
