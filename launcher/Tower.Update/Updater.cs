using System;
using System.Collections.Generic;
using System.IO;
using System.IO.Compression;
using System.Linq;
using System.Text.Json;
using System.Threading;
using System.Threading.Tasks;

namespace Tower.Update
{
    public enum UpdatePhase { Checking, Downloading, Installing, Done, Failed }

    public sealed class UpdateProgress
    {
        public UpdatePhase Phase { get; set; }
        public long DoneBytes { get; set; }
        public long TotalBytes { get; set; }
        public double Percent => TotalBytes <= 0 ? (Phase == UpdatePhase.Done ? 100 : 0) : Math.Min(100, DoneBytes * 100.0 / TotalBytes);
    }

    /// <summary>
    /// 下载并安装一个更新计划。下载的数据先解压、校验哈希后放进暂存区（中断后再来会跳过已下好的）；
    /// 安装时先写进度记录，每个要替换或删除的文件先挪进备份区再放新文件，全部完成后才写安装记录；
    /// 任何一步失败都从备份还原，进程被强杀的话下次启动由 Recover 还原。
    /// </summary>
    public sealed class Updater
    {
        readonly string installDir;
        readonly IReleaseSource source;
        public int Concurrency { get; set; } = 4;
        /// <summary>测试用：安装到第几个文件时模拟失败</summary>
        internal int? FailAfterWrites { get; set; }

        public Updater(string installDir, IReleaseSource source)
        {
            this.installDir = Path.GetFullPath(installDir);
            this.source = source;
        }

        public async Task ApplyAsync(UpdatePlan plan, IProgress<UpdateProgress>? progress, CancellationToken cancel)
        {
            Recover(installDir);
            var total = plan.DownloadBytes;
            long done = 0;
            void report(UpdatePhase phase) => progress?.Report(new UpdateProgress { Phase = phase, DoneBytes = Interlocked.Read(ref done), TotalBytes = total });
            report(UpdatePhase.Downloading);
            Directory.CreateDirectory(InstallPaths.Work(installDir, "stage"));
            using (var gate = new SemaphoreSlim(Math.Max(1, Concurrency)))
            {
                var tasks = Ranges.Coalesce(plan.Blobs).Select(async request =>
                {
                    await gate.WaitAsync(cancel).ConfigureAwait(false);
                    try
                    {
                        var data = await source.RangeAsync(request.Tag, request.Asset, request.Offset, request.Length, cancel).ConfigureAwait(false);
                        foreach (var blob in request.Blobs) Stage(blob.Key, blob.Value, data, request.Offset);
                        Interlocked.Add(ref done, request.Blobs.Sum(b => b.Value.Length));
                        report(UpdatePhase.Downloading);
                    }
                    finally { gate.Release(); }
                }).ToList();
                await Task.WhenAll(tasks).ConfigureAwait(false);
            }
            cancel.ThrowIfCancellationRequested();
            report(UpdatePhase.Installing);
            Commit(plan);
            report(UpdatePhase.Done);
        }

        void Stage(string sha, BlobLocation blob, byte[] range, long rangeOffset)
        {
            var start = checked((int)(blob.Offset - rangeOffset));
            byte[] content;
            if (blob.Method == "store")
            {
                content = new byte[blob.Length];
                Buffer.BlockCopy(range, start, content, 0, content.Length);
            }
            else
            {
                using var input = new MemoryStream(range, start, checked((int)blob.Length));
                using var inflate = new DeflateStream(input, CompressionMode.Decompress);
                using var output = new MemoryStream();
                inflate.CopyTo(output);
                content = output.ToArray();
            }
            if (content.LongLength != blob.Size || Hashes.Of(content) != sha) throw new InvalidDataException($"下载的数据校验失败：{sha.Substring(0, 12)}");
            var path = Planner.StagePath(installDir, sha);
            File.WriteAllBytes(path + ".part", content);
            if (File.Exists(path)) File.Delete(path);
            File.Move(path + ".part", path);
        }

        sealed class Journal
        {
            public int Version { get; set; }
            /// <summary>这次安装的备份目录（工作目录下）。每次安装一个新目录：被占用删不掉的旧备份（比如正在运行的旧版启动器）留到下次启动再清</summary>
            public string Backup { get; set; } = "backup";
            /// <summary>已经挪进备份区的原文件（相对路径），还原时放回</summary>
            public List<string> Backed { get; set; } = new List<string>();
            /// <summary>已经写入的新文件，还原时删除（如果原来没有这个文件）</summary>
            public List<string> Written { get; set; } = new List<string>();
        }

        static string JournalPath(string installDir) => InstallPaths.Work(installDir, "journal.json");

