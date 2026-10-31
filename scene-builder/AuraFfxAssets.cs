using SoulsFormats;

static class AuraFfxAssets {
    public const int Gold=9872000, Blue=9872001;
    const string Asset="sfxbnd_commoneffects.ffxbnd.dcx";
    public static void Build(string game,string output) {
        var cache=Path.Combine(output,"source-cache",Asset);
        if(!File.Exists(cache)) {
            const string path="/sfx/"+Asset;ulong hash=0;foreach(char ch in path)hash=unchecked(hash*0x85+ch);
            foreach(var archive in new[]{"DLC","Data0","Data1","Data2","Data3"}) {
                using var header=UXM.CryptographyUtility.DecryptRsa(Path.Combine(game,archive+".bhd"),UXM.ArchiveKeys.EldenRingKeys[archive]);
                var entry=BHD5.Read(header,BHD5.Game.EldenRing).Buckets.SelectMany(b=>b).FirstOrDefault(f=>f.FileNameHash==hash);
                if(entry==null)continue;
                using var data=File.OpenRead(Path.Combine(game,archive+".bdt"));Directory.CreateDirectory(Path.GetDirectoryName(cache)!);
                File.WriteAllBytes(cache,entry.ReadFile(data));break;
            }
        }
        var binder=BND4.Read(cache);var original=binder.Files.ToArray();
        var effects=binder.Files.Where(f=>f.Name.EndsWith(".fxr")).ToDictionary(f=>BitConverter.ToInt32(f.Bytes,12));
        var persistent=FXR3.Read(effects[4830].Bytes);
        int next=effects.Values.Max(f=>f.ID)+1;
        var added=new List<BinderFile>();
        foreach(var (source,id) in new[]{(440092,Gold),(523812,Blue)}) {
            if(effects.ContainsKey(id)||binder.Files.Any(f=>f.ID==next))throw new Exception("Aura FXR ID collision");
            var entry=effects[source];var fxr=FXR3.Read(entry.Bytes);
            var expected=new FXR3(fxr){Id=id,RootStateMap=new FXR3(persistent).RootStateMap};
            var bytes=expected.Write();var reread=FXR3.Read(bytes);
            if(!System.Xml.Linq.XNode.DeepEquals(FXR3EnhancedSerialization.FXR3ToXML(expected),FXR3EnhancedSerialization.FXR3ToXML(reread)))throw new Exception("Aura FXR round-trip changed fields");
            var name=entry.Name[..(entry.Name.LastIndexOf('\\')+1)]+$"f{id:D9}.fxr";
            var file=new BinderFile(entry.Flags,next++,name,bytes){CompressionInfo=entry.CompressionInfo};
            binder.Files.Add(file);added.Add(file);
            var resources=binder.Files.Single(f=>f.Name.EndsWith($"\\f{source:D9}.ffxreslist"));
            var resourceId=checked(file.ID+resources.ID-entry.ID);
            if(binder.Files.Any(f=>f.ID==resourceId))throw new Exception("Aura resource-list ID collision");
            var resourceName=resources.Name[..(resources.Name.LastIndexOf('\\')+1)]+$"f{id:D9}.ffxreslist";
            var resourceFile=new BinderFile(resources.Flags,resourceId,resourceName,resources.Bytes.ToArray()){CompressionInfo=resources.CompressionInfo};
            binder.Files.Add(resourceFile);added.Add(resourceFile);
        }
        binder.Files.Sort((a,b)=>a.ID.CompareTo(b.ID));
        var target=Path.Combine(output,"sfx",Asset);Directory.CreateDirectory(Path.GetDirectoryName(target)!);binder.Write(target);
        var actual=BND4.Read(target).Files.ToDictionary(f=>f.ID);
        if(actual.Count!=original.Length+4)throw new Exception("Unexpected aura SFX file count");
        foreach(var file in original.Concat(added)) {
            var check=actual[file.ID];
            if(check.Name!=file.Name||check.Flags!=file.Flags||!check.Bytes.SequenceEqual(file.Bytes))throw new Exception("SFX archive entry changed: "+file.Name);
        }
        var manifestPath=Path.Combine(output,"aura-build.json");
        var manifest=System.Text.Json.Nodes.JsonNode.Parse(File.ReadAllText(manifestPath));
        manifest["sfx"]=new System.Text.Json.Nodes.JsonObject {
            ["file"]="sfx/"+Asset,
            ["sha256"]=Convert.ToHexString(System.Security.Cryptography.SHA256.HashData(File.ReadAllBytes(target))),
            ["preservedEntries"]=original.Length,
            ["addedEntries"]=added.Count
        };
        File.WriteAllText(manifestPath,manifest.ToJsonString(new System.Text.Json.JsonSerializerOptions{WriteIndented=true}));
        Console.WriteLine($"Verified two persistent ground FXRs with matching resource lists; preserved all {original.Length} original SFX entries");
    }
}
