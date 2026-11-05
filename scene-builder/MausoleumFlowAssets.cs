using SoulsFormats;
using System.Numerics;

static class MausoleumFlowAssets {
    const uint Actor=45011860,Enabled=45011859,Awake=45011861,Skulls=45011862,Setup=45011863,Down=45011864,Door=45011865;
    const uint SourceActor=1042330400;
    const uint SkullCountBase=45011500;
    const int SkullCount=44,RequiredSkulls=12;
    static bool OwnHelper(long id)=>id>=45011600&&id<45011700||id>=SkullCountBase&&id<SkullCountBase+SkullCount;
    const string Map="m13_00_00_00";
    static uint Remap(uint id)=>id switch {
        SourceActor=>Actor,1242330400=>Down,1242332400=>Awake,1042336420=>Skulls,1242338911=>Door,
        >=1042331400 and <=1042331478=>45011700+id-1042331400,_=>id,
    };
    static bool OwnAsset(MSBE.Part.Asset p)=>p.Name.StartsWith("tower_mausoleum_");
    static void VerifyLocalLoading(MSBE.Part.TileLoadConfig tile) {
        if(tile.MapID.Any(b=>b!=255)||tile.Unk04!=0||tile.Unk0C!=-1||tile.Unk10!=0||tile.CullingHeightBehavior!=-1)
            throw new Exception("Mausoleum retains overworld tile/height culling in a dungeon map");
    }
    static EMEVD.Event Specialize(EMEVD.Event source,byte[] arguments,uint id) {
        var clone=EMEVD.Read(new EMEVD{Events=[source]}.Write()).Events.Single();
        foreach(var p in clone.Parameters)Array.Copy(arguments,p.SourceStartByte,clone.Instructions[(int)p.InstructionIndex].ArgData,p.TargetStartByte,p.ByteCount);
        clone.Parameters.Clear();clone.ID=id;clone.RestBehavior=EMEVD.Event.RestBehaviorType.Restart;
        // Slot-local completion flags must belong to this encounter, not the vanilla common event.
        foreach(var ins in clone.Instructions) {
            if(ins.Bank==2003&&ins.ID==69) {
                ins.ID=66;
                if(ins.ArgData[0]==2){ins.ArgData[0]=0;BitConverter.GetBytes(id).CopyTo(ins.ArgData,4);}
            }
            if(ins.Bank==1003&&ins.ID==101&&ins.ArgData[2]==2){ins.ArgData[2]=0;BitConverter.GetBytes(id).CopyTo(ins.ArgData,4);}
        }
        PersonalEvents.Local(clone);return clone;
    }
    public static void Verify(string root) {
        var map=MSBE.Read(Path.Combine(root,"map/mapstudio",Map+".msb.dcx"));
        var actor=map.Parts.Enemies.Single(e=>e.EntityID==Actor);
        if(actor.BackupEventAnimID!=-1||actor.NPCParamID!=44500012||actor.ThinkParamID!=44500000)throw new Exception("Mausoleum must use the standing Weeping Peninsula actor and native AI");
        var assets=map.Parts.Assets.Where(OwnAsset).ToArray();
        VerifyLocalLoading(actor.TileLoad);
        foreach(var a in assets) {
            VerifyLocalLoading(a.TileLoad);
            if(a.ModelName is "AEG300_003" or "AEG300_004" or "AEG300_005") {
                if(a.Unk1.DrawGroups.Any(v=>v!=uint.MaxValue)||a.Unk1.Condition2!=1)throw new Exception("Visible mausoleum building/door has no dungeon draw group");
            }else if(a.Unk1.DrawGroups.Any(v=>v!=0))throw new Exception("Source-hidden mausoleum proxy must stay hidden");
        }
        var skulls=assets.Where(a=>a.ModelName=="AEG300_006").ToArray();
        if(skulls.Length!=44||skulls.Any(a=>!a.EntityGroupIDs.Contains(Skulls)))throw new Exception("All 44 destructible skulls must retain their remapped damage group");
        var door=assets.Single(a=>a.ModelName=="AEG300_004");
        if(map.Events.ObjActs.Count(e=>e.ObjActPartName==door.Name&&e.EventFlagID==Door&&e.ObjActID==300004)!=1)throw new Exception("Native door interaction is missing");
        var file=EMEVD.Read(Path.Combine(root,"event",Map+".emevd.dcx"));
        var setup=file.Events.Single(e=>e.ID==Setup);PersonalEvents.Verify(setup);
        if(setup.Instructions.Any(i=>i.Bank==2004&&i.ID==1&&i.ArgData[4]==0))throw new Exception("Mausoleum AI is disabled");
        if(setup.Instructions.Any(i=>i.Bank==2003&&i.ID==66&&BitConverter.ToUInt32(i.ArgData,4)==Down&&i.ArgData[8]!=0))throw new Exception("Mausoleum descent is being pre-completed");
        var helpers=file.Events.Where(e=>e.ID>=45011600&&e.ID<45011700).ToArray();
        if(helpers.Length!=50)throw new Exception($"Expected 50 specialized native helpers, got {helpers.Length}");
        foreach(var e in helpers) {PersonalEvents.Verify(e);if(e.Parameters.Count!=0)throw new Exception("Unbound native helper argument");}
        var counts=helpers.SelectMany(e=>e.Instructions).Single(i=>i.Bank==3&&i.ID==10);
        if(BitConverter.ToUInt32(counts.ArgData,4)!=SkullCountBase||BitConverter.ToUInt32(counts.ArgData,8)!=SkullCountBase+SkullCount-1||counts.ArgData[12]!=4||BitConverter.ToInt32(counts.ArgData,16)!=RequiredSkulls)throw new Exception("Expected 12 of 44 explicit skull receipts");
        var watchers=file.Events.Where(e=>e.ID>=SkullCountBase&&e.ID<SkullCountBase+SkullCount).ToArray();
        var targets=watchers.Select(e=>e.Instructions.Single(i=>i.Bank==5&&i.ID==0)).Select(i=>BitConverter.ToUInt32(i.ArgData,4)).Order().ToArray();
        if(!targets.SequenceEqual(skulls.Select(a=>a.EntityID).Order()))throw new Exception("Every skull needs exactly one destruction watcher");
        foreach(var e in watchers)PersonalEvents.Verify(e);
        var first=file.Events.Single(e=>e.ID==0).Instructions[0];
        if(first.Bank!=2000||first.ID!=0||BitConverter.ToUInt32(first.ArgData,4)!=Setup)throw new Exception("Mausoleum initialization must precede map branches");
        Console.WriteLine("Verified standing actor, 44 skulls, 12-skull threshold, native door ObjAct, 50 bound local helpers and startup ordering.");
    }
    public static void Patch(string input,string output,string sourceRoot) {
        if(Path.GetFullPath(input)==Path.GetFullPath(output))throw new Exception("Separate staging required");
        var source=MSBE.Read(Path.Combine(sourceRoot,"m60_10_08_02.msb.dcx"));
        var dungeon=MSBE.Read(Path.Combine(sourceRoot,"m12_03_00_00.msb.dcx"));
        var sourceEvents=EMEVD.Read(Path.Combine(sourceRoot,"m60_42_33_00.emevd.dcx"));
        var common=EMEVD.Read(Path.Combine(sourceRoot,"common_func.emevd.dcx"));
        var vanillaRatio=common.Events.Single(e=>e.ID==90005451).Instructions.Single(i=>i.Bank==5&&i.ID==6);
        if((int)Math.Ceiling(SkullCount*BitConverter.ToSingle(vanillaRatio.ArgData,12))!=20||RequiredSkulls!=12)throw new Exception("Expected native 20-skull threshold relaxed to 12");
        var reg=SoulsFormats.Cryptography.RegulationDecryptor.DecryptERRegulation(Path.Combine(input,"regulation.bin"));
        foreach(var (table,id) in new[]{("NpcParam",44500012),("NpcThinkParam",44500000),("ObjActParam",300004)}) {
            var row=reg.Files.Single(f=>Path.GetFileNameWithoutExtension(f.Name.Replace('\\','/'))==table);
            if(!PARAM.Read(row.Bytes).Rows.Any(r=>r.ID==id))throw new Exception($"Missing native dependency {table}/{id}");
        }
        var map=MSBE.Read(Path.Combine(input,"map/mapstudio",Map+".msb.dcx"));
        var events=EMEVD.Read(Path.Combine(input,"event",Map+".emevd.dcx"));
        var previousActor=map.Parts.Enemies.SingleOrDefault(e=>e.Name=="tower_mausoleum");
        if(events.Events.Any(e=>OwnHelper(e.ID))&&previousActor?.NPCParamID!=44500012)throw new Exception("Reserved helper event IDs already occupied by unrelated content");
        map.Parts.Enemies.RemoveAll(e=>e.Name=="tower_mausoleum");
        var oldNames=map.Parts.Assets.Where(OwnAsset).Select(a=>a.Name).ToHashSet();
        map.Events.ObjActs.RemoveAll(e=>oldNames.Contains(e.ObjActPartName));map.Parts.Assets.RemoveAll(OwnAsset);
        events.Events.RemoveAll(e=>e.ID==Setup||OwnHelper(e.ID));
        var init=events.Events.Single(e=>e.ID==0);
        init.Instructions.RemoveAll(i=>i.Bank==2000&&i.ID==0&&BitConverter.ToUInt32(i.ArgData,4)==Setup);
        var beforeMap=map.Write();var beforeEvents=events.Write();
        var original=source.Parts.Enemies.Single(e=>e.EntityID==SourceActor);
        var site=File.ReadAllLines(Path.Combine(input,"arenas-built.txt")).Single(l=>l.StartsWith("farum_dragonlord|")).Split('|');
        float Parse(int n)=>float.Parse(site[n],System.Globalization.CultureInfo.InvariantCulture);
        var offset=new Vector3(Parse(3),Parse(4),Parse(5))-original.Position;
        var actor=(MSBE.Part.Enemy)original.DeepCopy();actor.Name="tower_mausoleum";actor.EntityID=Actor;actor.Position+=offset;
        actor.TileLoad=dungeon.Parts.Enemies.Single(e=>e.ModelName=="c4450").TileLoad.DeepCopy();
        actor.CollisionPartName=map.Parts.Enemies.Single(e=>e.Name=="c4520_9000").CollisionPartName;
        actor.TalkID=-1;actor.WalkRouteName=null;actor.EntityGroupIDs=new uint[8];PartyPlacement.Prepare(actor);map.Parts.Enemies.Add(actor);
        var assets=new List<MSBE.Part.Asset>();
        foreach(var old in source.Parts.Assets.Where(a=>a.EntityID>=1042331400&&a.EntityID<=1042331478)) {
            if(old.UnkPartNames.Any(n=>!string.IsNullOrEmpty(n)&&n!=old.Name)||!string.IsNullOrEmpty(old.UnkT54PartName)&&old.UnkT54PartName!=old.Name)throw new Exception("Unexpected source part reference");
            var a=(MSBE.Part.Asset)old.DeepCopy();a.Name="tower_mausoleum_"+old.EntityID;a.EntityID=Remap(old.EntityID);a.Position+=offset;a.MapStudioLayer=uint.MaxValue;
            a.UnkPartNames=new string[6];a.UnkT54PartName=null;
            a.EntityGroupIDs=old.EntityGroupIDs.Select(g=>g==1042336420?Skulls:0u).ToArray();
            var drawing=dungeon.Parts.Assets.First(p=>p.ModelName==old.ModelName);
            a.TileLoad=drawing.TileLoad.DeepCopy();a.Unk1=drawing.Unk1.DeepCopy();
            a.Unk1.DrawGroups=drawing.Unk1.DrawGroups.All(v=>v==0)?new uint[8]:Enumerable.Repeat(uint.MaxValue,8).ToArray();
            a.Unk1.DisplayGroups=drawing.Unk1.DisplayGroups.All(v=>v==0)?new uint[8]:Enumerable.Repeat(uint.MaxValue,8).ToArray();
            assets.Add(a);map.Parts.Assets.Add(a);
            if(!map.Models.Assets.Any(m=>m.Name==a.ModelName))map.Models.Assets.Add((MSBE.Model.Asset)source.Models.Assets.Single(m=>m.Name==a.ModelName).DeepCopy());
        }
        if(!map.Models.Enemies.Any(m=>m.Name==actor.ModelName))map.Models.Enemies.Add((MSBE.Model.Enemy)source.Models.Enemies.Single(m=>m.Name==actor.ModelName).DeepCopy());
        var sourceDoor=source.Events.ObjActs.Single(e=>e.ObjActPartName==source.Parts.Assets.Single(a=>a.EntityID==1042331410).Name);
        var door=new MSBE.Event.ObjAct{Name="Tower mausoleum door",ObjActPartName=assets.Single(a=>a.EntityID==45011710).Name,ObjActID=300004,StateType=sourceDoor.StateType,EventFlagID=Door,MapID=-1,UnkE0C=255,UnkS04=0,UnkS08=0,UnkS0C=-1};
        map.Events.ObjActs.Add(door);
        var setup=new EMEVD.Event(Setup,EMEVD.Event.RestBehaviorType.Restart);PersonalEvents.Local(setup);
        void Visible(bool on) {
            setup.Instructions.Add(new(2004,5,new object[]{Actor,(byte)(on?1:0)}));
            foreach(var a in assets)setup.Instructions.Add(new(2005,3,new object[]{a.EntityID,(byte)(on?1:0)}));
        }
        Visible(false);setup.Instructions.Add(new(3,0,new object[]{(sbyte)0,(byte)1,(byte)0,Enabled}));
        setup.Instructions.Add(new(2003,66,new object[]{(byte)0,Awake,(byte)0}));
        setup.Instructions.Add(new(2003,66,new object[]{(byte)0,Actor,(byte)0}));
        Visible(true);setup.Instructions.Add(new(2004,1,new object[]{Actor,(byte)1}));
        var tracked=assets.Where(a=>a.ModelName=="AEG300_006").OrderBy(a=>a.EntityID).ToArray();
        if(tracked.Length!=SkullCount)throw new Exception("Unexpected skull count");
        for(int index=0;index<tracked.Length;index++) {
            uint receipt=SkullCountBase+(uint)index;
            var watch=new EMEVD.Event(receipt,EMEVD.Event.RestBehaviorType.Restart);PersonalEvents.Local(watch);
            watch.Instructions.Add(new(5,0,new object[]{(sbyte)0,(byte)1,tracked[index].EntityID,(sbyte)0,1f}));
            watch.Instructions.Add(new(2003,66,new object[]{(byte)0,receipt,(byte)1}));
            events.Events.Add(watch);
            setup.Instructions.Add(new(2003,66,new object[]{(byte)0,receipt,(byte)0}));
            setup.Instructions.Add(new(2000,0,new object[]{0,receipt,0u}));
        }
        uint helperId=45011600;
        foreach(var ev in sourceEvents.Events.Where(e=>e.ID is 250 or 200).OrderByDescending(e=>e.ID))
        foreach(var call in ev.Instructions.Where(i=>i.Bank==2000&&i.ID==6&&i.ArgData.Length>=12&&BitConverter.ToUInt32(i.ArgData,8)==SourceActor&&BitConverter.ToUInt32(i.ArgData,4) is >=90005450 and <=90005458)) {
            var sourceId=BitConverter.ToUInt32(call.ArgData,4);var args=call.ArgData[8..].ToArray();
            for(int at=0;at+4<=args.Length;at+=4)BitConverter.GetBytes(Remap(BitConverter.ToUInt32(args,at))).CopyTo(args,at);
            if(sourceId==90005456)BitConverter.GetBytes(Down).CopyTo(args,12);
            var helper=Specialize(common.Events.Single(e=>e.ID==sourceId),args,helperId);
            if(sourceId==90005451) {
                int at=helper.Instructions.FindIndex(i=>i.Bank==5&&i.ID==6);
                if(at<0)throw new Exception("Native skull ratio condition missing");
                helper.Instructions[at]=new(3,10,new object[]{(sbyte)1,(byte)0,SkullCountBase,SkullCountBase+SkullCount-1,(sbyte)4,RequiredSkulls});
            }
            events.Events.Add(helper);
            setup.Instructions.Add(new(2003,66,new object[]{(byte)0,helperId,(byte)0}));
            setup.Instructions.Add(new(2000,0,new object[]{0,helperId,0u}));helperId++;
        }
        setup.Instructions.Add(new(3,0,new object[]{(sbyte)0,(byte)0,(byte)0,Enabled}));Visible(false);setup.Instructions.Add(new(1000,4,new object[]{(byte)1}));
        events.Events.Add(setup);init.Instructions.Insert(0,new(2000,0,new object[]{0,Setup,0u}));
        foreach(var dir in new[]{"map/mapstudio","event"})Directory.CreateDirectory(Path.Combine(output,dir));
        map.Write(Path.Combine(output,"map/mapstudio",Map+".msb.dcx"));events.Write(Path.Combine(output,"event",Map+".emevd.dcx"));
        Verify(output);
        var check=MSBE.Read(Path.Combine(output,"map/mapstudio",Map+".msb.dcx"));
        check.Parts.Enemies.RemoveAll(e=>e.Name=="tower_mausoleum");check.Parts.Assets.RemoveAll(OwnAsset);check.Events.ObjActs.RemoveAll(e=>e.Name==door.Name);
        var originalMap=MSBE.Read(beforeMap);check.Models.Assets.RemoveAll(m=>!originalMap.Models.Assets.Any(o=>o.Name==m.Name));check.Models.Enemies.RemoveAll(m=>!originalMap.Models.Enemies.Any(o=>o.Name==m.Name));
        if(!check.Write().SequenceEqual(beforeMap))throw new Exception("Unrelated map content changed");
        var checkEvents=EMEVD.Read(Path.Combine(output,"event",Map+".emevd.dcx"));checkEvents.Events.RemoveAll(e=>e.ID==Setup||OwnHelper(e.ID));checkEvents.Events.Single(e=>e.ID==0).Instructions.RemoveAt(0);
        if(!checkEvents.Write().SequenceEqual(beforeEvents))throw new Exception("Unrelated map events changed");
        MusicAssets.RefreshArenaManifest(input,output);
        Console.WriteLine("Verified unrelated map/event preservation after replacing only the mausoleum assembly.");
    }
}
