using System.Diagnostics;
using System.Net;
using System.Text;
using Tower.Update;
using Xunit;

namespace Tower.Update.Tests;

/// <summary>端到端：用 distribution/release_pack.py 生成真实的版本数据，再用更新引擎安装，检查结果与安装记录。</summary>
public sealed class UpdateTests : IDisposable
{
    readonly string root = Path.Combine(Path.GetTempPath(), "tower-update-tests-" + Guid.NewGuid().ToString("N"));
    static readonly string Repo = FindRepo();

    public UpdateTests() => Directory.CreateDirectory(root);
    public void Dispose() { try { Directory.Delete(root, true); } catch (IOException) { } }

    static string FindRepo()
    {
        var dir = AppContext.BaseDirectory;
        while (dir != null && !File.Exists(Path.Combine(dir, "distribution", "release_pack.py"))) dir = Path.GetDirectoryName(dir);
        return dir ?? throw new InvalidOperationException("找不到仓库根目录");
    }

    static void Write(string dir, Dictionary<string, byte[]> files)
    {
        foreach (var (rel, data) in files)
        {
            var path = Path.Combine(dir, rel.Replace('/', Path.DirectorySeparatorChar));
            Directory.CreateDirectory(Path.GetDirectoryName(path)!);
            File.WriteAllBytes(path, data);
        }
    }

    /// <summary>把一个版本打包进 releases/{tag}/，并把它设为 latest</summary>
    string Release(int version, Dictionary<string, byte[]> files, string? previous)
    {
        var stage = Path.Combine(root, "stage-" + version);
        Write(stage, files);
        var output = Path.Combine(root, "releases", "v" + version);
        var args = $"\"{Path.Combine(Repo, "distribution", "release_pack.py")}\" \"{stage}\" {version} \"{output}\"" + (previous == null ? "" : $" --previous \"{previous}\"");
        using var process = Process.Start(new ProcessStartInfo("python3", args) { RedirectStandardError = true, RedirectStandardOutput = true })!;
        process.WaitForExit();
        Assert.True(process.ExitCode == 0, process.StandardError.ReadToEnd());
        var latest = Path.Combine(root, "releases", "latest");
        Directory.CreateDirectory(latest);
        File.Copy(Path.Combine(output, ReleaseManifest.FileName), Path.Combine(latest, ReleaseManifest.FileName), true);
        return Path.Combine(output, ReleaseManifest.FileName);
    }

    async Task<UpdatePlan> Update(string install, IReleaseSource? source = null, int? failAfter = null)
    {
        source ??= new DirectoryReleaseSource(Path.Combine(root, "releases"));
        Updater.Recover(install);
        var plan = Planner.Plan(install, await source.LatestAsync(CancellationToken.None), HashCache.Load(install));
        var updater = new Updater(install, source) { FailAfterWrites = failAfter };
        await updater.ApplyAsync(plan, null, CancellationToken.None);
        return plan;
    }

    static Dictionary<string, byte[]> V1 => new()
    {
        ["erdtree_tower.dll"] = Encoding.UTF8.GetBytes(new string('a', 20000)),
        ["scene-assets/regulation.bin"] = Enumerable.Range(0, 50000).Select(i => (byte)(i * 7)).ToArray(),
        ["scene-assets/reward-icons/1.rgba"] = Enumerable.Range(0, 80000).Select(i => (byte)(i % 13)).ToArray(),
        ["scene-assets/old.txt"] = Encoding.UTF8.GetBytes("going away"),
    };

    static Dictionary<string, byte[]> V2 => new()
    {
        ["erdtree_tower.dll"] = Encoding.UTF8.GetBytes(new string('b', 20000)),
        ["scene-assets/regulation.bin"] = V1["scene-assets/regulation.bin"],
        ["scene-assets/reward-icons/1.rgba"] = V1["scene-assets/reward-icons/1.rgba"],
        ["scene-assets/new.txt"] = Encoding.UTF8.GetBytes("fresh"),
    };

    static Dictionary<string, byte[]> V3 => new(V2)
    {
        ["erdtree_tower.dll"] = Encoding.UTF8.GetBytes(new string('c', 20000)),
        ["scene-assets/regulation.bin"] = Enumerable.Range(0, 50000).Select(i => (byte)(i * 11)).ToArray(),
    };

