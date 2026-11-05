using SoulsFormats;
using System.Numerics;

static class DragonlordArrival {
    const long Event=45027952;
    static readonly uint[] Entries=[45022600,45022951,45022952];
    static Vector3 Lower(Vector3 p) {
        if(MathF.Abs(p.Y-1009.726f)<.002f)return p;
        if(MathF.Abs(p.Y-1010.026f)>.002f)throw new Exception("Unexpected Dragonlord spawn height");
        return new(p.X,1009.726f,p.Z);
    }
    static void LowerEntries(MSBE map) {
        var regions=Entries.Select(id=>map.Regions.GetEntries().Single(r=>r.EntityID==id)).ToArray();
        var at=regions[1].Position;
        if(regions.Any(r=>Vector3.Distance(r.Position,at)>.01f)||map.Parts.Players.Any(p=>Vector3.Distance(p.Position,at)>.01f))throw new Exception("Dragonlord spawn positions differ");
        var target=Lower(at);
        foreach(var r in regions)r.Position=target;
        foreach(var p in map.Parts.Players)p.Position=target;
    }
    static EMEVD.Event Original() {
        var e=new EMEVD.Event(Event,EMEVD.Event.RestBehaviorType.Restart);
        PersonalEvents.Local(e);
        e.Instructions.Add(new(3,0,new object[]{(sbyte)0,(byte)1,(byte)0,45007990u}));
        e.Instructions.Add(new(3,0,new object[]{(sbyte)0,(byte)1,(byte)0,45007992u}));
        e.Instructions.Add(new(4,7,new object[]{(sbyte)0,10000u,(byte)1,(sbyte)0,1f}));
        e.Instructions.Add(new(2004,42,new object[]{10000u,(byte)1,45022951u,-1,45020100u}));
        e.Instructions.Add(new(3,2,new object[]{(sbyte)0,(byte)1,10000u,45022952u,1}));
        e.Instructions.Add(new(2003,66,new object[]{(byte)0,45007993u,(byte)1}));
        e.Instructions.Add(new(3,0,new object[]{(sbyte)0,(byte)0,(byte)0,45007992u}));
        e.Instructions.Add(new(1000,4,new object[]{(byte)1}));
        return e;
    }
    static byte[] Bytes(EMEVD.Event e)=>new EMEVD{Events=[e]}.Write();
    public static void Apply(EMEVD.Event e) {
        var expected=Original();var original=Bytes(expected);expected.Instructions.RemoveAt(4);
        if(Bytes(e).SequenceEqual(Bytes(expected)))return;
        if(!Bytes(e).SequenceEqual(original))throw new Exception("Unexpected Dragonlord arrival event; refusing to modify it");
        e.Instructions.RemoveAt(4);
    }
    public static void Test() {
        var e=Original();Apply(e);
        if(e.Instructions.Any(i=>i.Bank==2004&&i.ID==42))throw new Exception("Arrival must not teleport the player a second time");
        var expected=Original();expected.Instructions.RemoveAt(4);
        if(!Bytes(e).SequenceEqual(Bytes(expected)))throw new Exception("Arrival readiness checks changed");
        var once=Bytes(e);Apply(e);
        if(!once.SequenceEqual(Bytes(e)))throw new Exception("Arrival patch is not idempotent");
        var damaged=Original();damaged.Instructions[2].ArgData[1]=0;
        bool rejected=false;try{Apply(damaged);}catch(Exception){rejected=true;}
        if(!rejected)throw new Exception("Unexpected arrival script accepted");
        var start=new Vector3(21.856926f,1010.026f,300.43274f);var lower=Lower(start);
        if(lower.X!=start.X||lower.Z!=start.Z||MathF.Abs(start.Y-lower.Y-.3f)>.001f||Lower(lower)!=lower)throw new Exception("Spawn height correction or idempotence failed");
        rejected=false;try{Lower(Vector3.Zero);}catch(Exception){rejected=true;}
        if(!rejected)throw new Exception("Unknown spawn height accepted");
        Console.WriteLine("Verified duplicate warp removal, readiness checks, idempotence and unexpected-script rejection");
        Console.WriteLine("Verified 0.30m spawn correction, horizontal position preservation and unknown-height rejection");
    }
    public static void Patch(string input,string output) {
        if(Path.GetFullPath(input)==Path.GetFullPath(output))throw new Exception("Separate staging required");
        const string name="m13_00_00_00";
        var map=MSBE.Read(Path.Combine(input,"map/mapstudio",name+".msb.dcx"));
        var entry=map.Regions.GetEntries().Single(r=>r.EntityID==45022951);
        var check=map.Regions.GetEntries().Single(r=>r.EntityID==45022952);
        if(Vector3.Distance(entry.Position,check.Position)>.01f||check.Shape is not MSB.Shape.Sphere{Radius:2})throw new Exception("Dragonlord spawn and arrival check no longer coincide");
        var mapBefore=map.Write();
        var regionsBefore=Entries.ToDictionary(id=>id,id=>map.Regions.GetEntries().Single(r=>r.EntityID==id).Position);
        var playersBefore=map.Parts.Players.ToDictionary(p=>p.Name,p=>p.Position);
        LowerEntries(map);
        var mapTarget=Path.Combine(output,"map/mapstudio",name+".msb.dcx");Directory.CreateDirectory(Path.GetDirectoryName(mapTarget)!);map.Write(mapTarget);
        var mapCheck=MSBE.Read(mapTarget);var mapOnce=mapCheck.Write();LowerEntries(mapCheck);
        if(!mapOnce.SequenceEqual(mapCheck.Write()))throw new Exception("Spawn patch is not idempotent after roundtrip");
        foreach(var r in mapCheck.Regions.GetEntries().Where(r=>Entries.Contains(r.EntityID)))r.Position=regionsBefore[r.EntityID];
        foreach(var p in mapCheck.Parts.Players)p.Position=playersBefore[p.Name];
        if(!mapBefore.SequenceEqual(mapCheck.Write()))throw new Exception("Unrelated map content changed");
        var file=EMEVD.Read(Path.Combine(input,"event",name+".emevd.dcx"));
        var before=file.Write();var e=file.Events.Single(e=>e.ID==Event);var original=Bytes(e);
        Apply(e);
        var target=Path.Combine(output,"event",name+".emevd.dcx");Directory.CreateDirectory(Path.GetDirectoryName(target)!);file.Write(target);
        var roundtrip=EMEVD.Read(target);var patched=roundtrip.Events.Single(e=>e.ID==Event);
        if(!Bytes(patched).SequenceEqual(Bytes(e)))throw new Exception("Arrival roundtrip mismatch");
        roundtrip.Events[roundtrip.Events.IndexOf(patched)]=EMEVD.Read(original).Events.Single();
        if(!before.SequenceEqual(roundtrip.Write()))throw new Exception("Unrelated events changed");
        MusicAssets.RefreshArenaManifest(input,output);
        Console.WriteLine("Dragonlord arrival patched; all other events including rescue retained; arena manifest refreshed");
    }
}
