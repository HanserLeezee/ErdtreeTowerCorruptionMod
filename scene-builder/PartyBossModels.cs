using SoulsFormats;

// 追加的多人 Boss 在 boss-pool.txt 里换了模型或 AI（例如蕾娜菈从一阶段 c2030 换成二阶段 c2031）后，同步地图里已经放好的联机实体：
// party-extra-slots.tsv 里的战斗槽（tower_party_*）与守卫槽（tower_guard_*）只改模型名与 AI，实体编号、位置、事件都不动；
// 新模型补进地图的模型表，旧模型保留（二阶段的贴图借用一阶段模型）。可重复运行，没有要改的地图不输出
static class PartyBossModels {
    public static void Sync(string input,string output) {
        if(Path.GetFullPath(input)==Path.GetFullPath(output))throw new Exception("Separate staging output required");
        var parts=BossAssets.Parts(input).ToDictionary(p=>BossAssets.Npc(p,false));
        var wanted=new Dictionary<uint,(string Model,int Think)>();
        foreach(var line in File.ReadAllLines(Path.Combine(input,"party-extra-slots.tsv"))) {
            var p=line.Split('|').Select(uint.Parse).ToArray();
            if(!parts.TryGetValue((int)p[1],out var boss))continue;
            if(boss.Model!=p[2]||boss.Think!=p[3])throw new Exception($"party-extra-slots.tsv disagrees with boss-pool.txt: {boss.Name}");
            foreach(var entity in p[5..7])wanted.Add(entity,($"c{boss.Model:D4}",boss.Think));
        }
        int changed=0;var seen=new HashSet<uint>();
        foreach(var path in Directory.EnumerateFiles(Path.Combine(input,"map/mapstudio"),"*.msb.dcx").Order()) {
            var map=MSBE.Read(path);int here=0;
            foreach(var part in map.Parts.Enemies.Where(e=>wanted.ContainsKey(e.EntityID))) {
                if(!part.Name.StartsWith("tower_"))throw new Exception($"Unexpected entity {part.EntityID} in {Path.GetFileName(path)}");
                var (model,think)=wanted[part.EntityID];seen.Add(part.EntityID);
                if(part.ModelName==model&&part.ThinkParamID==think)continue;
                Console.WriteLine($"{Path.GetFileName(path)} {part.Name} ({part.EntityID}): {part.ModelName}/{part.ThinkParamID} -> {model}/{think}");
                part.ModelName=model;part.ThinkParamID=think;here++;
                if(!map.Models.Enemies.Any(m=>m.Name==model))map.Models.Enemies.Add(new MSBE.Model.Enemy{Name=model});
            }
            if(here==0)continue;
            PartyModels.Prepare(map);PartyModels.Verify(map);
            var dest=Path.Combine(output,"map/mapstudio",Path.GetFileName(path));Directory.CreateDirectory(Path.GetDirectoryName(dest)!);map.Write(dest);
            var before=MSBE.Read(path);var after=MSBE.Read(dest);PartyModels.Verify(after);
            if(after.Parts.Enemies.Count!=before.Parts.Enemies.Count||after.Models.Enemies.Count<before.Models.Enemies.Count)throw new Exception("Unexpected map entry count");
            foreach(var (a,b) in after.Parts.Enemies.Zip(before.Parts.Enemies)) {
                bool ours=wanted.TryGetValue(a.EntityID,out var w);
                if(a.Name!=b.Name||a.EntityID!=b.EntityID||a.NPCParamID!=b.NPCParamID||a.Position!=b.Position||a.CollisionPartName!=b.CollisionPartName)throw new Exception($"Map part changed: {a.Name}");
                if(ours?(a.ModelName!=w.Model||a.ThinkParamID!=w.Think):(a.ModelName!=b.ModelName||a.ThinkParamID!=b.ThinkParamID))throw new Exception($"Map part model mismatch: {a.Name}");
            }
            changed+=here;
        }
        if(!seen.SetEquals(wanted.Keys))throw new Exception("Missing party boss entities: "+string.Join(',',wanted.Keys.Except(seen)));
        Console.WriteLine($"Updated {changed} party boss entities ({seen.Count} checked); other map parts unchanged");
    }
}