    static void AssertInstalled(string install, Dictionary<string, byte[]> expected, int version)
    {
        foreach (var (rel, data) in expected) Assert.Equal(data, File.ReadAllBytes(Path.Combine(install, rel)));
        var state = InstallState.Load(install)!;
        Assert.Equal(version, state.Version);
        Assert.Equal(expected.Keys.OrderBy(k => k, StringComparer.Ordinal), state.Files.Select(f => f.Path).OrderBy(k => k, StringComparer.Ordinal));
        Assert.Empty(Directory.EnumerateDirectories(InstallPaths.Work(install), "backup*"));
        Assert.False(File.Exists(InstallPaths.Work(install, "journal.json")));
    }

    [Fact]
    public async Task FreshInstallThenOnlyChangedFilesAreDownloaded()
    {
        var install = Path.Combine(root, "install");
        Directory.CreateDirectory(install);
        var v1 = Release(1, V1, null);
        var first = await Update(install);
        Assert.Equal(4, first.Writes.Count);
        AssertInstalled(install, V1, 1);
        File.WriteAllText(Path.Combine(install, "GamePath.txt"), "D:/Game");
        Release(2, V2, v1);
        var second = await Update(install);
        // 只下载改动的 DLL 与新文件；没变的大文件不下载；旧版本的文件被删除，玩家自己的文件保留
        Assert.Equal(new[] { "erdtree_tower.dll", "scene-assets/new.txt" }, second.Writes.Select(w => w.Path).OrderBy(p => p, StringComparer.Ordinal));
        Assert.Equal(new[] { "scene-assets/old.txt" }, second.Deletes);
        Assert.Equal(2, second.Blobs.Count);
        AssertInstalled(install, V2, 2);
        Assert.False(File.Exists(Path.Combine(install, "scene-assets", "old.txt")));
        Assert.True(File.Exists(Path.Combine(install, "GamePath.txt")));
        Assert.True((await Update(install)).UpToDate);
    }

    [Fact]
    public async Task SkippedVersionsDownloadEachChangeOnceAndDamageIsRepaired()
    {
        var install = Path.Combine(root, "install");
        Directory.CreateDirectory(install);
        var v1 = Release(1, V1, null);
        await Update(install);
        var v2 = Release(2, V2, v1);
        Release(3, V3, v2);
        File.WriteAllText(Path.Combine(install, "scene-assets", "reward-icons", "1.rgba"), "damaged");
        var plan = await Update(install);
        // 从第一版直接到第三版：DLL 与 regulation 取第三版，新文件取第二版的数据包，损坏的图标从第一版的数据包修复
        Assert.Equal(new[] { "v1", "v2", "v3" }, plan.Blobs.Values.Select(b => b.Tag).Distinct().OrderBy(t => t, StringComparer.Ordinal));
        AssertInstalled(install, V3, 3);
    }

    [Fact]
    public async Task FailedInstallRollsBackEveryFile()
    {
        var install = Path.Combine(root, "install");
        Directory.CreateDirectory(install);
        var v1 = Release(1, V1, null);
        await Update(install);
        Release(2, V2, v1);
        await Assert.ThrowsAsync<IOException>(() => Update(install, failAfter: 1));
        AssertInstalled(install, V1, 1);
        // 下好的数据还留在暂存区：再来一次不用重新下载
        var source = new CountingSource(new DirectoryReleaseSource(Path.Combine(root, "releases")));
        await Update(install, source);
        Assert.Equal(0, source.Requests);
        AssertInstalled(install, V2, 2);
    }

