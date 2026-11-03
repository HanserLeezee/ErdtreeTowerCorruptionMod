using SoulsFormats;

static class ElitePoolAssets {
    const int Index=158;
    const uint Original=1052380800;
    static readonly uint[] Slots=[48020298,48020299];
    static byte[] NativeEvent(string game) {
        const string path="/event/m60_52_38_00.emevd.dcx";
        ulong hash=0;foreach(char ch in path)hash=unchecked(hash*0x85+ch);
        foreach(string archive in new[]{"Data0","Data1","Data2","Data3"}) {
            using var header=UXM.CryptographyUtility.DecryptRsa(Path.Combine(game,archive+".bhd"),UXM.ArchiveKeys.EldenRingKeys[archive]);
            var file=BHD5.Read(header,BHD5.Game.EldenRing).Buckets.SelectMany(b=>b).FirstOrDefault(f=>f.FileNameHash==hash);
            if(file==null)continue;
            using var data=File.OpenRead(Path.Combine(game,archive+".bdt"));return file.ReadFile(data);
        }
        throw new Exception("Native Radahn event missing");
    }
    // 保留原版陨石状态与动画；远距落点使用原版玩家分支，去掉红狮子广场旗标和辅助人偶。
    static EMEVD.Event Meteor(EMEVD.Event source,uint entity,long id) {
        if(source.Instructions.Count!=37||source.Instructions[2].Bank!=4||source.Instructions[2].ID!=5||BitConverter.ToInt32(source.Instructions[2].ArgData,8)!=13906)
            throw new Exception("Unexpected native Radahn meteor event");
        var ev=new EMEVD.Event(id,EMEVD.Event.RestBehaviorType.Restart);
        foreach(int slot in new[]{1,2,3,4,6,7,12,13,14,15,16,26,27,28,30,31,32,35,36}) {
            var old=source.Instructions[slot];var data=old.ArgData.ToArray();
            for(int i=0;i+4<=data.Length;i+=4)if(BitConverter.ToUInt32(data,i)==Original)BitConverter.GetBytes(entity).CopyTo(data,i);
            ev.Instructions.Add(new(old.Bank,old.ID,data));
        }
        return ev;
    }
    static bool Same(EMEVD.Event a,EMEVD.Event b)=>a.ID==b.ID&&a.RestBehavior==b.RestBehavior&&a.Parameters.Count==b.Parameters.Count&&a.Instructions.Count==b.Instructions.Count&&a.Instructions.Zip(b.Instructions).All(p=>p.First.Bank==p.Second.Bank&&p.First.ID==p.Second.ID&&p.First.ArgData.SequenceEqual(p.Second.ArgData));
    static void Parameters(string input,string output,BossAssets.Part spec) {
        BossAssets.Append(input,output,Index);
        var path=Path.Combine(output,"regulation.bin");var reg=SoulsFormats.Cryptography.RegulationDecryptor.DecryptERRegulation(path);
        var file=reg.Files.Single(f=>Path.GetFileNameWithoutExtension(f.Name.Replace('\\','/'))=="NpcParam");
        var npcs=PARAM.Read(file.Bytes);npcs.ApplyParamdef(PARAMDEF.XmlDeserialize("work/SoulsRandomizers/diste/Defs/NpcParam.xml"));
        var original=npcs.Rows.Where(r=>r.ID!=BossAssets.Npc(spec,false)&&r.ID!=BossAssets.Npc(spec,true)).ToDictionary(r=>r.ID,r=>new PARAM.Row(r));
        var tiers=TowerScalingAssets.Tiers(input)[spec.Act];
        foreach(bool elite in new[]{false,true})TowerScalingAssets.Apply(npcs.Rows.Single(r=>r.ID==BossAssets.Npc(spec,elite)),tiers[elite?1:2]);
        file.Bytes=npcs.Write();SoulsFormats.Cryptography.RegulationDecryptor.EncryptERRegulation(path,reg);
        var check=SoulsFormats.Cryptography.RegulationDecryptor.DecryptERRegulation(path);
        var reread=PARAM.Read(check.Files.Single(f=>f.Name==file.Name).Bytes);reread.ApplyParamdef(npcs.AppliedParamdef);
        foreach(var row in reread.Rows.Where(r=>original.ContainsKey(r.ID)))foreach(var cell in row.Cells)
            if(!System.Collections.StructuralComparisons.StructuralEqualityComparer.Equals(cell.Value,original[row.ID][cell.Def.InternalName].Value))throw new Exception($"Unrelated NPC changed: {row.ID}");
        foreach(var old in reg.Files.Where(f=>f.Name!=file.Name))if(!old.Bytes.SequenceEqual(check.Files.Single(f=>f.Name==old.Name).Bytes))throw new Exception("Unrelated parameter changed");
    }
    public static void Patch(string game,string input,string output) {
        if(Path.GetFullPath(input)==Path.GetFullPath(output))throw new Exception("Separate staging output required");
        var spec=BossAssets.Parts(input).Single(p=>p.Index==Index);
        var native=MSBE.Read("work/boss-report/maps/m60_13_09_02.msb.dcx").Parts.Enemies.Single(p=>p.EntityID==Original);
        if((spec.Act,spec.Model,spec.Source,spec.Think)!=(2,4730,native.NPCParamID,native.ThinkParamID)||native.ModelName!="c4730")throw new Exception("Radahn source model/AI mismatch");
        var meteor=EMEVD.Read(NativeEvent(game)).Events.Single(e=>e.ID==1252382820);
        AffixTextFix.Patch(input,output);Parameters(input,output,spec);
        var extra=File.ReadAllLines(Path.Combine(input,"party-extra-slots.tsv")).ToList();
        for(int i=0;i<Slots.Length;i++) {
            uint entity=Slots[i];int npc=BossAssets.Npc(spec,i==1);
            string line=$"2|{npc}|4730|{spec.Think}|0|{entity}|{entity+3000000}";
            if(extra.Any(l=>l.Split('|')[5]==entity.ToString()&&l!=line))throw new Exception("Radahn slot already occupied");
            if(!extra.Contains(line))extra.Add(line);
        }
        File.WriteAllLines(Path.Combine(output,"party-extra-slots.tsv"),extra);
        int maps=0,actors=0;var represented=new HashSet<uint>();
        foreach(string path in Directory.EnumerateFiles(Path.Combine(input,"map/mapstudio"),"*.msb.dcx").Order()) {
            var map=MSBE.Read(path);string name=Path.GetFileNameWithoutExtension(Path.GetFileNameWithoutExtension(path));
            uint basis=new[]{45000000u,45010000u,45020000u}.FirstOrDefault(b=>map.Regions.GetEntries().Any(r=>r.EntityID==b+2600));
            if(basis==0)continue;
            var events=EMEVD.Read(Path.Combine(input,"event",name+".emevd.dcx"));
            var before=MSBE.Read(map.Write());var originalEvents=EMEVD.Read(events.Write());
            var init=events.Events.Single(e=>e.ID==0);int prefix=0;var additions=new HashSet<uint>();var eventIds=new HashSet<long>();
            bool modelAdded=!map.Models.Enemies.Any(m=>m.Name=="c4730");
            if(modelAdded)map.Models.Enemies.Add(new MSBE.Model.Enemy{Name="c4730",SibPath=@"N:\GR\data\Model\chr\c4730\sib\c4730.sib"});
            void AddEvent(EMEVD.Event ev) {
                var existing=events.Events.SingleOrDefault(e=>e.ID==ev.ID);
                if(existing!=null){if(!Same(existing,ev))throw new Exception($"Event collision {ev.ID}");return;}
                events.Events.Add(ev);eventIds.Add(ev.ID);
                init.Instructions.Insert(prefix++,new(2000,0,new object[]{0,(uint)ev.ID,0u}));
            }
            for(int slot=0;slot<3;slot++)AddEvent(Meteor(meteor,basis+850+(uint)slot,basis+9820+slot));
            foreach(bool guard in new[]{false,true}) {
                uint first=guard?51020000u:48020000u;
                var anchor=map.Parts.Enemies.FirstOrDefault(p=>p.EntityID>=first&&p.EntityID<first+298);
                if(anchor==null)continue;
                for(int i=0;i<Slots.Length;i++) {
                    uint entity=Slots[i]+(guard?3000000u:0u);int npc=BossAssets.Npc(spec,i==1);represented.Add(entity);
                    var part=map.Parts.Enemies.SingleOrDefault(p=>p.EntityID==entity);
                    if(part==null) {
                        part=(MSBE.Part.Enemy)native.DeepCopy();part.Name=guard?$"tower_guard_{entity}":$"tower_party_actor_{entity}";
                        part.EntityID=entity;part.EntityGroupIDs=new uint[8];part.NPCParamID=npc;part.ThinkParamID=spec.Think;
                        part.InstanceID=map.Parts.Enemies.Where(p=>p.ModelName==native.ModelName).Select(p=>p.InstanceID).DefaultIfEmpty(-1).Max()+1;
                        part.Position=anchor.Position;part.Rotation=anchor.Rotation;part.CollisionPartName=anchor.CollisionPartName;
                        part.TalkID=-1;part.PlatoonID=-1;part.ChrActivateCondParamID=0;part.BackupEventAnimID=-1;part.WalkRouteName=null;
                        part.SpEffectSetParamID=new int[part.SpEffectSetParamID.Length];PartyPlacement.Prepare(part);
                        map.Parts.Enemies.Add(part);additions.Add(entity);actors++;
                        init.Instructions.Insert(prefix++,new(2004,5,new object[]{entity,(byte)0}));
                        uint request=guard?45003300+Slots[i]%10000:PartyEnemyEvents.Request(entity);
                        init.Instructions.Insert(prefix++,new(2003,66,new object[]{(byte)0,request,(byte)0}));
                        if(!guard)init.Instructions.Insert(prefix++,new(2003,66,new object[]{(byte)0,PartyEnemyEvents.Ready(entity),(byte)0}));
                    }
                    if(part.NPCParamID!=npc||part.ModelName!="c4730"||part.ThinkParamID!=spec.Think)throw new Exception("Existing Radahn actor mismatch");
                    foreach(bool battle in new[]{false,true})AddEvent(guard?PartyGuardAssets.ForEntity(Slots[i],battle):battle?PartyEnemyEvents.Battle(entity):PartyEnemyEvents.Create(entity));
                    AddEvent(Meteor(meteor,entity,entity+200000));
                }
            }
            string mapOut=Path.Combine(output,"map/mapstudio",name+".msb.dcx"),eventOut=Path.Combine(output,"event",name+".emevd.dcx");
            Directory.CreateDirectory(Path.GetDirectoryName(mapOut)!);Directory.CreateDirectory(Path.GetDirectoryName(eventOut)!);
            PartyModels.Prepare(map);PartyModels.Verify(map);map.Write(mapOut);events.Write(eventOut);
            var checkMap=MSBE.Read(mapOut);var checkEvents=EMEVD.Read(eventOut);
            foreach(uint entity in additions) {
                var actor=checkMap.Parts.Enemies.Single(p=>p.EntityID==entity);PartyPlacement.Verify(actor);
                if(!checkMap.Parts.Collisions.Any(c=>c.Name==actor.CollisionPartName))throw new Exception("Radahn ground collision missing");
                int disabled=checkEvents.Events.Single(e=>e.ID==0).Instructions.FindIndex(i=>i.Bank==2004&&i.ID==5&&BitConverter.ToUInt32(i.ArgData,0)==entity&&i.ArgData[4]==0);
                if(disabled<0||disabled>=prefix)throw new Exception("Radahn not disabled before activation events");
            }
            foreach(long id in eventIds)if(!Same(checkEvents.Events.Single(e=>e.ID==id),events.Events.Single(e=>e.ID==id)))throw new Exception("Radahn event roundtrip mismatch");
            checkMap.Parts.Enemies.RemoveAll(p=>additions.Contains(p.EntityID));
            if(modelAdded)checkMap.Models.Enemies.RemoveAll(m=>m.Name=="c4730");
            if(!checkMap.Write().SequenceEqual(before.Write()))throw new Exception($"Unrelated map content changed: {name}");
            checkEvents.Events.RemoveAll(e=>eventIds.Contains(e.ID));checkEvents.Events.Single(e=>e.ID==0).Instructions.RemoveRange(0,prefix);
            if(!checkEvents.Write().SequenceEqual(originalEvents.Write()))throw new Exception($"Unrelated events changed: {name}");
            maps++;
        }
        if(maps<19||!represented.SetEquals(Slots.Concat(Slots.Select(id=>id+3000000))))throw new Exception("Incomplete Radahn arena/guard coverage");
        MusicAssets.RefreshArenaManifest(input,output);
        Console.WriteLine($"Verified Radahn: {maps} maps, {actors} added actors; native meteor instructions preserved, original map/event content unchanged");
    }
    public static void Verify(string input) {
        int count=0;
        foreach(var line in File.ReadLines(Path.Combine(input,"arenas-built.txt")).Where(l=>!l.StartsWith('#')&&l.Length>0)) {
            var p=line.Split('|');string name=p[1];
            var map=MSBE.Read(Path.Combine(input,"map/mapstudio",name+".msb.dcx"));
            var events=EMEVD.Read(Path.Combine(input,"event",name+".emevd.dcx"));
            int[] acts=p[0]=="farum_dragonlord"?[1,2,3]:[int.Parse(p[10])];
            foreach(int act in acts)ArenaPartyAssets.Verify(map,events,MSBE.Read(Path.Combine(input,"map/mapstudio",$"m45_0{3-act}_00_00.msb.dcx")),act);
            if(!map.Models.Enemies.Any(m=>m.Name=="c4730")||!events.Events.Any(e=>e.ID==45029820))throw new Exception($"Radahn solo resources missing: {name}");
            if(acts.Contains(2))foreach(uint entity in Slots) {
                if(map.Parts.Enemies.Count(e=>e.EntityID==entity&&e.ModelName=="c4730")!=1||!events.Events.Any(e=>e.ID==entity+200000))throw new Exception($"Radahn party resources missing: {name}");
            }
            count++;
        }
        if(count<16)throw new Exception("Incomplete arena verification");
        Console.WriteLine($"Verified all party slots and Radahn solo/party events in {count} built arenas");
    }
}
