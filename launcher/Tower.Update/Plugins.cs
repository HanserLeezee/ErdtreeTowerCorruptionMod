using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Text;
using System.Text.RegularExpressions;

namespace Tower.Update
{
    /// <summary>
    /// 额外 DLL 插件：复制进模组目录的 plugins/，清单是 plugins/natives.toml（ME3 的 [[natives]] 格式，enabled = false 表示停用）。
    /// 两个启动脚本都读这个清单（Launch-Preview.ps1 / Launch-Seamless-Tower.ps1 的 Get-TowerPluginSections）。
    /// plugins/ 不在任何发布清单里，更新不会覆盖或删除；手动加的选项（load_after 等）在启用/停用时原样保留。
    /// </summary>
    public sealed class Plugins
    {
        public const string Folder = "plugins", ListName = "natives.toml";
        const string Header = "# 额外 DLL 插件：启动器管理，也可以手动改（格式同 ME3 的 [[natives]]，enabled = false 表示停用）。说明见“额外DLL插件说明.txt”。";
        static readonly Regex Section = new Regex(@"(?ms)^\s*\[\[natives\]\][^\r\n]*\r?\n.*?(?=^\s*\[\[|\z)");
        static readonly Regex PathLine = new Regex(@"(?m)^\s*path\s*=\s*['""]([^'""\r\n]+)['""]");
        static readonly Regex EnabledLine = new Regex(@"(?m)^\s*enabled\s*=\s*(\w+)[^\r\n]*");
        /// <summary>这些是加载器、无缝或爬塔自己的 DLL，不能当插件再加一遍</summary>
        static readonly Regex Reserved = new Regex(@"^(ersc|dinput8|me3.*|erdtree_tower_\w+)\.dll$", RegexOptions.IgnoreCase);
        /// <summary>下载下来的压缩包常和 DLL 同名放在一起，不跟着复制</summary>
        static readonly string[] Archives = { ".zip", ".7z", ".rar", ".gz", ".tar" };
        const int MaxFiles = 2000;
        const long MaxBytes = 1024L * 1024 * 1024;

        public sealed class Entry
        {
            public string Path { get; set; } = "";
            public string Name { get; set; } = "";
            public bool Enabled { get; set; }
            public bool Exists { get; set; }
        }

        readonly string installDir;
        public Plugins(string installDir) { this.installDir = installDir; }

        public string Dir => System.IO.Path.Combine(installDir, Folder);
        string ListFile => System.IO.Path.Combine(Dir, ListName);

        public List<Entry> List()
        {
            var (_, sections) = Load();
            return sections.Select(ToEntry).Where(e => e != null).Select(e => e!).ToList();
        }

        Entry? ToEntry(string section)
        {
            var path = PathLine.Match(section);
            if (!path.Success) return null;
            var enabled = EnabledLine.Match(section);
            var full = Full(path.Groups[1].Value);
            return new Entry
            {
                Path = path.Groups[1].Value,
                Name = System.IO.Path.GetFileName(path.Groups[1].Value.Replace('\\', '/').Split('/').Last()),
                Enabled = !enabled.Success || !enabled.Groups[1].Value.Equals("false", StringComparison.OrdinalIgnoreCase),
                Exists = full != null && File.Exists(full),
            };
        }

        string? Full(string path)
        {
            try { return System.IO.Path.GetFullPath(System.IO.Path.IsPathRooted(path) ? path : System.IO.Path.Combine(installDir, path.Replace('/', System.IO.Path.DirectorySeparatorChar))); }
            catch (ArgumentException) { return null; }
            catch (NotSupportedException) { return null; }
        }

        (string head, List<string> sections) Load()
        {
            if (!File.Exists(ListFile)) return (Header + "\r\n", new List<string>());
            var text = File.ReadAllText(ListFile);
            var matches = Section.Matches(text).Cast<Match>().ToList();
            var head = matches.Count == 0 ? text : text.Substring(0, matches[0].Index);
            return (head, matches.Select(m => m.Value.TrimEnd() + "\r\n").ToList());
        }

        void Save(string head, IEnumerable<string> sections)
        {
            Directory.CreateDirectory(Dir);
            var text = head.TrimEnd() + "\r\n\r\n" + string.Join("\r\n", sections);
            File.WriteAllText(ListFile, text.TrimEnd() + "\r\n", new UTF8Encoding(false));
        }

        int IndexOf(List<string> sections, string path) =>
            sections.FindIndex(s => PathLine.Match(s) is var m && m.Success && m.Groups[1].Value.Equals(path, StringComparison.OrdinalIgnoreCase));

        public void SetEnabled(string path, bool enabled)
        {
            var (head, sections) = Load();
            var index = IndexOf(sections, path);
            if (index < 0) return;
            var line = "enabled = " + (enabled ? "true" : "false");
            sections[index] = EnabledLine.IsMatch(sections[index])
                ? EnabledLine.Replace(sections[index], line, 1)
                : PathLine.Replace(sections[index], m => m.Value + "\r\n" + line, 1);
            Save(head, sections);
        }

