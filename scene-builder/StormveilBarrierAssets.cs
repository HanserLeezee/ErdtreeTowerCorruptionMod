using SoulsFormats;
using System.Numerics;

static class StormveilBarrierAssets {
    const string Map="map/mapstudio/m10_00_00_00.msb.dcx";
    const string Prefix="tower_parkour_sidewall_";
    public static void Patch(string input,string output) {
        if(Path.GetFullPath(input)==Path.GetFullPath(output))throw new Exception("Separate staging required");
        var map=MSBE.Read(Path.Combine(input,Map));
        var donor=map.Parts.Assets.First(a=>a.ModelName=="AEG210_092");
        var doorway=map.Parts.Assets.Single(a=>a.Name=="AEG210_171_2000");
        var baseline=MSBE.Read(Path.Combine(input,Map));
        map.Parts.Assets.RemoveAll(a=>a.Name.StartsWith(Prefix));
        baseline.Parts.Assets.RemoveAll(a=>a.Name.StartsWith(Prefix));
        int instance=map.Parts.Assets.Where(a=>a.ModelName==donor.ModelName).Max(a=>a.InstanceID)+1;
        // Visual masonry; runtime parkour_barrier supplies the player boundary (map masonry collision is baked).
        for(int i=0;i<6;i++) {
            uint entity=10006980u+(uint)i;
            if(map.Parts.GetEntries().Any(p=>p.EntityID==entity))throw new Exception("Sidewall entity collision");
            var wall=(MSBE.Part.Asset)donor.DeepCopy();
            wall.Name=Prefix+i;wall.EntityID=entity;wall.InstanceID=instance+i;
            wall.Position=new(-113.677f,20.8f+i*2f,17.145f);wall.Rotation=new(0,-145,0);wall.Scale=Vector3.One;
            wall.EntityGroupIDs=new uint[8];wall.UnkPartNames=doorway.UnkPartNames.ToArray();wall.UnkT54PartName=doorway.UnkT54PartName;
            wall.MapStudioLayer=uint.MaxValue;
            wall.Unk1.DisplayGroups=Enumerable.Repeat(uint.MaxValue,8).ToArray();wall.Unk1.DrawGroups=Enumerable.Repeat(uint.MaxValue,8).ToArray();
            map.Parts.Assets.Add(wall);
        }
        var path=Path.Combine(output,Map);Directory.CreateDirectory(Path.GetDirectoryName(path)!);map.Write(path);
        var actual=MSBE.Read(path);var walls=actual.Parts.Assets.Where(a=>a.Name.StartsWith(Prefix)).ToArray();
        if(walls.Length!=6||walls.Any(a=>a.ModelName!=donor.ModelName||a.Scale!=Vector3.One))throw new Exception("Sidewall roundtrip failed");
        foreach(var wall in walls)foreach(var name in wall.UnkPartNames.Append(wall.UnkT54PartName).Where(n=>n!=null))
            if(!actual.Parts.GetEntries().Any(p=>p.Name==name))throw new Exception("Sidewall loading reference missing");
        actual.Parts.Assets.RemoveAll(a=>a.Name.StartsWith(Prefix));
        if(!actual.Write().SequenceEqual(baseline.Write()))throw new Exception("Unrelated map contents changed");
        Console.WriteLine("Verified six side-door masonry panels; main gate, spawn, NPC rune isolation and existing map parts preserved");
    }
}
