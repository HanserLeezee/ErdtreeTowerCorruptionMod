using SoulsFormats;
using System.Numerics;

static class PortalArchTests {
    public static void Test() {
        const uint portal=45021900;
        var map=new MSBE();
        map.Models.Assets.Add(new MSBE.Model.Asset{Name="AEG099_510"});
        map.Parts.Assets.Add(new MSBE.Part.Asset{Name="portal",ModelName="AEG099_510",EntityID=portal,Position=new(3,2,7),Rotation=new(0,43,0)});
        PortalReveal.AddRegion(map,portal);
        var copy=MSBE.Read(map.Write());
        var carrier=copy.Parts.Assets.Single(a=>a.EntityID==portal);
        if(carrier.ModelName!="AEG099_001")throw new Exception("Expected native fog carrier at the original interaction entity");
        var arch=copy.Parts.Assets.Single(a=>a.EntityID==portal+2100);
        var ring=copy.Parts.Assets.Single(a=>a.EntityID==portal+2101);
        var rune=copy.Parts.Assets.Single(a=>a.EntityID==portal+2102);
        if(rune.Position!=carrier.Position+new Vector3(0,1.15f,0)||rune.Scale!=new Vector3(1.3f)||rune.ModelName!="AEG099_991")throw new Exception("Rune must be enlarged and lowered into the doorway");
        if(arch.ModelName!="AEG990_446"||arch.Scale!=new Vector3(.5f)||arch.Position!=carrier.Position||arch.Rotation!=carrier.Rotation)throw new Exception("Portal must use the isolated decorative arch");
        if(ring.Rotation.X!=90||ring.Rotation.Y!=carrier.Rotation.Y||ring.ModelName!="AEG099_001")throw new Exception("Inner flow must be upright in the doorway plane");
        if(copy.Regions.SFX.Any(r=>r.EntityID==PortalReveal.RingRegion(portal)))throw new Exception("Obsolete upright ground-ring effect remains");
        foreach(int step in Enumerable.Range(0,PortalReveal.Steps)) {
            var frame=copy.Parts.Assets.Single(a=>a.EntityID==PortalReveal.StepEntity(portal,step));
            if(frame.ModelName!=arch.ModelName||frame.Scale!=arch.Scale)throw new Exception("Rise frame does not match the arch");
        }
        if(copy.Parts.Assets.Any(a=>a.ModelName=="AEG099_510"))throw new Exception("Old small portal still visible");
        PortalReveal.Test();
        Console.WriteLine("Ornate arch model, transforms, roundtrip and lifecycle passed");
    }
}