        /// <summary>从清单里去掉；复制进 plugins/ 的文件夹没有别的插件在用时一并删除</summary>
        public void Remove(string path)
        {
            var (head, sections) = Load();
            var index = IndexOf(sections, path);
            if (index < 0) return;
            sections.RemoveAt(index);
            Save(head, sections);
            var parts = path.Replace('\\', '/').Split('/');
            if (parts.Length >= 3 && parts[0].Equals(Folder, StringComparison.OrdinalIgnoreCase))
            {
                var prefix = parts[0] + "/" + parts[1] + "/";
                var stillUsed = sections.Any(s => PathLine.Match(s) is var m && m.Success && m.Groups[1].Value.Replace('\\', '/').StartsWith(prefix, StringComparison.OrdinalIgnoreCase));
                var folder = System.IO.Path.Combine(Dir, parts[1]);
                if (!stillUsed && Directory.Exists(folder)) Directory.Delete(folder, true);
            }
        }

        /// <summary>添加单个 DLL：连同同名的配置文件（x.ini、x.toml…）和同名文件夹一起复制到 plugins/&lt;名字&gt;/</summary>
        public string AddDll(string dll)
        {
            var name = System.IO.Path.GetFileName(dll);
            Check(name);
            var source = System.IO.Path.GetDirectoryName(dll)!;
            var stem = System.IO.Path.GetFileNameWithoutExtension(dll);
            var target = NewFolder(stem);
            Directory.CreateDirectory(target);
            foreach (var file in Directory.EnumerateFiles(source, stem + ".*").Where(f => !Archives.Contains(System.IO.Path.GetExtension(f).ToLowerInvariant())))
                File.Copy(file, System.IO.Path.Combine(target, System.IO.Path.GetFileName(file)));
            var companion = System.IO.Path.Combine(source, stem);
            if (Directory.Exists(companion)) CopyTree(companion, System.IO.Path.Combine(target, stem));
            var relative = $"{Folder}/{System.IO.Path.GetFileName(target)}/{name}";
            Append(new[] { relative });
            return relative;
        }

        /// <summary>添加整个插件文件夹：复制到 plugins/&lt;文件夹名&gt;/，顶层的每个 DLL 都登记（多余的辅助 DLL 可以在列表里停用）</summary>
        public List<string> AddFolder(string folder)
        {
            folder = System.IO.Path.GetFullPath(folder);
            if (System.IO.Path.GetPathRoot(folder)!.TrimEnd('\\', '/').Equals(folder.TrimEnd('\\', '/'), StringComparison.OrdinalIgnoreCase))
                throw new InvalidOperationException("不能把整个磁盘当成插件文件夹");
            var files = Directory.EnumerateFiles(folder, "*", SearchOption.AllDirectories).Take(MaxFiles + 1).ToList();
            if (files.Count > MaxFiles || files.Sum(f => new FileInfo(f).Length) > MaxBytes)
                throw new InvalidOperationException("这个文件夹太大了（超过 2000 个文件或 1 GB），不像是一个 DLL 插件，请选插件自己的文件夹");
            var dlls = Directory.EnumerateFiles(folder, "*.dll").Select(System.IO.Path.GetFileName).Where(n => !Reserved.IsMatch(n!)).ToList();
            if (dlls.Count == 0) throw new InvalidOperationException("文件夹顶层没有可以加载的 DLL");
            var target = NewFolder(System.IO.Path.GetFileName(folder.TrimEnd('\\', '/')));
            CopyTree(folder, target);
            var added = dlls.Select(n => $"{Folder}/{System.IO.Path.GetFileName(target)}/{n}").ToList();
            Append(added);
            return added;
        }

        void Check(string name)
        {
            if (!name.EndsWith(".dll", StringComparison.OrdinalIgnoreCase)) throw new InvalidOperationException("只能添加 .dll 文件");
            if (Reserved.IsMatch(name)) throw new InvalidOperationException($"{name} 是加载器、无缝联机或爬塔自己的文件，不需要也不能作为插件添加");
            if (name.Contains("'")) throw new InvalidOperationException("文件名里不能有单引号");
        }

        string NewFolder(string stem)
        {
            var safe = new string(stem.Select(c => System.IO.Path.GetInvalidFileNameChars().Contains(c) || c == '\'' ? '_' : c).ToArray()).Trim();
            if (safe.Length == 0) safe = "plugin";
            var target = System.IO.Path.Combine(Dir, safe);
            for (var n = 2; Directory.Exists(target); n++) target = System.IO.Path.Combine(Dir, $"{safe}-{n}");
            return target;
        }

        void Append(IEnumerable<string> paths)
        {
            var (head, sections) = Load();
            foreach (var path in paths)
                if (IndexOf(sections, path) < 0) sections.Add($"[[natives]]\r\npath = '{path}'\r\nenabled = true\r\n");
            Save(head, sections);
        }

        static void CopyTree(string from, string to)
        {
            Directory.CreateDirectory(to);
            foreach (var dir in Directory.EnumerateDirectories(from, "*", SearchOption.AllDirectories))
                Directory.CreateDirectory(System.IO.Path.Combine(to, dir.Substring(from.Length).TrimStart('\\', '/')));
            foreach (var file in Directory.EnumerateFiles(from, "*", SearchOption.AllDirectories))
                File.Copy(file, System.IO.Path.Combine(to, file.Substring(from.Length).TrimStart('\\', '/')));
        }
    }
}
