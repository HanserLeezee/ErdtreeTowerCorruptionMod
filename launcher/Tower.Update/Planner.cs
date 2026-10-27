using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Text.Json;

namespace Tower.Update
{
    /// <summary>本地文件哈希缓存：大小与修改时间都没变就沿用上次的哈希，不用每次把几百 MB 文件重算一遍。</summary>
    public sealed class HashCache
    {
        sealed class Entry { public long Size { get; set; } public long Ticks { get; set; } public string Sha256 { get; set; } = ""; }
        readonly Dictionary<string, Entry> entries;
        readonly string path;
        bool dirty;

        HashCache(string path, Dictionary<string, Entry> entries) { this.path = path; this.entries = entries; }

        public static HashCache Load(string installDir)
        {
            var path = InstallPaths.Work(installDir, "hashes.json");
            try
            {
                if (File.Exists(path))
                    return new HashCache(path, JsonSerializer.Deserialize<Dictionary<string, Entry>>(File.ReadAllText(path)) ?? new Dictionary<string, Entry>());
            }
            catch (JsonException) { }
            return new HashCache(path, new Dictionary<string, Entry>());
        }

        /// <summary>文件不存在返回 null</summary>
        public string? Sha256(string installDir, string relative)
        {
            var full = InstallPaths.Full(installDir, relative);
            var info = new FileInfo(full);
            if (!info.Exists) return null;
            if (entries.TryGetValue(relative, out var known) && known.Size == info.Length && known.Ticks == info.LastWriteTimeUtc.Ticks) return known.Sha256;
            string sha;
            using (var stream = File.OpenRead(full)) sha = Hashes.Of(stream);
            entries[relative] = new Entry { Size = info.Length, Ticks = info.LastWriteTimeUtc.Ticks, Sha256 = sha };
            dirty = true;
            return sha;
        }

        public void Forget(string relative) { if (entries.Remove(relative)) dirty = true; }

        /// <summary>校验文件时用：忘掉所有缓存，每个文件都重新计算哈希</summary>
        public void Clear() { if (entries.Count > 0) { entries.Clear(); dirty = true; } }

        public void Save()
        {
            if (!dirty) return;
            Atomic.WriteText(path, JsonSerializer.Serialize(entries));
            dirty = false;
        }
    }

    /// <summary>一次更新要做的事：下载哪些数据（按哈希去重）、写哪些文件、删哪些旧文件。</summary>
    public sealed class UpdatePlan
    {
        public ReleaseManifest Target { get; }
        public int? InstalledVersion { get; }
        public IReadOnlyList<FileEntry> Writes { get; }
        public IReadOnlyList<string> Deletes { get; }
        public IReadOnlyDictionary<string, BlobLocation> Blobs { get; }
        /// <summary>需要下载的字节数（压缩后）</summary>
        public long DownloadBytes => Blobs.Values.Sum(b => b.Length);
        public bool UpToDate => Writes.Count == 0 && Deletes.Count == 0 && InstalledVersion == Target.Version;

        internal UpdatePlan(ReleaseManifest target, int? installed, List<FileEntry> writes, List<string> deletes, Dictionary<string, BlobLocation> blobs)
        {
            Target = target; InstalledVersion = installed; Writes = writes; Deletes = deletes; Blobs = blobs;
        }
    }

    public static class Planner
    {
        /// <summary>
        /// 和清单逐个比对本地文件：缺失或哈希不同的要写入；上次安装记录里有、新版本没有的要删除（玩家自己的文件不碰）。
        /// 同一份内容只下载一次，暂存区里已经下好并校验过的跳过（续传）。
        /// </summary>
        public static UpdatePlan Plan(string installDir, ReleaseManifest target, HashCache cache)
        {
            target.Validate();
            var state = InstallState.Load(installDir);
            var writes = new List<FileEntry>();
            foreach (var file in target.Files)
                if (cache.Sha256(installDir, file.Path) != file.Sha256) writes.Add(file);
            var keep = new HashSet<string>(target.Files.Select(f => f.Path), StringComparer.OrdinalIgnoreCase);
            var deletes = new List<string>();
            foreach (var old in state?.Files ?? new List<FileEntry>())
            {
                try { InstallPaths.Check(old.Path); } catch (InvalidDataException) { continue; }
                if (!keep.Contains(old.Path) && File.Exists(InstallPaths.Full(installDir, old.Path))) deletes.Add(old.Path);
            }
            var blobs = new Dictionary<string, BlobLocation>();
            foreach (var file in writes)
            {
                if (blobs.ContainsKey(file.Sha256) || Staged(installDir, file.Sha256)) continue;
                blobs[file.Sha256] = target.Blobs[file.Sha256];
            }
            cache.Save();
            return new UpdatePlan(target, state?.Version, writes, deletes, blobs);
        }

        internal static string StagePath(string installDir, string sha) => InstallPaths.Work(installDir, "stage", sha);

        internal static bool Staged(string installDir, string sha)
        {
            var path = StagePath(installDir, sha);
            if (!File.Exists(path)) return false;
            using var stream = File.OpenRead(path);
            if (Hashes.Of(stream) == sha) return true;
            stream.Dispose();
            File.Delete(path);
            return false;
        }
    }

    /// <summary>把同一个数据包里位置相邻的数据合并成一次分段下载（间隔不超过 64 KiB、单次不超过 8 MiB，进度条也按这个粒度前进）。</summary>
    public static class Ranges
    {
        public sealed class Request
        {
            public string Tag { get; set; } = "";
            public string Asset { get; set; } = "";
            public long Offset { get; set; }
            public long Length { get; set; }
            public List<KeyValuePair<string, BlobLocation>> Blobs { get; } = new List<KeyValuePair<string, BlobLocation>>();
        }

        public const long MaxGap = 64 * 1024, MaxRequest = 8L * 1024 * 1024;

        public static List<Request> Coalesce(IReadOnlyDictionary<string, BlobLocation> blobs)
        {
            var requests = new List<Request>();
            foreach (var group in blobs.GroupBy(b => (b.Value.Tag, b.Value.Asset)).OrderBy(g => g.Key.Tag, StringComparer.Ordinal).ThenBy(g => g.Key.Asset, StringComparer.Ordinal))
            {
                Request? current = null;
                foreach (var blob in group.OrderBy(b => b.Value.Offset))
                {
                    var end = blob.Value.Offset + blob.Value.Length;
                    if (current != null && blob.Value.Offset - (current.Offset + current.Length) <= MaxGap && end - current.Offset <= MaxRequest)
                        current.Length = Math.Max(current.Length, end - current.Offset);
                    else
                    {
                        current = new Request { Tag = group.Key.Tag, Asset = group.Key.Asset, Offset = blob.Value.Offset, Length = blob.Value.Length };
                        requests.Add(current);
                    }
                    current.Blobs.Add(blob);
                }
            }
            return requests;
        }
    }
}
