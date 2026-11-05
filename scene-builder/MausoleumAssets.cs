using SoulsFormats;
using System.Numerics;

static class MausoleumAssets {
    const uint Actor=45011860,Enabled=45011859;
    static string PlatformCollision(MSBE map) {
        var name=map.Parts.Enemies.Single(e=>e.Name=="c4520_9000").CollisionPartName;
        if(name==null||!map.Parts.Collisions.Any(c=>c.Name==name))throw new Exception("Dragonlord platform collision missing");
        return name;
    }
    static void Visibility(MSBE.Part.Asset target,MSBE.Part.Asset source) {
        target.Unk1.DisplayGroups=source.Unk1.DisplayGroups.All(v=>v==0)?new uint[8]:Enumerable.Repeat(uint.MaxValue,8).ToArray();
        target.Unk1.DrawGroups=source.Unk1.DrawGroups.All(v=>v==0)?new uint[8]:Enumerable.Repeat(uint.MaxValue,8).ToArray();
    }
    static uint Remap(uint id)=>id switch {12030400=>Actor,12032400=>45011861,12036420=>45011862,>=12031400 and <=12031435=>45011700+id-12031400,_=>id};
    public static void Patch(string input,string output,string sources) {
        const string name="m13_00_00_00";
        var source=MSBE.Read(Path.Combine(sources,"m12_03_00_00.msb.dcx"));
        var original=source.Parts.Enemies.Single(p=>p.EntityID==12030400&&p.ModelName=="c4450");
        var map=MSBE.Read(Path.Combine(input,"map/mapstudio",name+".msb.dcx"));var before=MSBE.Read(map.Write());
        if(map.Parts.Enemies.Any(p=>p.EntityID==Actor)||map.Parts.Assets.Any(p=>p.EntityID is >=45011700 and <=45011735))throw new Exception("Mausoleum slots occupied; use unmodified event-room source");
        var site=File.ReadAllLines(Path.Combine(input,"arenas-built.txt")).Single(l=>l.StartsWith("farum_dragonlord|")).Split('|');
        var center=new Vector3(float.Parse(site[3],System.Globalization.CultureInfo.InvariantCulture),float.Parse(site[4],System.Globalization.CultureInfo.InvariantCulture),float.Parse(site[5],System.Globalization.CultureInfo.InvariantCulture));
        var offset=center-original.Position;
        var actor=(MSBE.Part.Enemy)original.DeepCopy();actor.Name="tower_mausoleum";actor.EntityID=Actor;
        actor.Position+=offset;actor.CollisionPartName=PlatformCollision(map);
        actor.WalkRouteName=null;actor.TalkID=-1;actor.EntityGroupIDs=new uint[8];actor.BackupEventAnimID=30001;PartyPlacement.Prepare(actor);
        if(!map.Models.Enemies.Any(m=>m.Name==actor.ModelName))map.Models.Enemies.Add((MSBE.Model.Enemy)source.Models.Enemies.Single(m=>m.Name==actor.ModelName).DeepCopy());
        map.Parts.Enemies.Add(actor);
        var assets=new List<MSBE.Part.Asset>();
        foreach(var old in source.Parts.Assets.Where(p=>p.EntityID is >=12031400 and <=12031435)) {
            if(old.UnkPartNames.Any(n=>!string.IsNullOrEmpty(n))||!string.IsNullOrEmpty(old.UnkT54PartName))throw new Exception("Unresolved mausoleum part link");
            var part=(MSBE.Part.Asset)old.DeepCopy();part.Name="tower_mausoleum_"+old.EntityID;part.EntityID=Remap(old.EntityID);
            part.EntityGroupIDs=new uint[8];part.Position+=offset;part.MapStudioLayer=uint.MaxValue;
            Visibility(part,old);
            if(!map.Models.Assets.Any(m=>m.Name==part.ModelName))map.Models.Assets.Add((MSBE.Model.Asset)source.Models.Assets.Single(m=>m.Name==part.ModelName).DeepCopy());
            assets.Add(part);map.Parts.Assets.Add(part);
        }
        if(!assets.Any(a=>a.ModelName=="AEG300_003")||!assets.Any(a=>a.ModelName=="AEG300_005"))throw new Exception("Structure or door missing");
        var events=EMEVD.Read(Path.Combine(input,"event",name+".emevd.dcx"));
        var sourceEvents=EMEVD.Read(Path.Combine(sources,"m12_03_00_00.emevd.dcx"));
        var setup=new EMEVD.Event(45011863,EMEVD.Event.RestBehaviorType.Restart);PersonalEvents.Local(setup);
        void Visible(bool enabled) {
            setup.Instructions.Add(new(2004,5,new object[]{Actor,(byte)(enabled?1:0)}));
            foreach(var asset in assets)setup.Instructions.Add(new(2005,3,new object[]{asset.EntityID,(byte)(enabled?1:0)}));
        }
        Visible(false);setup.Instructions.Add(new(3,0,new object[]{(sbyte)0,(byte)1,(byte)0,Enabled}));
        setup.Instructions.Add(new(2003,66,new object[]{(byte)0,Actor,(byte)1}));Visible(true);
        int helpers=0;
        foreach(var ev in sourceEvents.Events.Where(e=>e.ID is 0 or 50))foreach(var cmd in ev.Instructions.Where(i=>i.Bank==2000&&i.ID==6&&i.ArgData.Length>=12&&BitConverter.ToUInt32(i.ArgData,8)==12030400&&BitConverter.ToUInt32(i.ArgData,4) is >=90005450 and <=90005458)) {
            var bytes=cmd.ArgData.ToArray();
            for(int at=8;at+4<=bytes.Length;at+=4)BitConverter.GetBytes(Remap(BitConverter.ToUInt32(bytes,at))).CopyTo(bytes,at);
            for(int at=8;at+4<=bytes.Length;at+=4)if(BitConverter.ToUInt32(bytes,at) is >=12030000 and <=12039999)throw new Exception("Assembly still references the original Deeproot map");
            setup.Instructions.Add(new(cmd.Bank,cmd.ID,bytes));helpers++;
        }
        if(helpers<20)throw new Exception("Incomplete assembly events");
        setup.Instructions.Add(new(2004,1,new object[]{Actor,(byte)0}));
        setup.Instructions.Add(new(3,0,new object[]{(sbyte)0,(byte)0,(byte)0,Enabled}));Visible(false);
        setup.Instructions.Add(new(1000,4,new object[]{(byte)1}));
        events.Events.Add(setup);events.Events.Single(e=>e.ID==0).Instructions.Insert(0,new(2000,0,new object[]{0,45011863u,0u}));
        foreach(string dir in new[]{"map/mapstudio","event"})Directory.CreateDirectory(Path.Combine(output,dir));
        var mapPath=Path.Combine(output,"map/mapstudio",name+".msb.dcx");map.Write(mapPath);
        events.Write(Path.Combine(output,"event",name+".emevd.dcx"));RecoveryText.Patch(input,output);
        var check=MSBE.Read(mapPath);
        if(check.Parts.Enemies.Count(p=>p.EntityID==Actor)!=1||check.Parts.Assets.Count(p=>p.EntityID is >=45011700 and <=45011735)!=assets.Count)throw new Exception("Assembly roundtrip mismatch");
        check.Parts.Enemies.RemoveAll(p=>p.EntityID==Actor);check.Parts.Assets.RemoveAll(p=>p.EntityID is >=45011700 and <=45011735);
        check.Models.Enemies.RemoveAll(m=>!before.Models.Enemies.Any(b=>b.Name==m.Name));check.Models.Assets.RemoveAll(m=>!before.Models.Assets.Any(b=>b.Name==m.Name));
        if(!check.Write().SequenceEqual(before.Write()))throw new Exception("Unrelated map content changed");
        RemoveQuestionRoom(input,output);
        MusicAssets.RefreshArenaManifest(input,output);
        Console.WriteLine($"Moved mausoleum actor, {assets.Count} assets and {helpers} native initializers into {name}; unrelated map content unchanged.");
    }
    public static void TestDisplay() {
        var source=new MSBE.Part.Asset();var target=new MSBE.Part.Asset();
        source.Unk1.DrawGroups=new uint[8];source.Unk1.DisplayGroups=new uint[8];
        Visibility(target,source);
        if(target.Unk1.DrawGroups.Any(v=>v!=0)||target.Unk1.DisplayGroups.Any(v=>v!=0))throw new Exception("Hidden source parts became visible");
        source.Unk1.DisplayGroups[1]=4;
        Visibility(target,source);
        if(target.Unk1.DrawGroups.Any(v=>v!=0)||target.Unk1.DisplayGroups.Any(v=>v!=uint.MaxValue))throw new Exception("Display and draw masks must remain independent");
        source.Unk1.DrawGroups[1]=8;Visibility(target,source);
        if(target.Unk1.DrawGroups.Any(v=>v!=uint.MaxValue)||source.Unk1.DrawGroups[1]!=8)throw new Exception("Visible cross-map part or source preservation failed");
        Console.WriteLine("Verified hidden masks, independent draw/display masks and visible cross-map parts");
    }
    public static void PatchDisplay(string input,string output,string sources) {
        if(Path.GetFullPath(input)==Path.GetFullPath(output))throw new Exception("Separate staging required");
        const string name="m13_00_00_00";
        var map=MSBE.Read(Path.Combine(input,"map/mapstudio",name+".msb.dcx"));var before=map.Write();
        var source=MSBE.Read(Path.Combine(sources,"m12_03_00_00.msb.dcx"));
        var actor=map.Parts.Enemies.Single(e=>e.EntityID==Actor&&e.Name=="tower_mausoleum");
        var oldCollision=actor.CollisionPartName;actor.CollisionPartName=PlatformCollision(map);
        var assets=map.Parts.Assets.Where(a=>a.EntityID is >=45011700 and <=45011735).ToArray();
        var oldMasks=assets.ToDictionary(a=>a.EntityID,a=>(a.Unk1.DrawGroups.ToArray(),a.Unk1.DisplayGroups.ToArray()));
        if(assets.Length!=15)throw new Exception("Unexpected mausoleum asset count");
        foreach(var asset in assets) {
            var original=source.Parts.Assets.Single(a=>Remap(a.EntityID)==asset.EntityID);
            if(asset.ModelName!=original.ModelName||!asset.Name.StartsWith("tower_mausoleum_"))throw new Exception("Unexpected mausoleum asset");
            Visibility(asset,original);
        }
        var mapPath=Path.Combine(output,"map/mapstudio",name+".msb.dcx");Directory.CreateDirectory(Path.GetDirectoryName(mapPath)!);map.Write(mapPath);
        var check=MSBE.Read(mapPath);
        if(check.Parts.Enemies.Single(e=>e.EntityID==Actor).CollisionPartName!=PlatformCollision(check))throw new Exception("Wrong platform collision after roundtrip");
        var hidden=check.Parts.Assets.Count(a=>oldMasks.ContainsKey(a.EntityID)&&a.Unk1.DrawGroups.All(v=>v==0));
        if(hidden!=12)throw new Exception("Expected 12 source-hidden components");
        check.Parts.Enemies.Single(e=>e.EntityID==Actor).CollisionPartName=oldCollision;
        foreach(var asset in check.Parts.Assets.Where(a=>oldMasks.ContainsKey(a.EntityID))){asset.Unk1.DrawGroups=oldMasks[asset.EntityID].Item1;asset.Unk1.DisplayGroups=oldMasks[asset.EntityID].Item2;}
        if(!before.SequenceEqual(check.Write()))throw new Exception("Unrelated map content changed");
        var events=EMEVD.Read(Path.Combine(input,"event",name+".emevd.dcx"));var eventBefore=events.Write();
        var setup=events.Events.Single(e=>e.ID==45011863);PersonalEvents.Verify(setup);
        var last=setup.Instructions.Last();bool append=last.Bank!=1000||last.ID!=4;
        if(append)setup.Instructions.Add(new(1000,4,new object[]{(byte)1}));
        else if(last.ArgData.Length!=4||last.ArgData[0]!=1)throw new Exception("Unexpected visibility event ending");
        var eventPath=Path.Combine(output,"event",name+".emevd.dcx");Directory.CreateDirectory(Path.GetDirectoryName(eventPath)!);events.Write(eventPath);
        var eventCheck=EMEVD.Read(eventPath);var end=eventCheck.Events.Single(e=>e.ID==45011863).Instructions;
        if(end.Last().Bank!=1000||end.Last().ID!=4||end.Last().ArgData[0]!=1)throw new Exception("Visibility event must restart");
        if(append)end.RemoveAt(end.Count-1);
        if(!eventBefore.SequenceEqual(eventCheck.Write()))throw new Exception("Unrelated event changed");
        MusicAssets.RefreshArenaManifest(input,output);
        Console.WriteLine($"Verified collision {oldCollision} -> {actor.CollisionPartName}, {hidden} hidden components and reusable visibility event; unrelated content preserved");
    }
    static void RemoveQuestionRoom(string input,string output) {
        const string mapPath="map/mapstudio/m45_01_00_00.msb.dcx",eventPath="event/m45_01_00_00.emevd.dcx";
        var map=MSBE.Read(Path.Combine(input,mapPath));var events=EMEVD.Read(Path.Combine(input,eventPath));
        var actors=map.Parts.Enemies.Where(p=>p.EntityID==Actor).ToArray();
        var assets=map.Parts.Assets.Where(p=>p.EntityID is >=45011700 and <=45011735).ToArray();
        if(actors.Length==0&&assets.Length==0)return;
        if(actors.Length!=1||actors[0].Name!="tower_mausoleum"||assets.Length!=15||assets.Any(a=>!a.Name.StartsWith("tower_mausoleum_")))throw new Exception("Unexpected question-room mausoleum records");
        var models=actors.Select(a=>a.ModelName).Concat(assets.Select(a=>a.ModelName)).ToHashSet();
        map.Parts.Enemies.RemoveAll(p=>p.EntityID==Actor);map.Parts.Assets.RemoveAll(p=>assets.Contains(p));
        var used=map.Parts.GetEntries().Select(p=>p.ModelName).ToHashSet();
        map.Models.Enemies.RemoveAll(m=>models.Contains(m.Name)&&!used.Contains(m.Name));map.Models.Assets.RemoveAll(m=>models.Contains(m.Name)&&!used.Contains(m.Name));
        if(events.Events.RemoveAll(e=>e.ID==45011863)!=1)throw new Exception("Old mausoleum setup missing");
        var init=events.Events.Single(e=>e.ID==0);
        if(init.Instructions.RemoveAll(i=>i.Bank==2000&&i.ID==0&&i.ArgData.Length==12&&BitConverter.ToUInt32(i.ArgData,4)==45011863)!=1)throw new Exception("Old mausoleum initializer missing");
        map.Write(Path.Combine(output,mapPath));events.Write(Path.Combine(output,eventPath));
        var result=MSBE.Read(Path.Combine(output,mapPath));
        if(result.Parts.Enemies.Any(p=>p.EntityID==Actor)||result.Parts.Assets.Any(p=>p.EntityID is >=45011700 and <=45011735))throw new Exception("Question-room cleanup failed");
        Console.WriteLine("Removed only the previous mausoleum assembly from the question room; event NPCs retained.");
    }
    public static void FixStartup(string input,string output) {
        const string path="event/m13_00_00_00.emevd.dcx";
        var file=EMEVD.Read(Path.Combine(input,path));var original=EMEVD.Read(file.Write());
        var init=file.Events.Single(e=>e.ID==0);
        var call=init.Instructions.Single(i=>i.Bank==2000&&i.ID==0&&i.ArgData.Length==12&&BitConverter.ToUInt32(i.ArgData,4)==45011863);
        init.Instructions.Remove(call);init.Instructions.Insert(0,call);
        Directory.CreateDirectory(Path.Combine(output,"event"));var target=Path.Combine(output,path);file.Write(target);
        var check=EMEVD.Read(target);var result=check.Events.Single(e=>e.ID==0);
        if(result.Instructions[0].Bank!=2000||BitConverter.ToUInt32(result.Instructions[0].ArgData,4)!=45011863)throw new Exception("Mausoleum startup is not first");
        static string Key(EMEVD.Instruction i)=>$"{i.Bank}/{i.ID}/"+Convert.ToHexString(i.ArgData);
        var before=original.Events.Single(e=>e.ID==0).Instructions.Where(i=>Key(i)!=Key(call)).Select(Key);
        if(!before.SequenceEqual(result.Instructions.Skip(1).Select(Key)))throw new Exception("Unrelated constructor order changed");
        foreach(var old in original.Events.Where(e=>e.ID!=0)) {
            var actual=check.Events.Single(e=>e.ID==old.ID);
            if(!new EMEVD{Events=[old]}.Write().SequenceEqual(new EMEVD{Events=[actual]}.Write()))throw new Exception("Unrelated event changed");
        }
        Console.WriteLine("Verified mausoleum initialization before constructor branches and exits; all other instructions/events unchanged.");
    }
}
