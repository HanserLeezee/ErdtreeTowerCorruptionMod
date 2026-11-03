using SoulsFormats;
using System.Numerics;

static class MelinaEntranceAssets {
    public static Vector3 Position(MSBE.Region entry) {
        float angle=entry.Rotation.Y*MathF.PI/180;
        return entry.Position+new Vector3(3*MathF.Cos(angle)-MathF.Sin(angle),0,-3*MathF.Sin(angle)-MathF.Cos(angle));
    }
    public static void Patch(string input,string output) {
        int count=0;
        foreach(var file in Directory.GetFiles(Path.Combine(input,"map/mapstudio"),"*.msb.dcx")) {
            var map=MSBE.Read(file);
            var services=map.Parts.Enemies.Where(e=>e.ModelName=="c2180"&&e.EntityID>=45000000&&e.EntityID<45040000).ToArray();
            if(services.Length==0)continue;
            foreach(var npc in services) {
                npc.NPCParamID=21800000;npc.Scale=Vector3.One;
                if(npc.EntityID%10000==1810) {
                    var entry=map.Regions.GetEntries().Single(e=>e.EntityID==npc.EntityID-1810+2600);
                    npc.Position=Position(entry);npc.Rotation=entry.Rotation;
                }
            }
            var destination=Path.Combine(output,"map/mapstudio",Path.GetFileName(file));
            Directory.CreateDirectory(Path.GetDirectoryName(destination)!);map.Write(destination);
            var check=MSBE.Read(destination);
            foreach(var npc in services) {
                var actual=check.Parts.Enemies.Single(e=>e.EntityID==npc.EntityID);
                if(actual.NPCParamID!=21800000||actual.Scale!=Vector3.One||actual.Position!=npc.Position)throw new Exception("Melina entrance roundtrip mismatch");
            }
            count+=services.Length;
        }
        if(count==0)throw new Exception("No Melina services found");
        MusicAssets.RefreshArenaManifest(input,output);
        Console.WriteLine($"Verified {count} normal Melina services; arena attendants moved beside entry regions");
    }
}