    [Fact]
    public async Task InterruptedInstallIsRestoredOnNextStart()
    {
        var install = Path.Combine(root, "install");
        Directory.CreateDirectory(install);
        var v1 = Release(1, V1, null);
        await Update(install);
        Release(2, V2, v1);
        // 模拟进程在安装中途被杀：伪造一份进度记录，原 DLL 已经挪进备份区、新 DLL 已写入
        var backup = InstallPaths.Work(install, "backup-123");
        Directory.CreateDirectory(backup);
        File.Move(Path.Combine(install, "erdtree_tower.dll"), Path.Combine(backup, "erdtree_tower.dll"));
        File.WriteAllText(Path.Combine(install, "erdtree_tower.dll"), "half written");
        File.WriteAllText(InstallPaths.Work(install, "journal.json"), "{\"Version\":2,\"Backup\":\"backup-123\",\"Backed\":[\"erdtree_tower.dll\"],\"Written\":[\"erdtree_tower.dll\"]}");
        // 更早一次安装留下、当时删不掉的备份（比如正在运行的旧版启动器）：这次启动顺手清掉
        Directory.CreateDirectory(InstallPaths.Work(install, "backup-99", "launcher"));
        File.WriteAllText(InstallPaths.Work(install, "backup-99", "ErdtreeTowerLauncher.exe"), "old");
        Updater.Recover(install);
        AssertInstalled(install, V1, 1);
    }

    [Fact]
    public async Task HttpSourceUsesRangeRequestsAgainstAReleaseServer()
    {
        var install = Path.Combine(root, "install");
        Directory.CreateDirectory(install);
        var v1 = Release(1, V1, null);
        Release(2, V2, v1);
        using var server = new RangeServer(Path.Combine(root, "releases"));
        // 和启动器一样关掉自动跳转：每一跳由 HttpReleaseSource 自己跟，并带上 Range
        using var http = new HttpClient(new HttpClientHandler { AllowAutoRedirect = false });
        await Update(install, new HttpReleaseSource(http, server.BaseUrl));
        AssertInstalled(install, V2, 2);
        Assert.True(server.PartialResponses > 0);
        Assert.True(server.Redirects > server.PartialResponses);
    }

    [Fact]
    public async Task VerifyRehashesFilesTheCacheWouldTrust()
    {
        var install = Path.Combine(root, "install");
        Directory.CreateDirectory(install);
        Release(1, V1, null);
        var source = new DirectoryReleaseSource(Path.Combine(root, "releases"));
        await Update(install, source);
        var target = await source.LatestAsync(CancellationToken.None);
        Assert.True(Planner.Plan(install, target, HashCache.Load(install)).UpToDate);
        // 内容被改坏，但大小和修改时间都和缓存里一样：普通检查会信任缓存，校验必须重新计算
        var dll = Path.Combine(install, "erdtree_tower.dll");
        var info = new FileInfo(dll);
        var stamp = info.LastWriteTimeUtc;
        var damaged = File.ReadAllBytes(dll);
        damaged[0] ^= 0xFF;
        File.WriteAllBytes(dll, damaged);
        File.SetLastWriteTimeUtc(dll, stamp);
        Assert.Empty(Planner.Plan(install, target, HashCache.Load(install)).Writes);
        var cache = HashCache.Load(install);
        cache.Clear();
        var plan = Planner.Plan(install, target, cache);
        Assert.Equal(new[] { "erdtree_tower.dll" }, plan.Writes.Select(w => w.Path));
        await new Updater(install, source).ApplyAsync(plan, null, CancellationToken.None);
        AssertInstalled(install, V1, 1);
    }

    [Theory]
    [InlineData("../evil.dll")]
    [InlineData("/etc/passwd")]
    [InlineData("C:/Windows/x.dll")]
    [InlineData("a\\b.txt")]
    [InlineData(".tower-update/journal.json")]
    [InlineData("tower-install.json")]
    [InlineData("tower-options.ini")]
    [InlineData("scene-assets/./x")]
    public void UnsafePathsAreRejected(string path) => Assert.Throws<InvalidDataException>(() => InstallPaths.Check(path));

    [Fact]
    public void NeighbouringBlobsShareOneRequest()
    {
        BlobLocation At(string tag, long offset, long length) => new() { Tag = tag, Asset = "tower-pack.bin", Offset = offset, Length = length, Method = "store", Size = length };
        var requests = Ranges.Coalesce(new Dictionary<string, BlobLocation>
        {
            ["a"] = At("v1", 0, 100), ["b"] = At("v1", 150, 50), ["c"] = At("v1", 10_000_000, 10), ["d"] = At("v2", 0, 5),
        });
        Assert.Equal(3, requests.Count);
        Assert.Equal((0L, 200L), (requests[0].Offset, requests[0].Length));
        Assert.Equal(2, requests[0].Blobs.Count);
    }

