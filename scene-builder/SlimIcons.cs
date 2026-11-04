using SoulsFormats;
using Tower.Assets;
using System.Xml.Linq;

static class SlimIcons
{
    public static IconRecipe[] Build(string game, string assets, string output)
    {
        var codec = Environment.GetEnvironmentVariable("TOWER_ICON_CODEC") ?? Path.Combine(output, "Tower.IconCodec.dll");
        System.Runtime.InteropServices.NativeLibrary.Load(Path.GetFullPath(codec));
        const string layoutPath = "/menu/hi/01_common.sblytbnd.dcx";
        ulong hash = 0; foreach (char c in layoutPath) hash = unchecked(hash * 0x85 + c);
        BND4 layout = null;
        foreach (var archive in new[] { "DLC", "Data0", "Data1", "Data2", "Data3" })
        {
            using var header = UXM.CryptographyUtility.DecryptRsa(Path.Combine(game, archive + ".bhd"), UXM.ArchiveKeys.EldenRingKeys[archive]);
            var entry = BHD5.Read(header, BHD5.Game.EldenRing).Buckets.SelectMany(b => b).FirstOrDefault(f => f.FileNameHash == hash);
            if (entry == null) continue;
            using var data = File.OpenRead(Path.Combine(game, archive + ".bdt")); layout = BND4.Read(entry.ReadFile(data)); break;
        }
        if (layout == null) throw new Exception("Missing native icon layout");
        var packed = File.ReadAllBytes(Path.Combine(assets, "menu/hi/01_common.tpf.dcx"));
        var decoded = DCX.Decompress(packed);
        var textures = TPF.Read(packed).Textures.ToDictionary(t => t.Name);
        var recipes = new Dictionary<int, IconRecipe>();
        foreach (var file in layout.Files.Where(f => f.Name.Contains("SB_Icon")))
        {
            var xml = XElement.Parse(System.Text.Encoding.UTF8.GetString(file.Bytes));
            var atlasName = Path.GetFileNameWithoutExtension((string)xml.Attribute("imagePath"));
            if (!textures.TryGetValue(atlasName, out var texture)) continue;
            var dds = texture.Bytes;
            if (dds.Length < 148 || System.Text.Encoding.ASCII.GetString(dds, 84, 4) != "DX10" || BitConverter.ToInt32(dds, 128) != 98)
                throw new Exception("Expected BC7 icon atlas: " + atlasName);
            int start = decoded.AsSpan().IndexOf(dds), width = BitConverter.ToInt32(dds, 16);
            if (start < 0 || width <= 0 || width % 4 != 0) throw new Exception("Cannot locate native DDS: " + atlasName);
            foreach (var item in xml.Elements())
            {
                var name = (string)item.Attribute("name");
                if (name == null || !name.StartsWith("MENU_ItemIcon_") || !name.EndsWith(".png") || !int.TryParse(name[14..^4], out var id) || id >= 990000) continue;
                var path = Path.Combine(assets, "reward-icons", id + ".rgba"); if (!File.Exists(path)) continue;
                int x = (int)item.Attribute("x"), y = (int)item.Attribute("y"), w = (int)item.Attribute("width"), h = (int)item.Attribute("height");
                if (w != 160 || h != 160 || x < 0 || y < 0 || x % 4 != 0 || y % 4 != 0 || x + w > width) continue;
                int stride = width / 4 * 16, offset = checked(start + 148 + y / 4 * stride + x / 4 * 16);
                var pixels = IconCodec.Decode(decoded, offset, stride);
                if (!pixels.SequenceEqual(File.ReadAllBytes(path))) continue;
                recipes[id] = new IconRecipe { Id = id, Offset = offset, Stride = stride, Sha256 = AssetHash.Of(pixels) };
            }
        }
        Console.WriteLine($"Verified {recipes.Count} native icons byte-for-byte; custom or unmatched icons stay in the download");
        return recipes.Values.OrderBy(i => i.Id).ToArray();
    }
}
