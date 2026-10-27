using System;
using System.IO;
using System.Linq;
using System.Text;
using System.Text.RegularExpressions;

namespace Tower.Launcher
{
    internal static class LaunchResult
    {
        internal static string Summary(string errorFile, string logs, int exitCode)
        {
            var reason = Tail(errorFile).Trim();
            if (reason.Length == 0) {
                var lines = (Tail(Path.Combine(logs, "stdout.log")) + "\n" + Tail(Path.Combine(logs, "stderr.log")))
                    .Split(new[] {'\r','\n'}, StringSplitOptions.RemoveEmptyEntries);
                reason = lines.LastOrDefault(l => Regex.IsMatch(l, "error|exception|failed|fatal|错误|失败", RegexOptions.IgnoreCase)) ?? "";
            }
            reason = reason.Replace('\r', ' ').Replace('\n', ' ').Trim();
            return reason.Length == 0 ? $"脚本退出码 {exitCode}，请打开日志查看详情" : reason.Substring(0, Math.Min(300, reason.Length));
        }
        static string Tail(string path)
        {
            try {
                using var file = new FileStream(path, FileMode.Open, FileAccess.Read, FileShare.ReadWrite | FileShare.Delete);
                file.Seek(Math.Max(0, file.Length - 16384), SeekOrigin.Begin);
                using var reader = new StreamReader(file, Encoding.UTF8, true);
                return reader.ReadToEnd();
            } catch (IOException) { return ""; }
            catch (UnauthorizedAccessException) { return ""; }
        }
    }
}
