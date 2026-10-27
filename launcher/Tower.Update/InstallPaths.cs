using System;
using System.IO;
using System.Linq;

namespace Tower.Update
{
    /// <summary>安装目录内的相对路径（统一用 /）：清单路径一律先过这里，保证写入、删除都不会越出安装目录。</summary>
    public static class InstallPaths
    {
        /// <summary>更新器自己的工作目录：暂存、备份、进度记录与哈希缓存，不属于任何版本</summary>
        public const string WorkDir = ".tower-update";
        /// <summary>玩家自己的文件：发布清单里不能有，更新不会写也不会删</summary>
        static readonly string[] Private = { InstallState.FileName, "GamePath.txt", "tower-options.ini", "preview.launch.me3" };
        /// <summary>玩家自己的目录：额外 DLL 插件</summary>
        static readonly string[] PrivateDirs = { "plugins" };

        public static void Check(string relative)
        {
            if (string.IsNullOrEmpty(relative) || relative.Length > 400) throw new InvalidDataException("文件路径无效");
            if (relative.Contains('\\') || relative.Contains(':') || relative.StartsWith("/", StringComparison.Ordinal))
                throw new InvalidDataException($"文件路径无效：{relative}");
            var parts = relative.Split('/');
            if (parts.Any(p => p.Length == 0 || p == "." || p == ".." || p.Trim() != p || p.EndsWith(".", StringComparison.Ordinal)))
                throw new InvalidDataException($"文件路径无效：{relative}");
            if (relative.IndexOfAny(Path.GetInvalidPathChars()) >= 0 || parts.Any(p => p.IndexOfAny(new[] { '<', '>', '"', '|', '?', '*' }) >= 0 || p.Any(char.IsControl)))
                throw new InvalidDataException($"文件路径无效：{relative}");
            if (parts[0].Equals(WorkDir, StringComparison.OrdinalIgnoreCase) || (parts.Length > 1 && PrivateDirs.Any(d => d.Equals(parts[0], StringComparison.OrdinalIgnoreCase))) || Private.Any(p => p.Equals(relative, StringComparison.OrdinalIgnoreCase)))
                throw new InvalidDataException($"更新不能改动这个文件：{relative}");
        }

        public static string Full(string installDir, string relative)
        {
            Check(relative);
            var root = Path.GetFullPath(installDir).TrimEnd(Path.DirectorySeparatorChar, Path.AltDirectorySeparatorChar) + Path.DirectorySeparatorChar;
            var full = Path.GetFullPath(Path.Combine(root, relative.Replace('/', Path.DirectorySeparatorChar)));
            if (!full.StartsWith(root, StringComparison.OrdinalIgnoreCase)) throw new InvalidDataException($"文件路径越出安装目录：{relative}");
            return full;
        }

        public static string Work(string installDir, params string[] parts) => Path.Combine(new[] { installDir, WorkDir }.Concat(parts).ToArray());
    }
}
