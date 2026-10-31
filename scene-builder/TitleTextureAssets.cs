using System.Text;
using System.Xml.Linq;
using SoulsFormats;

static class TitleTextureAssets {
    static readonly string[] Art=["title","loading1","loading2","loading3"];

    public static void Build(string game,string output,string cache) {
        var paths=(from quality in new[]{"hi","low"} from suffix in new[]{"tpf","sblytbnd"} select $"/menu/{quality}/01_common.{suffix}.dcx").ToArray();
        string Cached(string path)=>Path.Combine(cache,path.TrimStart('/'));
        foreach(var archive in new[]{"DLC","Data0","Data1","Data2","Data3"}) {
            var missing=paths.Where(p=>!File.Exists(Cached(p))).ToArray();if(missing.Length==0)break;
            using var header=UXM.CryptographyUtility.DecryptRsa(Path.Combine(game,archive+".bhd"),UXM.ArchiveKeys.EldenRingKeys[archive]);
            var files=BHD5.Read(header,BHD5.Game.EldenRing).Buckets.SelectMany(b=>b).ToDictionary(f=>f.FileNameHash);
            using var data=File.OpenRead(Path.Combine(game,archive+".bdt"));
            foreach(var path in missing) {
                ulong hash=0;foreach(char ch in path)hash=unchecked(hash*0x85+ch);
                if(!files.TryGetValue(hash,out var file))continue;
                Directory.CreateDirectory(Path.GetDirectoryName(Cached(path))!);File.WriteAllBytes(Cached(path),file.ReadFile(data));
            }
        }
        foreach(var quality in new[]{"hi","low"}) {
            var originalTpf=Cached($"/menu/{quality}/01_common.tpf.dcx");
            var originalLayout=Cached($"/menu/{quality}/01_common.sblytbnd.dcx");
            var tpf=TPF.Read(originalTpf);var binder=BND4.Read(originalLayout);
            var textures=tpf.Textures.ToArray();var layouts=binder.Files.ToArray();
            if(textures.Any(t=>t.Name.StartsWith("SB_TowerTheme_")))throw new Exception("Use unmodified game texture archives as the cache");
            var template=layouts[0];int id=layouts.Max(f=>f.ID)+1;
            foreach(var name in Art) {
                string atlas="SB_TowerTheme_"+name, resource="MENU_TowerTheme_"+name;
                var bytes=File.ReadAllBytes(Path.Combine(output,"theme-textures",name+".dds"));var dds=new DDS(bytes);
                if(dds.dwWidth!=1920||dds.dwHeight!=1080||bytes.Length!=128+1920*1080*4||
                    BitConverter.ToUInt32(bytes,80)!=65||BitConverter.ToUInt32(bytes,88)!=32||
                    BitConverter.ToUInt32(bytes,92)!=0x00ff0000||BitConverter.ToUInt32(bytes,96)!=0x0000ff00||
                    BitConverter.ToUInt32(bytes,100)!=0x000000ff||BitConverter.ToUInt32(bytes,104)!=0xff000000)throw new Exception("Expected full-screen BGRA8 DDS: "+name);
                tpf.Textures.Add(new TPF.Texture(atlas,9,0,bytes,TPF.TPFPlatform.PC){Mipmaps=1});
                var xml=new XElement("TextureAtlas",new XAttribute("imagePath",atlas+".png"),
                    new XElement("SubTexture",new XAttribute("name",resource+".png"),new XAttribute("x",0),new XAttribute("y",0),
                        new XAttribute("width",1920),new XAttribute("height",1080),new XAttribute("half",0)));
                var fileName=template.Name[..(template.Name.LastIndexOf('\\')+1)]+atlas+".layout";
                binder.Files.Add(new BinderFile(template.Flags,id++,fileName,Encoding.UTF8.GetBytes(xml.ToString())){CompressionInfo=template.CompressionInfo});
            }
            var folder=Path.Combine(output,"menu",quality);Directory.CreateDirectory(folder);
            var tpfPath=Path.Combine(folder,"01_common.tpf.dcx");var layoutPath=Path.Combine(folder,"01_common.sblytbnd.dcx");
            tpf.Write(tpfPath);binder.Write(layoutPath);
            var check=TPF.Read(tpfPath);var layoutCheck=BND4.Read(layoutPath);
            if(check.Textures.Count!=textures.Length+Art.Length||layoutCheck.Files.Count!=layouts.Length+Art.Length)throw new Exception("Theme archive count mismatch");
            foreach(var original in textures) {
                var actual=check.Textures.Single(t=>t.Name==original.Name);
                if(!actual.Bytes.SequenceEqual(original.Bytes)||actual.Format!=original.Format||actual.Flags1!=original.Flags1||actual.Mipmaps!=original.Mipmaps||actual.Type!=original.Type)throw new Exception("Original texture changed: "+original.Name);
            }
            foreach(var original in layouts) {
                var actual=layoutCheck.Files.Single(f=>f.ID==original.ID);
                if(actual.Name!=original.Name||actual.Flags!=original.Flags||!actual.Bytes.SequenceEqual(original.Bytes))throw new Exception("Original layout changed: "+original.Name);
            }
            foreach(var name in Art) {
                var texture=check.Textures.Single(t=>t.Name=="SB_TowerTheme_"+name);
                if(!texture.Bytes.SequenceEqual(File.ReadAllBytes(Path.Combine(output,"theme-textures",name+".dds"))))throw new Exception("Theme DDS changed: "+name);
                var layout=layoutCheck.Files.Single(f=>f.Name.EndsWith("\\SB_TowerTheme_"+name+".layout"));
                var xml=XElement.Parse(Encoding.UTF8.GetString(layout.Bytes));
                if((string)xml.Attribute("imagePath")!=texture.Name+".png"||(string)xml.Element("SubTexture")?.Attribute("name")!="MENU_TowerTheme_"+name+".png")throw new Exception("Theme texture mapping mismatch");
            }
            Console.WriteLine($"{quality}: added {Art.Length} theme textures and layouts; preserved {textures.Length} textures and {layouts.Length} layouts");
        }
    }
}
