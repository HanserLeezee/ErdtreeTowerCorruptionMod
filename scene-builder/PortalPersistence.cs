using SoulsFormats;

static class PortalPersistence {
    public static void Apply(EMEVD.Event visual) {
        PersonalEvents.Verify(visual);
        var end=visual.Instructions.Last();
        if(end.Bank!=1000||end.ID!=4||end.ArgData.Length!=4||end.ArgData[0]>1)
            throw new Exception("Unexpected portal visual terminator");
        // End this visual instance without running its initial hide commands again.
        end.ArgData[0]=0;
    }
    public static void Patch(string input,string output) {
        if(Path.GetFullPath(input)==Path.GetFullPath(output))throw new Exception("Separate staging required");
        int count=0;
        foreach(var path in Directory.EnumerateFiles(Path.Combine(input,"event"),"*.emevd.dcx")) {
            var events=EMEVD.Read(path);
            var visuals=events.Events.Where(e=>e.ID is 45000986 or 45010986 or 45020986).ToArray();
            if(visuals.Length==0)continue;
            var original=events.Write();
            var endings=visuals.ToDictionary(e=>e.ID,e=>e.Instructions.Last().ArgData[0]);
            foreach(var visual in visuals) {
                if(visual.Instructions.Any(i=>i.Bank==2003&&i.ID==66||i.Bank==3&&i.ID==24))throw new Exception("Portal visual unexpectedly contains interaction logic");
                Apply(visual);
            }
            var target=Path.Combine(output,"event",Path.GetFileName(path));Directory.CreateDirectory(Path.GetDirectoryName(target)!);events.Write(target);
            var check=EMEVD.Read(target);var once=check.Write();
            foreach(var id in endings.Keys)Apply(check.Events.Single(e=>e.ID==id));
            if(!once.SequenceEqual(check.Write()))throw new Exception("Portal persistence patch is not idempotent");
            foreach(var (id,value) in endings)check.Events.Single(e=>e.ID==id).Instructions.Last().ArgData[0]=value;
            if(!original.SequenceEqual(check.Write()))throw new Exception("Portal patch changed interaction, rewards or unrelated events");
            Console.WriteLine($"{Path.GetFileName(path)}: visual persistence verified; interaction and reward events unchanged");count+=visuals.Length;
        }
        if(count==0)throw new Exception("No portal visual events found");
        MusicAssets.RefreshArenaManifest(input,output);
        Console.WriteLine($"Verified {count} persistent portal visuals");
    }
}
