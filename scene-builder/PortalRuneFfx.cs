using SoulsFormats;
using System.Text;
using System.Xml.Linq;

static class PortalRuneFfx {
    public static readonly bool Enabled=false;
    public const int Border=9872100,Rune=9872101;
    public const string FileName="sfx/sfxbnd_commoneffects.ffxbnd.dcx";
    static IEnumerable<FXR3.Container> Containers(FXR3.Container c) {
        yield return c;foreach(var child in c.Containers)foreach(var item in Containers(child))yield return item;
    }
    public static FXR3 Create(FXR3 source,int id) {
        if(source.Id!=806932||id!=Border&&id!=Rune)throw new Exception("Unexpected rune portal source or target");
        var fx=new FXR3(source){Id=id};int removed=0,replaced=0;
        foreach(var c in Containers(fx.RootContainer)) {
            removed+=c.Effects.RemoveAll(e=>e.Actions.Any(a=>a.Id is 10015 or 609));
            foreach(var a in c.Effects.SelectMany(e=>e.Actions).Where(a=>a.Id==604)) {
                if(a.Fields1.Count!=17||a.Fields1[1] is not FXR3.Field.FieldInt{Value:60521})throw new Exception("Native rune renderer layout changed");
                if(id==Border)foreach(var field in a.Fields1.OfType<FXR3.Field.FieldInt>().Where(f=>f.Value==60521)){field.Value=60571;replaced++;}
            }
        }
        if(removed!=2||id==Border&&replaced!=4)throw new Exception("Unexpected native rune layers");
        var copy=FXR3.Read(fx.Write());
        if(!XNode.DeepEquals(FXR3EnhancedSerialization.FXR3ToXML(fx),FXR3EnhancedSerialization.FXR3ToXML(copy)))throw new Exception("Rune FXR roundtrip changed fields");
        var before=FXR3EnhancedSerialization.FXR3ToXML(source);var after=FXR3EnhancedSerialization.FXR3ToXML(copy);
        if(!XNode.DeepEquals(before.Element("StateMap"),after.Element("StateMap")))throw new Exception("Rune lifetime changed");
        return copy;
    }
    public static void Build(string input,string output) {
        var binder=BND4.Read(Path.Combine(input,FileName));var original=binder.Files.ToArray();
        bool Ours(BinderFile f)=>new[]{Border,Rune}.Any(id=>f.Name.EndsWith($"f{id:D9}.fxr")||f.Name.EndsWith($"f{id:D9}.ffxreslist"));
        if(Enabled) {
        var source=binder.Files.Single(f=>f.Name.EndsWith("f000806932.fxr"));
        var list=binder.Files.Single(f=>f.Name.EndsWith("f000806932.ffxreslist"));
        int next=binder.Files.Where(f=>f.Name.EndsWith(".fxr")&&!Ours(f)).Max(f=>f.ID)+1;
        foreach(int id in new[]{Border,Rune}) {
            var fx=Create(FXR3.Read(source.Bytes),id);
            var resources=Encoding.UTF8.GetString(list.Bytes).Split(new[]{'\r','\n'},StringSplitOptions.RemoveEmptyEntries)
                .Where(s=>!s.StartsWith("s84005")&&s!="s00003_m.tif").Select(s=>id==Border?s.Replace("s60521","s60571"):s).ToArray();
            foreach(var resource in resources)if(!binder.Files.Any(f=>f.Name.EndsWith(resource.Replace(".tif",".tpf"))))throw new Exception("Missing rune resource: "+resource);
            var name=source.Name[..(source.Name.LastIndexOf('\\')+1)]+$"f{id:D9}.fxr";
            var resourceName=list.Name[..(list.Name.LastIndexOf('\\')+1)]+$"f{id:D9}.ffxreslist";
            int fxId=binder.Files.SingleOrDefault(f=>f.Name==name)?.ID??next++;
            next=Math.Max(next,fxId+1);
            int listId=checked(fxId+list.ID-source.ID);
            foreach(var (template,entryId,entryName,data) in new[]{(source,fxId,name,fx.Write()),(list,listId,resourceName,Encoding.UTF8.GetBytes(string.Join("\r\n",resources)+"\r\n"))}) {
                if(binder.Files.Any(f=>f.ID==entryId&&f.Name!=entryName))throw new Exception("Rune archive entry collision");
                binder.Files.RemoveAll(f=>f.Name==entryName);
                binder.Files.Add(new BinderFile(template.Flags,entryId,entryName,data){CompressionInfo=template.CompressionInfo});
            }
        }
        } else {binder.Files.RemoveAll(Ours);}
        binder.Files.Sort((a,b)=>a.ID.CompareTo(b.ID));
        var target=Path.Combine(output,FileName);Directory.CreateDirectory(Path.GetDirectoryName(target)!);
        if(!Enabled&&!original.Any(Ours)) {
            if(Path.GetFullPath(input)!=Path.GetFullPath(output))File.Copy(Path.Combine(input,FileName),target,true);
        } else {binder.Write(target);}
        var check=BND4.Read(target).Files.ToDictionary(f=>f.ID);
        if(check.Count!=original.Count(f=>!Ours(f))+(Enabled?4:0))throw new Exception("Unexpected rune portal archive size");
        foreach(var entry in original.Where(f=>!Ours(f))) {
            var actual=check[entry.ID];
            if(actual.Name!=entry.Name||actual.Flags!=entry.Flags||!actual.Bytes.SequenceEqual(entry.Bytes))throw new Exception("Unrelated SFX changed");
        }
        var manifest=Path.Combine(input,"aura-build.json");
        if(File.Exists(manifest)) {
            var data=System.Text.Json.Nodes.JsonNode.Parse(File.ReadAllText(manifest));
            if(data?["sfx"] is {} sfx) {
                sfx["sha256"]=Convert.ToHexString(System.Security.Cryptography.SHA256.HashData(File.ReadAllBytes(target)));
                sfx["addedEntries"]=check.Count-sfx["preservedEntries"]!.GetValue<int>();
                File.WriteAllText(Path.Combine(output,"aura-build.json"),data.ToJsonString(new System.Text.Json.JsonSerializerOptions{WriteIndented=true}));
            }
        }
        Console.WriteLine($"Rune portal effects enabled={Enabled}; {original.Count(f=>!Ours(f))} unrelated SFX files preserved");
    }
}
