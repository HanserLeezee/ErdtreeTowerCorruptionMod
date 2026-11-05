using SoulsFormats;
using System.Globalization;

static class PortalArchAssets {
    const uint Portal=45021900;
    static bool VisualPart(uint id)=>PortalReveal.VisualAsset(id,Portal);
    static bool VisualRegion(uint id)=>id==PortalReveal.RiseRegion(Portal)||id==PortalReveal.RingRegion(Portal);
    public static void RemoveFlow(string input,string output) {
        if(Path.GetFullPath(input)==Path.GetFullPath(output))throw new Exception("Separate staging required");
        int count=0;
        foreach(var line in File.ReadAllLines(Path.Combine(input,"arenas-built.txt")).Where(l=>l.Length>0&&!l.StartsWith('#'))) {
            var p=line.Split('|');if(p.Length<13||p[12]!="1")continue;
            var name=$"event/{p[1]}.emevd.dcx";var events=EMEVD.Read(Path.Combine(input,name));var before=events.Write();
            var visual=PortalReveal.Visual(events,Portal);
            if(visual.Parameters.Count!=0)throw new Exception("Unexpected parameterized portal visual");
            var removed=visual.Instructions.Select((instruction,index)=>(instruction,index)).Where(v=>v.instruction.Bank==2006&&v.instruction.ID==4
                &&v.instruction.ArgData.Length==12&&BitConverter.ToUInt32(v.instruction.ArgData)==PortalReveal.BorderEntity(Portal)
                &&BitConverter.ToInt32(v.instruction.ArgData,4)==PortalReveal.FlowDummy&&BitConverter.ToInt32(v.instruction.ArgData,8)==PortalReveal.FlowEffect).ToArray();
            if(removed.Length>1)throw new Exception("Duplicate portal flow effects");
            foreach(var (instruction,index) in removed)visual.Instructions.RemoveAt(index);
            var target=Path.Combine(output,name);Directory.CreateDirectory(Path.GetDirectoryName(target)!);events.Write(target);
            var check=EMEVD.Read(target);PortalReveal.Verify(MSBE.Read(Path.Combine(input,$"map/mapstudio/{p[1]}.msb.dcx")),check,Portal,p[0]);
            foreach(var (instruction,index) in removed)PortalReveal.Visual(check,Portal).Instructions.Insert(index,instruction);
            if(!before.SequenceEqual(check.Write()))throw new Exception("Unrelated event changed while removing portal flow");
            count++;
        }
        if(count==0)throw new Exception("No built portals found");
        MusicAssets.RefreshArenaManifest(input,output);
        Console.WriteLine($"Removed upright flow from {count} portals; native rune, ground light, map geometry and other events preserved");
    }
    public static void Patch(string game,string input,string output) {
        if(Path.GetFullPath(input)==Path.GetFullPath(output))throw new Exception("Separate staging required");
        PortalVisualAssets.Build(game,input,output);
        PortalRuneFfx.Build(input,output);
        var effects=BND4.Read(Path.Combine(output,PortalRuneFfx.FileName));
        if(PortalRuneFfx.Enabled)throw new Exception("Experimental portal FXRs must remain disabled");
        foreach(int id in new[]{AuraFfxAssets.Gold,PortalReveal.RuneEffect})foreach(string extension in new[]{"fxr","ffxreslist"})
            if(!effects.Files.Any(f=>f.Name.EndsWith($"f{id:D9}.{extension}")))throw new Exception($"Missing portal dependency {id}.{extension}");
        var resources=effects.Files.Single(f=>f.Name.EndsWith($"f{PortalReveal.RuneEffect:D9}.ffxreslist"));
        foreach(var resource in System.Text.Encoding.UTF8.GetString(resources.Bytes).Split(new[]{'\r','\n'},StringSplitOptions.RemoveEmptyEntries)) {
            var name=resource.Replace(".tif",".tpf").Replace(".sib",".flver");
            if(!effects.Files.Any(f=>Path.GetFileName(f.Name.Replace('\\','/'))==name))throw new Exception("Missing native rune resource: "+resource);
        }
        int count=0;
        foreach(var line in File.ReadAllLines(Path.Combine(input,"arenas-built.txt")).Where(l=>l.Length>0&&!l.StartsWith('#'))) {
            var p=line.Split('|');if(p.Length<13||p[12]!="1")continue;
            string mapName=$"map/mapstudio/{p[1]}.msb.dcx",eventName=$"event/{p[1]}.emevd.dcx";
            var baseline=MSBE.Read(Path.Combine(input,mapName));var map=MSBE.Read(Path.Combine(input,mapName));
            var original=EMEVD.Read(Path.Combine(input,eventName));var events=EMEVD.Read(Path.Combine(input,eventName));
            PortalReveal.ClearMap(map,Portal);
            map.Parts.Assets.Single(a=>a.EntityID==Portal).Position=new(float.Parse(p[3],CultureInfo.InvariantCulture),float.Parse(p[4],CultureInfo.InvariantCulture)+.03f,float.Parse(p[5],CultureInfo.InvariantCulture));
            PortalReveal.AddRegion(map,Portal);PortalReveal.Apply(events,Portal);
            string mapOut=Path.Combine(output,mapName),eventOut=Path.Combine(output,eventName);
            Directory.CreateDirectory(Path.GetDirectoryName(mapOut)!);Directory.CreateDirectory(Path.GetDirectoryName(eventOut)!);map.Write(mapOut);events.Write(eventOut);
            var check=MSBE.Read(mapOut);var checkEvents=EMEVD.Read(eventOut);PortalReveal.Verify(check,checkEvents,Portal,p[0]);
            var visual=PortalReveal.Visual(checkEvents,Portal);
            foreach(var ev in original.Events.Where(e=>e.ID!=0&&e.ID!=visual.ID)) {
                var other=checkEvents.Events.Single(e=>e.ID==ev.ID);
                if(!EventBytes(ev).SequenceEqual(EventBytes(other)))throw new Exception("Unrelated event changed");
            }
            string[] Guards(EMEVD.Event e)=>e.Instructions.Where(i=>i.Bank<1001&&!(i.Bank==1000&&i.ID==4)).Select(i=>$"{i.Bank}/{i.ID}/"+Convert.ToHexString(i.ArgData)).ToArray();
            if(!Guards(original.Events.Single(e=>e.ID==visual.ID)).SequenceEqual(Guards(visual)))throw new Exception("Victory gate changed");
            // Restore only portal-owned records and compare the complete map.
            check.Parts.Assets.RemoveAll(a=>VisualPart(a.EntityID));
            foreach(var (part,index) in baseline.Parts.Assets.Select((a,i)=>(a,i)).Where(v=>VisualPart(v.a.EntityID)))check.Parts.Assets.Insert(index,(MSBE.Part.Asset)part.DeepCopy());
            check.Regions.SFX.RemoveAll(r=>VisualRegion(r.EntityID));
            foreach(var (region,index) in baseline.Regions.SFX.Select((r,i)=>(r,i)).Where(v=>VisualRegion(v.r.EntityID)))check.Regions.SFX.Insert(index,(MSBE.Region.SFX)region.DeepCopy());
            check.Models.Assets.RemoveAll(m=>!baseline.Models.Assets.Any(b=>b.Name==m.Name));
            if(!check.Write().SequenceEqual(baseline.Write()))throw new Exception("Unrelated map content changed");
            Console.WriteLine($"{p[0]}: ornate arch, intact native rune, original interaction and unrelated content verified");count++;
        }
        if(count==0)throw new Exception("No built arenas found");
        MusicAssets.RefreshArenaManifest(input,output);PortalVisualAssets.Manifest(output);Console.WriteLine($"Verified ornate portals in {count} arenas");
    }
    static byte[] EventBytes(EMEVD.Event e)=>new EMEVD{Events=[e]}.Write();
}
