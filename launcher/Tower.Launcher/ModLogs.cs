using System.IO;
using System.Linq;

namespace Tower.Launcher
{
    static class ModLogs
    {
        public static string? Latest(string root) => new[]{
            Path.Combine(root,"tower-preview.log"),
            Path.Combine(root,"seamless-entry","runtime","tower-preview.log"),
            Path.Combine(root,"seamless-entry","party-runtime","tower-preview.log")
        }.Where(File.Exists).OrderByDescending(File.GetLastWriteTimeUtc).FirstOrDefault();
    }
}
