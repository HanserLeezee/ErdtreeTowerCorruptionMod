using SoulsFormats;
using System.Numerics;

static class PartyDlcAssets {
    record Enemy(int Act,int Model,int Npc,int Think);
    static Dictionary<int,MSBE.Part.Enemy> Templates(string game,HashSet<int> models) {
        var found=new Dictionary<int,MSBE.Part.Enemy>();
        var paths=File.ReadLines("work/UXM/UXM/res/EldenRingDictionary.txt").Where(p=>p.StartsWith("/map/mapstudio/")&&p.EndsWith(".msb.dcx")&&!p.EndsWith("_99.msb.dcx")).Order().ToArray();
        using var header=UXM.CryptographyUtility.DecryptRsa(Path.Combine(game,"DLC.bhd"),UXM.ArchiveKeys.EldenRingKeys["DLC"]);
        var files=BHD5.Read(header,BHD5.Game.EldenRing).Buckets.SelectMany(b=>b).ToDictionary(f=>f.FileNameHash);
        using var data=File.OpenRead(Path.Combine(game,"DLC.bdt"));
        foreach(var path in paths) {
            ulong hash=0;foreach(var ch in path)hash=unchecked(hash*0x85+ch);
            if(!files.TryGetValue(hash,out var file))continue;
            foreach(var part in MSBE.Read(file.ReadFile(data)).Parts.Enemies) {
                if(part.ModelName?.StartsWith("c")!=true||!int.TryParse(part.ModelName[1..],out var model)||!models.Contains(model)||found.ContainsKey(model))continue;
                found.Add(model,part);Console.WriteLine($"DLC template c{model:D4}: {path}/{part.Name} npc={part.NPCParamID} think={part.ThinkParamID}");
            }
            if(found.Count==models.Count)break;
        }
        if(found.Count!=models.Count)throw new Exception("Missing DLC model templates: "+string.Join(',',models.Except(found.Keys)));
        return found;
    }
    static bool Same(EMEVD.Event a,EMEVD.Event b)=>a.ID==b.ID&&a.RestBehavior==b.RestBehavior&&a.Instructions.Count==b.Instructions.Count&&a.Instructions.Zip(b.Instructions).All(p=>p.First.Bank==p.Second.Bank&&p.First.ID==p.Second.ID&&p.First.ArgData.SequenceEqual(p.Second.ArgData));
    public static void Build(string game,string input,string output) {
        if(Path.GetFullPath(input)==Path.GetFullPath(output))throw new Exception("Separate staging required");
        var enemies=File.ReadAllLines(Path.Combine(input,"enemy-pools.txt")).Select(l=>l.Split('|')).Where(p=>p.Length>5&&p[5]=="dlc").Select(p=>new Enemy(int.Parse(p[0]),int.Parse(p[2]),900000000+int.Parse(p[2])*10000,int.Parse(p[2])*10000)).OrderBy(e=>e.Act).ThenBy(e=>e.Npc).ToArray();
        var templates=Templates(game,enemies.Select(e=>e.Model).ToHashSet());
        var maps=new Dictionary<int,MSBE>();var scripts=new Dictionary<int,EMEVD>();
        for(int act=1;act<=3;act++){string map=$"m45_0{3-act}_00_00";maps[act]=MSBE.Read(Path.Combine(input,"map/mapstudio",map+".msb.dcx"));scripts[act]=EMEVD.Read(Path.Combine(input,"event",map+".emevd.dcx"));}
        var originals=maps.ToDictionary(p=>p.Key,p=>p.Value.Write());
        var oldEvents=scripts.ToDictionary(p=>p.Key,p=>EMEVD.Read(p.Value.Write()).Events.ToDictionary(e=>e.ID));
        var newParts=new Dictionary<int,List<MSBE.Part.Enemy>>();var newModels=new Dictionary<int,List<MSBE.Model.Enemy>>();var newEvents=new Dictionary<int,List<long>>();var initCounts=new Dictionary<int,int>();
        for(int act=1;act<=3;act++){newParts[act]=new();newModels[act]=new();newEvents[act]=new();initCounts[act]=0;}
        var manifest=File.Exists(Path.Combine(input,"party-dlc-slots.tsv"))?File.ReadAllLines(Path.Combine(input,"party-dlc-slots.tsv")).ToList():new List<string>();
        var extra=new List<string>();
        void Add(Enemy enemy,uint id,int act,bool guard,MSBE.Part.Enemy template) {
            var map=maps[act];var events=scripts[act];var init=events.Events.Single(e=>e.ID==0);
            if(map.Parts.GetEntries().Any(p=>p.EntityID==id))throw new Exception($"Entity already exists: {id}");
            var part=(MSBE.Part.Enemy)template.DeepCopy();part.Name=guard?$"tower_guard_{id}":$"tower_party_{act}_{id%10000}";
            part.InstanceID=guard?12000+(enemy.Act-1)*300+(int)(id%10000):6000+(int)(id%10000);
            part.EntityID=id;part.EntityGroupIDs=new uint[8];part.ModelName=$"c{enemy.Model:D4}";part.NPCParamID=enemy.Npc;part.ThinkParamID=enemy.Think;
            part.TalkID=-1;part.PlatoonID=-1;part.ChrActivateCondParamID=0;part.BackupEventAnimID=-1;part.WalkRouteName=null;
            part.SpEffectSetParamID=new int[part.SpEffectSetParamID.Length];
            var anchor=map.Parts.Enemies.First(p=>p.EntityID>=(guard?51010000:48000000+act*10000)&&p.EntityID<(guard?51040000:48000000+act*10000+300));
            part.CollisionPartName=anchor.CollisionPartName;part.Position=anchor.Position;part.Rotation=Vector3.Zero;PartyPlacement.Prepare(part);
            if(!map.Models.Enemies.Any(m=>m.Name==part.ModelName)){var model=new MSBE.Model.Enemy{Name=part.ModelName};map.Models.Enemies.Add(model);newModels[act].Add(model);}
            map.Parts.Enemies.Add(part);newParts[act].Add(part);
            uint original=guard?id-3000000:id;
            foreach(bool battle in new[]{false,true}) {
                var ev=guard?PartyGuardAssets.ForEntity(original,battle):battle?PartyEnemyEvents.Battle(original):PartyEnemyEvents.Create(original);
                if(events.Events.Any(e=>e.ID==ev.ID))throw new Exception("Event collision");events.Events.Add(ev);newEvents[act].Add(ev.ID);
                init.Instructions.Insert(0,new(2000,0,new object[]{0,(uint)ev.ID,0u}));initCounts[act]++;
            }
            uint request=guard?45003000+(original/10000-4801)*300+original%10000:PartyEnemyEvents.Request(id);
            init.Instructions.Insert(0,new(2003,66,new object[]{(byte)0,request,(byte)0}));
            init.Instructions.Insert(0,new(2004,5,new object[]{id,(byte)0}));initCounts[act]+=2;
        }
        for(int act=1;act<=3;act++) {
            uint first=(uint)(48000000+act*10000);var existing=maps[act].Parts.Enemies.Where(p=>p.EntityID>=first&&p.EntityID<first+300).ToArray();
            int ordinal=existing.Length;
            if(!existing.Select(p=>p.EntityID).Order().SequenceEqual(Enumerable.Range(0,ordinal).Select(i=>first+(uint)i)))throw new Exception("Noncontiguous combat slot layout");
            foreach(var enemy in enemies.Where(e=>e.Act==act)) {
                var placed=existing.Where(p=>p.NPCParamID==enemy.Npc).ToArray();
                if(placed.Length>0) {
                    if(placed.Length!=4||placed.Any(p=>p.ModelName!=$"c{enemy.Model:D4}"||p.ThinkParamID!=enemy.Think)||manifest.Count(l=>l.StartsWith($"{act}|{enemy.Npc}|"))!=4)throw new Exception("Incomplete existing DLC slots");
                    continue;
                }
                for(int copy=0;copy<4;copy++) {
                uint id=first+(uint)ordinal++;if(ordinal>300)throw new Exception("Party flag capacity exceeded");
                Add(enemy,id,act,false,templates[enemy.Model]);Add(enemy,id+3000000,2,true,templates[enemy.Model]);
                manifest.Add($"{act}|{enemy.Npc}|{enemy.Model}|{enemy.Think}|{copy}|{id}|{id+3000000}");
                }
            }
            if(act==3) {
                var beast=BossAssets.Parts(input).Single(b=>b.Name=="坠星成兽");
                var enemy=new Enemy(3,beast.Model,BossAssets.Npc(beast,true),beast.Think);
                var placed=existing.SingleOrDefault(p=>p.NPCParamID==enemy.Npc);
                uint id=placed?.EntityID??first+(uint)ordinal++;
                if(ordinal>300)throw new Exception("Party flag capacity exceeded");
                if(placed==null) {
                    var template=maps[beast.Act].Parts.Enemies.First(p=>p.NPCParamID==enemy.Npc);
                    Add(enemy,id,3,false,template);Add(enemy,id+3000000,2,true,template);
                }
                extra.Add($"3|{enemy.Npc}|{enemy.Model}|{enemy.Think}|0|{id}|{id+3000000}");
            }
        }
        Directory.CreateDirectory(Path.Combine(output,"map/mapstudio"));Directory.CreateDirectory(Path.Combine(output,"event"));
        for(int act=1;act<=3;act++) {
            var map=maps[act];var events=scripts[act];string name=$"m45_0{3-act}_00_00";
            PartyModels.Prepare(map);PartyModels.Verify(map);
            map.Write(Path.Combine(output,"map/mapstudio",name+".msb.dcx"));events.Write(Path.Combine(output,"event",name+".emevd.dcx"));
            var reread=MSBE.Read(Path.Combine(output,"map/mapstudio",name+".msb.dcx"));var check=EMEVD.Read(Path.Combine(output,"event",name+".emevd.dcx"));
            foreach(var part in newParts[act]) {
                var actual=reread.Parts.Enemies.Single(p=>p.EntityID==part.EntityID);PartyPlacement.Verify(actual);
                if(actual.ModelName!=part.ModelName||actual.NPCParamID!=part.NPCParamID||actual.ThinkParamID!=part.ThinkParamID)throw new Exception("DLC placement roundtrip failed");
                uint original=part.EntityID>=51010000?part.EntityID-3000000:part.EntityID;
                foreach(bool battle in new[]{false,true}) {var expected=part.EntityID>=51010000?PartyGuardAssets.ForEntity(original,battle):battle?PartyEnemyEvents.Battle(original):PartyEnemyEvents.Create(original);if(!Same(check.Events.Single(e=>e.ID==expected.ID),expected))throw new Exception("DLC event roundtrip failed");}
            }
            var newIds=newParts[act].Select(p=>p.EntityID).ToHashSet();reread.Parts.Enemies.RemoveAll(p=>newIds.Contains(p.EntityID));
            var modelNames=newModels[act].Select(m=>m.Name).ToHashSet();reread.Models.Enemies.RemoveAll(m=>modelNames.Contains(m.Name));
            if(!reread.Write().SequenceEqual(originals[act]))throw new Exception("Unrelated map data changed");
            check.Events.RemoveAll(e=>newEvents[act].Contains(e.ID));check.Events.Single(e=>e.ID==0).Instructions.RemoveRange(0,initCounts[act]);
            foreach(var ev in check.Events)if(!Same(ev,oldEvents[act][ev.ID]))throw new Exception("Unrelated event changed");
        }
        File.WriteAllLines(Path.Combine(output,"party-dlc-slots.tsv"),manifest);
        File.WriteAllLines(Path.Combine(output,"party-extra-slots.tsv"),extra);
        PartyGuardAssets.Verify(output);Console.WriteLine($"Verified {manifest.Count} DLC and {extra.Count} relocated elite slots plus matching guards; all existing placements and events preserved");
    }
}