    static void CopyDir(string from, string to)
    {
        foreach (var file in Directory.EnumerateFiles(from, "*", SearchOption.AllDirectories))
        {
            var target = Path.Combine(to, Path.GetRelativePath(from, file));
            Directory.CreateDirectory(Path.GetDirectoryName(target)!);
            File.Copy(file, target, true);
        }
    }

    static KeyValuePair<string, IReleaseSource> Line(string name, IReleaseSource source) => new(name, source);

    [Fact]
    public async Task StaleMirrorDoesNotHideANewRelease()
    {
        var install = Path.Combine(root, "install");
        Directory.CreateDirectory(install);
        var v1 = Release(1, V1, null);
        CopyDir(Path.Combine(root, "releases"), Path.Combine(root, "mirror"));
        await Update(install);
        Release(2, V2, v1);
        // 镜像还停在 v1（忘了上传），GitHub 已经有 v2：要拿到 v2，而且数据只从有 v2 的线路下
        var mirror = new CountingSource(new DirectoryReleaseSource(Path.Combine(root, "mirror")));
        var lines = new MultiSource(new[] { Line("国内镜像", mirror), Line("GitHub", new DirectoryReleaseSource(Path.Combine(root, "releases"))) });
        await Update(install, lines);
        Assert.Equal("GitHub", lines.Chosen);
        Assert.Equal(0, mirror.Requests);
        AssertInstalled(install, V2, 2);
    }

    [Fact]
    public async Task PreferredLineWinsTiesAndAFailingLineIsSkipped()
    {
        var install = Path.Combine(root, "install");
        Directory.CreateDirectory(install);
        Release(1, V1, null);
        var broken = new FailingRanges(new DirectoryReleaseSource(Path.Combine(root, "releases")));
        var lines = new MultiSource(new[] { Line("国内镜像", broken), Line("GitHub", new DirectoryReleaseSource(Path.Combine(root, "releases"))) });
        var plan = Planner.Plan(install, await lines.LatestAsync(CancellationToken.None), HashCache.Load(install));
        Assert.Equal("国内镜像", lines.Chosen);
        await new Updater(install, lines) { Concurrency = 1 }.ApplyAsync(plan, null, CancellationToken.None);
        AssertInstalled(install, V1, 1);
        // 第一次失败后这条线路排到最后，后面的分段不再先试它
        Assert.Equal(1, broken.Attempts);
    }

    [Fact]
    public async Task AHangingLineDoesNotStallTheCheck()
    {
        Release(1, V1, null);
        var hanging = new HangingLatest();
        var lines = new MultiSource(new IReleaseSource[] { hanging, new DirectoryReleaseSource(Path.Combine(root, "releases")) }.Select((s, i) => Line(i == 0 ? "国内镜像" : "GitHub", s)))
        {
            Grace = TimeSpan.FromMilliseconds(200),
        };
        var clock = Stopwatch.StartNew();
        var manifest = await lines.LatestAsync(CancellationToken.None);
        Assert.True(clock.Elapsed < TimeSpan.FromSeconds(5));
        Assert.Equal(1, manifest.Version);
        Assert.Equal("GitHub", lines.Chosen);
        Assert.True(await hanging.Released.Task.WaitAsync(TimeSpan.FromSeconds(5)));
    }

    [Fact]
    public async Task WhenEveryLineIsDownEachReasonIsReported()
    {
        var lines = new MultiSource(new[] { Line("国内镜像", new DownSource("连接被拒绝")), Line("GitHub", new DownSource("无法解析主机")) });
        var error = await Assert.ThrowsAsync<IOException>(() => lines.LatestAsync(CancellationToken.None));
        Assert.Contains("国内镜像：连接被拒绝", error.Message);
        Assert.Contains("GitHub：无法解析主机", error.Message);
    }

