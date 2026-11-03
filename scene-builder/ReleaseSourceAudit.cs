using SoulsFormats;
using System.Numerics;

static class ReleaseSourceAudit {
    public static void Verify(string input) {
        int melina=0,merchants=0;
        foreach(var path in Directory.GetFiles(Path.Combine(input,"map/mapstudio"),"*.msb.dcx")) {
            var map=MSBE.Read(path);
            foreach(var npc in map.Parts.Enemies.Where(n=>n.EntityID>=45000000&&n.EntityID<45040000)) {
                if(npc.ModelName=="c2180") {
                    if(npc.NPCParamID!=21800000||npc.Scale!=Vector3.One)throw new Exception($"Melina mismatch: {Path.GetFileName(path)} / {npc.Name}");
                    melina++;
                }
                if(npc.NPCParamID==523200079)merchants++;
            }
        }
        if(melina!=24||merchants!=2)throw new Exception($"v103 service actor count changed: Melina={melina}, direct merchants={merchants}; multiplayer template slots are checked by elite-pool-verify");
        var slots=File.ReadAllLines(Path.Combine(input,"party-extra-slots.tsv")).Where(l=>l.Split('\t','|').Contains("4730")).ToArray();
        if(slots.Length!=2)throw new Exception("Expected two Radahn multiplayer slots");
        Console.WriteLine($"Verified {melina} normal-scale Melina actors using NPC 21800000, {merchants} merchant actors and 2 Radahn multiplayer slots; interaction gameplay not simulated");
    }
}
