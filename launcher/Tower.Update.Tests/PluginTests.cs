using System.Text;
using Tower.Update;
using Xunit;

/// <summary>额外 DLL 插件：复制、登记、开关、移除都只动 plugins/，并保留玩家手写的 ME3 选项</summary>
public sealed class PluginTests : IDisposable
{
    readonly string root = Path.Combine(Path.GetTempPath(), "tower-plugin-tests-" + Guid.NewGuid().ToString("N"));
    readonly string install, downloads;

    public PluginTests()
    {
        install = Path.Combine(root, "install");
        downloads = Path.Combine(root, "downloads");
        Directory.CreateDirectory(install);
        Directory.CreateDirectory(downloads);
    }

    public void Dispose() { try { Directory.Delete(root, true); } catch (IOException) { } }

    string Write(string relative, string text = "MZ")
    {
        var path = Path.Combine(root, relative.Replace('/', Path.DirectorySeparatorChar));
        Directory.CreateDirectory(Path.GetDirectoryName(path)!);
        File.WriteAllText(path, text);
        return path;
    }

    string List => File.ReadAllText(Path.Combine(install, "plugins", "natives.toml"));

    [Fact]
    public void AddingADllCopiesItsCompanionsButNotTheDownloadedArchive()
    {
        var dll = Write("downloads/erfps.dll");
        Write("downloads/erfps.ini", "fps=120");
        Write("downloads/erfps/textures/a.png");
        Write("downloads/erfps.zip");
        Write("downloads/unrelated.dll");
        var plugins = new Plugins(install);
        Assert.Equal("plugins/erfps/erfps.dll", plugins.AddDll(dll));
        var copied = Path.Combine(install, "plugins", "erfps");
        Assert.True(File.Exists(Path.Combine(copied, "erfps.ini")));
        Assert.True(File.Exists(Path.Combine(copied, "erfps", "textures", "a.png")));
        Assert.False(File.Exists(Path.Combine(copied, "erfps.zip")));
        Assert.False(File.Exists(Path.Combine(copied, "unrelated.dll")));
        var entry = Assert.Single(plugins.List());
        Assert.True(entry.Enabled && entry.Exists);
        // 同名再加一次：放进新文件夹，不覆盖
        Assert.Equal("plugins/erfps-2/erfps.dll", plugins.AddDll(dll));
    }

    [Theory]
    [InlineData("ersc.dll")]
    [InlineData("dinput8.dll")]
    [InlineData("erdtree_tower_preview_v95.dll")]
    [InlineData("readme.txt")]
    public void LoaderCoopTowerAndNonDllFilesAreRefused(string name) =>
        Assert.Throws<InvalidOperationException>(() => new Plugins(install).AddDll(Write("downloads/" + name)));

    [Fact]
    public void AFolderRegistersEveryTopLevelDllAndRefusesWholeDrives()
    {
        Write("downloads/LiveOverlay/overlay.dll");
        Write("downloads/LiveOverlay/helper.dll");
        Write("downloads/LiveOverlay/ersc.dll");
        Write("downloads/LiveOverlay/deps/inner.dll");
        var plugins = new Plugins(install);
        var added = plugins.AddFolder(Path.Combine(downloads, "LiveOverlay"));
        Assert.Equal(new[] { "plugins/LiveOverlay/helper.dll", "plugins/LiveOverlay/overlay.dll" }, added.OrderBy(a => a, StringComparer.Ordinal));
        Assert.True(File.Exists(Path.Combine(install, "plugins", "LiveOverlay", "deps", "inner.dll")));
        Assert.Throws<InvalidOperationException>(() => plugins.AddFolder(Path.GetPathRoot(downloads)!));
        Write("downloads/empty/readme.txt");
        Assert.Throws<InvalidOperationException>(() => plugins.AddFolder(Path.Combine(downloads, "empty")));
    }

    [Fact]
    public void TogglingKeepsHandWrittenOptionsAndRemovingDeletesOnlyUnusedFolders()
    {
        Directory.CreateDirectory(Path.Combine(install, "plugins"));
        File.WriteAllText(Path.Combine(install, "plugins", "natives.toml"),
            "# 我自己的说明\n[[natives]]\npath = 'plugins/a/a.dll'\nload_after = [{ id = \"ersc.dll\", optional = true }]\n\n[[natives]]\npath = 'plugins/a/b.dll'\nenabled = true\n", new UTF8Encoding(false));
        Write("install/plugins/a/a.dll");
        Write("install/plugins/a/b.dll");
        var plugins = new Plugins(install);
        plugins.SetEnabled("plugins/a/a.dll", false);
        Assert.Contains("# 我自己的说明", List);
        Assert.Contains("load_after", List);
        Assert.False(plugins.List().Single(p => p.Path == "plugins/a/a.dll").Enabled);
        plugins.SetEnabled("plugins/a/b.dll", false);
        Assert.DoesNotContain("enabled = true", List);
        // 同一个文件夹还有别的插件在用：只去掉清单里的一条
        plugins.Remove("plugins/a/a.dll");
        Assert.True(Directory.Exists(Path.Combine(install, "plugins", "a")));
        plugins.Remove("plugins/a/b.dll");
        Assert.False(Directory.Exists(Path.Combine(install, "plugins", "a")));
        Assert.Empty(plugins.List());
    }

    [Fact]
    public void AnEntryWhoseFileIsGoneStillShowsUpMarkedMissing()
    {
        Directory.CreateDirectory(Path.Combine(install, "plugins"));
        File.WriteAllText(Path.Combine(install, "plugins", "natives.toml"), "[[natives]]\npath = 'plugins/gone/gone.dll'\n");
        var entry = Assert.Single(new Plugins(install).List());
        Assert.False(entry.Exists);
        Assert.True(entry.Enabled);
    }
}
