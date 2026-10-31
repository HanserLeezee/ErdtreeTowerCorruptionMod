using SoulsFormats;

static class DormantLoadingAssets {
    static bool Hidden(EMEVD.Instruction i)=>i.Bank==2004&&i.ID==5&&i.ArgData.Length==8&&i.ArgData[4]==0;
    static bool Unloaded(EMEVD.Instruction i,uint id)=>i.Bank==2004&&i.ID==29&&i.ArgData.Length==8&&BitConverter.ToUInt32(i.ArgData)==id&&i.ArgData[4]==1;
    public static int Apply(MSBE map,EMEVD events) {
        var enemies=map.Parts.Enemies.Select(e=>e.EntityID).Where(id=>id!=0&&id!=uint.MaxValue).ToHashSet();
        var init=events.Events.Single(e=>e.ID==0);int count=0;
        if(init.Parameters.Count!=0)throw new Exception("Parameterized constructor requires explicit index remapping");
        for(int n=0;n<init.Instructions.Count;n++) {
            var ins=init.Instructions[n];if(!Hidden(ins))continue;
            var id=BitConverter.ToUInt32(ins.ArgData);if(!enemies.Contains(id))continue;
            if(n+1<init.Instructions.Count&&Unloaded(init.Instructions[n+1],id))continue;
            init.Instructions.Insert(++n,new(2004,29,new object[]{id,(byte)1}));count++;
        }
        Verify(map,events);return count;
    }
    static void Verify(MSBE map,EMEVD events) {
        var enemies=map.Parts.Enemies.Select(e=>e.EntityID).ToHashSet();var init=events.Events.Single(e=>e.ID==0).Instructions;
        for(int n=0;n<init.Count;n++)if(Hidden(init[n])&&enemies.Contains(BitConverter.ToUInt32(init[n].ArgData)))
            if(n+1==init.Count||!Unloaded(init[n+1],BitConverter.ToUInt32(init[n].ArgData)))throw new Exception("Hidden actor must release backread immediately");
    }
    public static void Test() {
        var map=new MSBE();map.Parts.Enemies.Add(new(){EntityID=48010000});
        var events=new EMEVD();var init=new EMEVD.Event(0);events.Events.Add(init);
        init.Instructions.Add(new(2004,5,new object[]{48010000u,(byte)0}));
        init.Instructions.Add(new(2000,0,new object[]{0,48010000u,0u}));
        if(Apply(map,events)!=1||Apply(map,events)!=0||init.Instructions.Count!=3)throw new Exception("Idempotence failed");
        if(!Unloaded(init.Instructions[1],48010000)||init.Instructions[2].Bank!=2000)throw new Exception("Unload must precede actor event initialization");
        Verify(map,EMEVD.Read(events.Write()));Console.WriteLine("Dormant startup order, idempotence and binary roundtrip passed");
    }
    public static void Patch(string input,string output) {
        if(Path.GetFullPath(input)==Path.GetFullPath(output))throw new Exception("Separate staging required");
        var maps=File.ReadLines(Path.Combine(input,"arenas-built.txt")).Where(l=>!string.IsNullOrWhiteSpace(l)&&!l.StartsWith('#')).Select(l=>l.Split('|')[1]).Distinct();
        int total=0;
        foreach(var name in maps) {
            var map=MSBE.Read(Path.Combine(input,"map/mapstudio",name+".msb.dcx"));
            var before=EMEVD.Read(Path.Combine(input,"event",name+".emevd.dcx"));
            var events=EMEVD.Read(before.Write());var count=Apply(map,events);
            var path=Path.Combine(output,"event",name+".emevd.dcx");Directory.CreateDirectory(Path.GetDirectoryName(path)!);events.Write(path);
            var after=EMEVD.Read(path);Verify(map,after);
            if(after.Events.Count!=before.Events.Count)throw new Exception("Event count changed");
            for(int e=0;e<before.Events.Count;e++) {
                var old=before.Events[e];var actual=after.Events[e];
                if(old.ID!=actual.ID||old.RestBehavior!=actual.RestBehavior||old.Parameters.Count!=actual.Parameters.Count)throw new Exception("Event metadata changed");
                int at=0;
                foreach(var ins in old.Instructions) {
                    var next=actual.Instructions[at++];
                    if(ins.Bank!=next.Bank||ins.ID!=next.ID||!ins.ArgData.SequenceEqual(next.ArgData))throw new Exception("Original event changed");
                    if(old.ID==0&&Hidden(ins)&&at<actual.Instructions.Count&&Unloaded(actual.Instructions[at],BitConverter.ToUInt32(ins.ArgData))
                        &&(old.Instructions.IndexOf(ins)+1==old.Instructions.Count||!Unloaded(old.Instructions[old.Instructions.IndexOf(ins)+1],BitConverter.ToUInt32(ins.ArgData))))at++;
                }
                if(at!=actual.Instructions.Count)throw new Exception("Unexpected added instructions");
            }
            total+=count;Console.WriteLine($"{name}: {count} early unload instructions; original events preserved");
        }
        var manifest=File.ReadAllLines(Path.Combine(input,"arenas-resources.txt"));
        for(int i=0;i<manifest.Length;i++) {
            var p=manifest[i].Split('|');if(p.Length!=3)continue;
            var path=Path.Combine(output,p[0]);if(!File.Exists(path))continue;
            var bytes=File.ReadAllBytes(path);ulong hash=14695981039346656037;foreach(byte b in bytes)hash=unchecked((hash^b)*1099511628211);
            manifest[i]=$"{p[0]}|{bytes.Length}|{hash:x16}";
        }
        File.WriteAllLines(Path.Combine(output,"arenas-resources.txt"),manifest);
        Console.WriteLine($"Verified {total} early unload additions; no actor, model, AI or spawn-slot changes");
    }
}
