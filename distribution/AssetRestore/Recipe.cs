using System;
using System.IO;
using System.Security.Cryptography;

namespace Tower.Assets
{
    public sealed class AssetManifest
    {
        public int Version { get; set; }
        public string OodleHash { get; set; }
        public AssetRecipe[] Files { get; set; }
        public IconRecipe[] Icons { get; set; }
    }
    public sealed class IconRecipe
    {
        public int Id { get; set; }
        public int Offset { get; set; }
        public int Stride { get; set; }
        public string Sha256 { get; set; }
    }
    public sealed class AssetRecipe
    {
        public string Path { get; set; }
        public string Archive { get; set; }
        public long Offset { get; set; }
        public int SourceSize { get; set; }
        public string SourceHash { get; set; }
        public SourceRepair[] Repairs { get; set; }
        public string Patch { get; set; }
        public string PatchHash { get; set; }
        public string Header { get; set; }
        public int DecodedSize { get; set; }
        public string DecodedHash { get; set; }
        public string TargetHash { get; set; }
    }
    public sealed class SourceRepair
    {
        public int Offset { get; set; }
        public string Data { get; set; }
    }
    public static class AssetHash
    {
        public static string Of(byte[] bytes) { using (var sha = SHA256.Create()) return Hex(sha.ComputeHash(bytes)); }
        public static string FileHash(string path) { using (var stream = File.OpenRead(path)) using (var sha = SHA256.Create()) return Hex(sha.ComputeHash(stream)); }
        static string Hex(byte[] bytes) { return BitConverter.ToString(bytes).Replace("-", "").ToLowerInvariant(); }
    }
}