    sealed class FailingRanges(IReleaseSource inner) : IReleaseSource
    {
        public int Attempts;
        public Task<ReleaseManifest> LatestAsync(CancellationToken cancel) => inner.LatestAsync(cancel);
        public Task<byte[]> RangeAsync(string tag, string asset, long offset, long length, CancellationToken cancel)
        {
            Interlocked.Increment(ref Attempts);
            throw new IOException("镜像上没有这个数据包（HTTP 404）");
        }
    }

    sealed class HangingLatest : IReleaseSource
    {
        public readonly TaskCompletionSource<bool> Released = new();
        public async Task<ReleaseManifest> LatestAsync(CancellationToken cancel)
        {
            try { await Task.Delay(Timeout.Infinite, cancel); }
            finally { Released.TrySetResult(true); }
            throw new InvalidOperationException();
        }
        public Task<byte[]> RangeAsync(string tag, string asset, long offset, long length, CancellationToken cancel) => throw new IOException("不该走到这里");
    }

    sealed class DownSource(string reason) : IReleaseSource
    {
        public Task<ReleaseManifest> LatestAsync(CancellationToken cancel) => Task.FromException<ReleaseManifest>(new IOException(reason));
        public Task<byte[]> RangeAsync(string tag, string asset, long offset, long length, CancellationToken cancel) => throw new IOException(reason);
    }

    sealed class CountingSource(IReleaseSource inner) : IReleaseSource
    {
        public int Requests;
        public Task<ReleaseManifest> LatestAsync(CancellationToken cancel) => inner.LatestAsync(cancel);
        public Task<byte[]> RangeAsync(string tag, string asset, long offset, long length, CancellationToken cancel)
        {
            Interlocked.Increment(ref Requests);
            return inner.RangeAsync(tag, asset, offset, length, cancel);
        }
    }

    /// <summary>
    /// 最小的 GitHub Release 目录结构服务器：releases/latest/download/{asset} 与 releases/download/{tag}/{asset}
    /// 都像 GitHub 一样先 302 到存储地址（assets/...，相对路径），存储地址支持单段 Range
    /// </summary>
    sealed class RangeServer : IDisposable
    {
        readonly HttpListener listener = new();
        readonly string dir;
        public string BaseUrl { get; }
        public int PartialResponses, Redirects;

        public RangeServer(string dir)
        {
            this.dir = dir;
            var port = new System.Net.Sockets.TcpListener(IPAddress.Loopback, 0);
            port.Start();
            var number = ((IPEndPoint)port.LocalEndpoint).Port;
            port.Stop();
            BaseUrl = $"http://127.0.0.1:{number}/releases";
            listener.Prefixes.Add($"http://127.0.0.1:{number}/");
            listener.Start();
            _ = Task.Run(Loop);
        }

        async Task Loop()
        {
            while (listener.IsListening)
            {
                HttpListenerContext context;
                try { context = await listener.GetContextAsync(); } catch { return; }
                var parts = context.Request.Url!.AbsolutePath.Split('/', StringSplitOptions.RemoveEmptyEntries);
                if (parts.Length == 4 && parts[0] == "releases")
                {
                    context.Response.StatusCode = 302;
                    context.Response.RedirectLocation = parts[1] == "latest" ? $"/assets/latest/{parts[3]}" : $"/assets/{parts[2]}/{parts[3]}";
                    Interlocked.Increment(ref Redirects);
                    context.Response.Close();
                    continue;
                }
                var file = parts.Length == 3 && parts[0] == "assets" ? Path.Combine(dir, parts[1], parts[2]) : "";
                if (!File.Exists(file)) { context.Response.StatusCode = 404; context.Response.Close(); continue; }
                var data = await File.ReadAllBytesAsync(file);
                var range = context.Request.Headers["Range"];
                if (range != null && range.StartsWith("bytes="))
                {
                    var bounds = range[6..].Split('-');
                    long from = long.Parse(bounds[0]), to = long.Parse(bounds[1]);
                    data = data[(int)from..(int)(to + 1)];
                    context.Response.StatusCode = 206;
                    Interlocked.Increment(ref PartialResponses);
                }
                await context.Response.OutputStream.WriteAsync(data);
                context.Response.Close();
            }
        }

        public void Dispose() { listener.Stop(); listener.Close(); }
    }
}