        void Commit(UpdatePlan plan)
        {
            foreach (var file in plan.Writes)
                if (!File.Exists(Planner.StagePath(installDir, file.Sha256))) throw new InvalidDataException($"暂存区缺少文件：{file.Path}");
            var journal = new Journal { Version = plan.Target.Version, Backup = "backup-" + DateTime.UtcNow.Ticks };
            var backup = InstallPaths.Work(installDir, journal.Backup);
            Save(journal);
            var cache = HashCache.Load(installDir);
            try
            {
                var count = 0;
                foreach (var file in plan.Writes)
                {
                    if (FailAfterWrites == count) throw new IOException("模拟安装失败");
                    var target = InstallPaths.Full(installDir, file.Path);
                    if (File.Exists(target)) { MoveAside(file.Path, target, backup); journal.Backed.Add(file.Path); }
                    Directory.CreateDirectory(Path.GetDirectoryName(target)!);
                    File.Copy(Planner.StagePath(installDir, file.Sha256), target);
                    journal.Written.Add(file.Path);
                    cache.Forget(file.Path);
                    Save(journal);
                    count++;
                }
                foreach (var path in plan.Deletes)
                {
                    var target = InstallPaths.Full(installDir, path);
                    if (!File.Exists(target)) continue;
                    MoveAside(path, target, backup);
                    journal.Backed.Add(path);
                    cache.Forget(path);
                    Save(journal);
                }
                new InstallState { Version = plan.Target.Version, DisplayVersion = plan.Target.DisplayVersion, Files = plan.Target.Files.Select(f => new FileEntry { Path = f.Path, Size = f.Size, Sha256 = f.Sha256 }).ToList() }.Save(installDir);
            }
            catch
            {
                Rollback(installDir, journal);
                File.Delete(JournalPath(installDir));
                throw;
            }
            File.Delete(JournalPath(installDir));
            TryDelete(backup);
            TryDelete(InstallPaths.Work(installDir, "stage"));
            cache.Save();
        }

        /// <summary>原文件挪进备份区；被占用（比如正在运行的启动器自己）就改名留在原处，下次启动清理</summary>
        static void MoveAside(string relative, string target, string backup)
        {
            var saved = Path.Combine(backup, relative.Replace('/', Path.DirectorySeparatorChar));
            Directory.CreateDirectory(Path.GetDirectoryName(saved)!);
            try { File.Move(target, saved); }
            catch (IOException)
            {
                var old = target + ".old-" + DateTime.UtcNow.Ticks;
                File.Move(target, old);
                File.WriteAllText(saved + ".moved", old);
            }
        }

        void Save(Journal journal) => Atomic.WriteText(JournalPath(installDir), JsonSerializer.Serialize(journal));

        /// <summary>上次安装被打断（进度记录还在）：删掉已写入的新文件，把备份放回原处</summary>
        public static void Recover(string installDir)
        {
            installDir = Path.GetFullPath(installDir);
            CleanOld(installDir);
            var path = JournalPath(installDir);
            if (File.Exists(path))
            {
                Journal? journal = null;
                try { journal = JsonSerializer.Deserialize<Journal>(File.ReadAllText(path)); } catch (JsonException) { }
                if (journal != null) Rollback(installDir, journal);
                File.Delete(path);
            }
            var work = InstallPaths.Work(installDir);
            if (Directory.Exists(work))
                foreach (var stale in Directory.EnumerateDirectories(work, "backup*")) TryDelete(stale);
        }

        static void Rollback(string installDir, Journal journal)
        {
            // 备份目录名来自进度记录：只接受本类自己起的名字，记录被改坏也不会去动别的目录
            var backup = InstallPaths.Work(installDir, IsBackupName(journal.Backup) ? journal.Backup : "backup");
            foreach (var relative in journal.Written)
            {
                var target = InstallPaths.Full(installDir, relative);
                if (File.Exists(target)) File.Delete(target);
            }
            foreach (var relative in journal.Backed)
            {
                var target = InstallPaths.Full(installDir, relative);
                var saved = Path.Combine(backup, relative.Replace('/', Path.DirectorySeparatorChar));
                if (File.Exists(saved)) { if (File.Exists(target)) File.Delete(target); File.Move(saved, target); }
                else if (File.Exists(saved + ".moved"))
                {
                    var old = File.ReadAllText(saved + ".moved");
                    if (File.Exists(old)) { if (File.Exists(target)) File.Delete(target); File.Move(old, target); }
                }
            }
            TryDelete(backup);
        }

        static bool IsBackupName(string name) => name == "backup" || (name.StartsWith("backup-", StringComparison.Ordinal) && name.Substring(7).All(char.IsDigit) && name.Length > 7);

        /// <summary>上次因为被占用而改名留下的旧文件，能删就删</summary>
        static void CleanOld(string installDir)
        {
            if (!Directory.Exists(installDir)) return;
            foreach (var old in Directory.EnumerateFiles(installDir, "*.old-*", SearchOption.AllDirectories))
            {
                if (File.Exists(JournalPath(installDir))) break;
                try { File.Delete(old); } catch (IOException) { } catch (UnauthorizedAccessException) { }
            }
        }

        static void TryDelete(string dir)
        {
            try { if (Directory.Exists(dir)) Directory.Delete(dir, true); } catch (IOException) { } catch (UnauthorizedAccessException) { }
        }
    }
}
