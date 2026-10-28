using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Text.Json;
using System.Text.Json.Serialization;

namespace Tower.Update
{
    /// <summary>tower-release.json（distribution/release_pack.py 生成）：这一版的完整文件列表，以及每个文件内容存在哪个版本的数据包里。</summary>
    public sealed class ReleaseManifest
    {
        public const int Schema = 1;
        public const string FileName = "tower-release.json";

        [JsonPropertyName("schema")] public int SchemaVersion { get; set; }
        [JsonPropertyName("version")] public int Version { get; set; }
        [JsonPropertyName("displayVersion")] public string DisplayVersion { get; set; } = "";
        [JsonPropertyName("launcherVersion")] public string LauncherVersion { get; set; } = "";
        [JsonPropertyName("updateKind")] public string UpdateKind { get; set; } = "package";
        [JsonPropertyName("tag")] public string Tag { get; set; } = "";
        [JsonPropertyName("notes")] public string Notes { get; set; } = "";
        [JsonPropertyName("files")] public List<FileEntry> Files { get; set; } = new List<FileEntry>();
        [JsonPropertyName("blobs")] public Dictionary<string, BlobLocation> Blobs { get; set; } = new Dictionary<string, BlobLocation>();

        public static ReleaseManifest Parse(string json)
        {
            var manifest = JsonSerializer.Deserialize<ReleaseManifest>(json) ?? throw new InvalidDataException("更新清单为空");
            manifest.Validate();
            return manifest;
        }

        /// <summary>清单来自网络：路径必须安全、哈希格式正确、每个文件都能找到数据位置。</summary>
        public void Validate()
        {
            if (SchemaVersion != Schema) throw new InvalidDataException($"不支持的更新清单格式：{SchemaVersion}");
            if (Version <= 0 || Files.Count == 0) throw new InvalidDataException("更新清单不完整");
            if (!string.IsNullOrEmpty(DisplayVersion) && !Versions.Valid(DisplayVersion)) throw new InvalidDataException("整合包显示版本格式无效");
            if (!string.IsNullOrEmpty(LauncherVersion) && !Versions.Valid(LauncherVersion)) throw new InvalidDataException("启动器版本格式无效");
            if (UpdateKind != "package" && UpdateKind != "launcher") throw new InvalidDataException("更新类型无效");
            var seen = new HashSet<string>(StringComparer.OrdinalIgnoreCase);
            foreach (var file in Files)
            {
                InstallPaths.Check(file.Path);
                if (!seen.Add(file.Path)) throw new InvalidDataException($"清单里有重复的文件：{file.Path}");
                if (!Hashes.IsSha256(file.Sha256) || file.Size < 0) throw new InvalidDataException($"文件校验信息无效：{file.Path}");
                if (!Blobs.TryGetValue(file.Sha256, out var blob)) throw new InvalidDataException($"找不到文件数据：{file.Path}");
                blob.Validate(file.Size);
            }
        }
    }

    public sealed class FileEntry
    {
        [JsonPropertyName("path")] public string Path { get; set; } = "";
        [JsonPropertyName("size")] public long Size { get; set; }
        [JsonPropertyName("sha256")] public string Sha256 { get; set; } = "";
    }

    public sealed class BlobLocation
    {
        [JsonPropertyName("tag")] public string Tag { get; set; } = "";
        [JsonPropertyName("asset")] public string Asset { get; set; } = "";
        [JsonPropertyName("offset")] public long Offset { get; set; }
        [JsonPropertyName("length")] public long Length { get; set; }
        [JsonPropertyName("method")] public string Method { get; set; } = "";
        [JsonPropertyName("size")] public long Size { get; set; }

        internal void Validate(long fileSize)
        {
            bool simpleName(string s) => s.Length > 0 && s.All(c => char.IsLetterOrDigit(c) || c == '.' || c == '-' || c == '_');
            if (!simpleName(Tag) || !simpleName(Asset)) throw new InvalidDataException("数据包名称无效");
            if (Offset < 0 || Length < 0 || Size != fileSize) throw new InvalidDataException("数据位置无效");
            if (Method != "deflate" && Method != "store") throw new InvalidDataException($"不支持的压缩方式：{Method}");
            if (Method == "store" && Length != Size) throw new InvalidDataException("未压缩数据的长度不一致");
        }
    }

    /// <summary>tower-install.json：最近一次成功更新后安装目录里属于本模组的文件。只有记在这里的文件才会在新版本去掉时被删除。</summary>
    public sealed class InstallState
    {
        public const string FileName = "tower-install.json";
        [JsonPropertyName("version")] public int Version { get; set; }
        [JsonPropertyName("displayVersion")] public string DisplayVersion { get; set; } = "";
        [JsonPropertyName("launcherVersion")] public string LauncherVersion { get; set; } = "";
        [JsonPropertyName("transaction")] public string Transaction { get; set; } = "";
        [JsonPropertyName("files")] public List<FileEntry> Files { get; set; } = new List<FileEntry>();

        public static InstallState? Load(string installDir)
        {
            var path = System.IO.Path.Combine(installDir, FileName);
            if (!File.Exists(path)) return null;
            try { return JsonSerializer.Deserialize<InstallState>(File.ReadAllText(path)); }
            catch (JsonException) { return null; }
        }

        public void Save(string installDir) => Atomic.WriteText(System.IO.Path.Combine(installDir, FileName), JsonSerializer.Serialize(this));
    }

    internal static class Hashes
    {
        public static bool IsSha256(string value) => value.Length == 64 && value.All(c => (c >= '0' && c <= '9') || (c >= 'a' && c <= 'f'));

        public static string Of(Stream stream)
        {
            using var sha = System.Security.Cryptography.SHA256.Create();
            return string.Concat(sha.ComputeHash(stream).Select(b => b.ToString("x2")));
        }

        public static string Of(byte[] data)
        {
            using var sha = System.Security.Cryptography.SHA256.Create();
            return string.Concat(sha.ComputeHash(data).Select(b => b.ToString("x2")));
        }
    }

    internal static class Atomic
    {
        /// <summary>先写临时文件再替换，进程中途被杀也不会留下半个文件。</summary>
        public static void WriteText(string path, string text)
        {
            Directory.CreateDirectory(System.IO.Path.GetDirectoryName(path)!);
            var temp = path + ".tmp";
            File.WriteAllText(temp, text);
            if (File.Exists(path)) File.Replace(temp, path, null);
            else File.Move(temp, path);
        }
    }
}
