using SoulsFormats;

static class StormveilAssets {
    const string Map="m10_00_00_00";
    public static void Build(string output) {
        var map=MSBE.Read($"work/boss-report/maps/{Map}.msb.dcx");
        var events=EMEVD.Read($"work/scene-cache/{Map}.emevd.dcx");
        var entry=map.Parts.Players.Single(p=>p.EntityID==10000988);
        foreach(var player in map.Parts.Players){player.Position=entry.Position;player.Rotation=entry.Rotation;}
        var init=events.Events.Single(e=>e.ID==0);
        StripGraces(events);
        bool Blocked(uint id)=>id is 10002500 or 10002501||id>=10002800&&id<10002900;
        foreach(var e in events.Events) e.Instructions.RemoveAll(i=>i.Bank==2000&&(i.ID==0||i.ID==6)&&i.ArgData.Length>=8&&Blocked(BitConverter.ToUInt32(i.ArgData,4)));
        events.Events.RemoveAll(e=>Blocked((uint)e.ID));
        var setup=new EMEVD.Event(10007980,EMEVD.Event.RestBehaviorType.Restart);PersonalEvents.Local(setup);
        foreach(uint id in new uint[]{10001500,10001501})setup.Instructions.Add(new(2005,7,new object[]{id,1}));
        foreach(var enemy in map.Parts.Enemies.Where(e=>e.EntityID is 10000800 or 10000850)) {
            setup.Instructions.Add(new(2004,5,new object[]{enemy.EntityID,(byte)0}));
            setup.Instructions.Add(new(2004,29,new object[]{enemy.EntityID,(byte)1}));
        }
        setup.Instructions.Add(new(2006,4,new object[]{10001800u,101,5}));
        events.Events.Add(setup);init.Instructions.Insert(0,new(2000,0,new object[]{0,10007980u,0u}));
        var cleanup=new EMEVD.Event(10007981,EMEVD.Event.RestBehaviorType.Restart);PersonalEvents.Local(cleanup);
        cleanup.Instructions.Add(new(3,0,new object[]{(sbyte)0,(byte)0,(byte)0,10007990u}));
        uint anonymous=10006000;
        foreach(var enemy in map.Parts.Enemies) {
            if(enemy.EntityID==0||enemy.EntityID==uint.MaxValue) {
                while(map.Parts.GetEntries().Any(p=>p.EntityID==anonymous))anonymous++;
                enemy.EntityID=anonymous++;
            }
            cleanup.Instructions.Add(new(2004,5,new object[]{enemy.EntityID,(byte)0}));
        }
        cleanup.Instructions.Add(new(1001,0,new object[]{0.2f}));
        cleanup.Instructions.Add(new(1000,4,new object[]{(byte)1}));
        events.Events.Add(cleanup);init.Instructions.Insert(0,new(2000,0,new object[]{0,10007981u,0u}));
        Directory.CreateDirectory(Path.Combine(output,"map/mapstudio"));Directory.CreateDirectory(Path.Combine(output,"event"));
        map.Write(Path.Combine(output,$"map/mapstudio/{Map}.msb.dcx"));
        events.Write(Path.Combine(output,$"event/{Map}.emevd.dcx"));
        Verify(output);
    }
    // 跑城中坐火会把重生点记到史东薇尔，之后用返回赐福的道具就传回城里卡住；
    // 去掉这张图的“注册赐福”（2009[3] RegisterBonfire），城里的赐福不能点亮也不能休息，只剩装饰
    static bool RegistersGrace(EMEVD.Instruction i)=>i.Bank==2009&&i.ID==3;
    static List<(long,uint,uint)> StripGraces(EMEVD events) {
        var removed=new List<(long,uint,uint)>();
        foreach(var e in events.Events) {
            for(int index=e.Instructions.Count-1;index>=0;index--) {
                var i=e.Instructions[index];
                if(!RegistersGrace(i))continue;
                removed.Add((e.ID,i.ArgData.Length>=4?BitConverter.ToUInt32(i.ArgData,0):0,i.ArgData.Length>=8?BitConverter.ToUInt32(i.ArgData,4):0));
                e.Instructions.RemoveAt(index);
                // 事件参数按指令序号替换：去掉指向这条的，后面的序号前移
                e.Parameters.RemoveAll(p=>p.InstructionIndex==index);
                foreach(var p in e.Parameters.Where(p=>p.InstructionIndex>index))p.InstructionIndex--;
            }
        }
        if(removed.Count==0)throw new Exception("Stormveil has no RegisterBonfire instruction; grace layout changed, inspect before patching");
        foreach(var (ev,flag,entity) in removed)Console.WriteLine($"Stormveil grace unregistered: event {ev} flag {flag} entity {entity}");
        return removed;
    }
    /// <summary>只改已生成的事件文件（地图上的符文、侧墙补丁不动）：去掉城里的赐福</summary>
    public static void PatchGraces(string input,string output) {
        if(Path.GetFullPath(input)==Path.GetFullPath(output))throw new Exception("Separate staging required");
        const string relative="event/"+Map+".emevd.dcx";
        var events=EMEVD.Read(Path.Combine(input,relative));
        var before=events.Events.Sum(e=>e.Instructions.Count);
        var removed=StripGraces(events);
        var path=Path.Combine(output,relative);Directory.CreateDirectory(Path.GetDirectoryName(path)!);events.Write(path);
        var actual=EMEVD.Read(path);
        if(actual.Events.SelectMany(e=>e.Instructions).Any(RegistersGrace)||actual.Events.Sum(e=>e.Instructions.Count)!=before-removed.Count
            ||actual.Events.Any(e=>e.Parameters.Any(p=>p.InstructionIndex<0||p.InstructionIndex>=e.Instructions.Count)))throw new Exception("Stormveil grace patch roundtrip failed");
        foreach(uint id in new uint[]{10007980,10007981})PersonalEvents.Verify(actual.Events.Single(e=>e.ID==id));
        Console.WriteLine($"Verified Stormveil: {removed.Count} graces unregistered; other events unchanged");
    }
    public static void Verify(string output) {
        var map=MSBE.Read(Path.Combine(output,$"map/mapstudio/{Map}.msb.dcx"));
        var original=MSBE.Read($"work/boss-report/maps/{Map}.msb.dcx");
        var events=EMEVD.Read(Path.Combine(output,$"event/{Map}.emevd.dcx"));
        var entry=map.Parts.Players.Single(p=>p.EntityID==10000988);
        if(entry.Position!=original.Parts.Players.Single(p=>p.EntityID==10000988).Position||map.Parts.Players.Any(p=>p.Position!=entry.Position))throw new Exception("Stormveil main gate arrival mismatch");
        if(map.Parts.Enemies.Count!=original.Parts.Enemies.Count||map.Parts.Enemies.Zip(original.Parts.Enemies).Any(p=>p.First.Position!=p.Second.Position||p.First.NPCParamID!=p.Second.NPCParamID||p.First.ThinkParamID!=p.Second.ThinkParamID))throw new Exception("Stormveil original encounters changed");
        if(events.Events.Any(e=>e.ID>=10002800&&e.ID<10002900))throw new Exception("Stormveil boss scripts must be absent");
        if(events.Events.SelectMany(e=>e.Instructions).Any(RegistersGrace))throw new Exception("Stormveil graces must stay unregistered");
        foreach(uint id in new uint[]{10007980,10007981})PersonalEvents.Verify(events.Events.Single(e=>e.ID==id));
        var native=EMEVD.Read($"work/scene-cache/{Map}.emevd.dcx");
        if(events.Format!=native.Format||!events.StringData.SequenceEqual(native.StringData)||!events.LinkedFileOffsets.SequenceEqual(native.LinkedFileOffsets))throw new Exception("Stormveil native event dependencies changed");
        Console.WriteLine($"Verified Stormveil main gate {entry.Position}, {map.Parts.Enemies.Count} original actors, open gate and disabled boss scripts");
    }
}
