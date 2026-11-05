using SoulsFormats;
using System.Numerics;

static class PortalReveal {
    public const string ArchModel=PortalVisualAssets.Model,CarrierModel="AEG099_001",RuneModel="AEG099_991";
    public const int Steps=24,EffectDummy=100,RuneEffect=806932;
    public const int FlowDummy=101,FlowEffect=AuraFfxAssets.Gold;
    public const float RiseDelay=.35f,Depth=6.5f,StepSeconds=.04f;
    public static readonly Vector3 DoorScale=new(.5f),BorderScale=new(.4f,.4f,.65f),RuneScale=new(1.3f);
    public const float EffectHeight=1.15f;
    static Vector3 FlowPosition(MSBE.Part.Asset portal)=>portal.Position+Vector3.Transform(new Vector3(0,2.65f,-.12f),Matrix4x4.CreateRotationY(portal.Rotation.Y*MathF.PI/180));
    public static uint StepEntity(uint portal,int step)=>portal+2000+(uint)step;
    public static uint ArchEntity(uint portal)=>portal+2100;
    public static uint BorderEntity(uint portal)=>portal+2101;
    public static uint RuneEntity(uint portal)=>portal+2102;
    public static uint RiseRegion(uint portal)=>portal+1062;
    public static uint RingRegion(uint portal)=>portal+1063;
    static bool Is(EMEVD.Instruction i,int bank,int id,uint entity)=>i.Bank==bank&&i.ID==id&&i.ArgData.Length>=4&&BitConverter.ToUInt32(i.ArgData)==entity;
    static bool Frame(uint id,uint portal)=>id>=StepEntity(portal,0)&&id<StepEntity(portal,Steps);
    public static bool VisualAsset(uint id,uint portal)=>id==portal||id==ArchEntity(portal)||id==BorderEntity(portal)||id==RuneEntity(portal)||Frame(id,portal);
    static bool Region(uint id,uint portal)=>id==RiseRegion(portal)||id==RingRegion(portal);
    static bool Glow(EMEVD.Instruction i,uint portal)=>Is(i,2006,4,portal)||Is(i,2006,4,BorderEntity(portal));
    static bool Anchor(EMEVD.Instruction i,uint portal)=>Glow(i,portal)||i.Bank==1001&&i.ID==0&&i.ArgData.Length==4&&BitConverter.ToSingle(i.ArgData)==0f;
    public static EMEVD.Event Visual(EMEVD events,uint portal)=>events.Events.Single(e=>e.ID==portal-914);

    public static void ClearMap(MSBE map,uint portal) {
        foreach(var part in map.Parts.Assets.Where(a=>a.EntityID!=portal&&VisualAsset(a.EntityID,portal)).ToArray()) {
            if(!part.Name.StartsWith("tower_portal_"))throw new Exception("Portal asset ID belongs to another feature");
            map.Parts.Assets.Remove(part);
        }
        foreach(var region in map.Regions.SFX.Where(r=>Region(r.EntityID,portal)).ToArray()) {
            if(!region.Name.StartsWith("tower_portal_"))throw new Exception("Portal SFX ID belongs to another feature");
            map.Regions.SFX.Remove(region);
        }
    }

    public static void AddRegion(MSBE map,uint portal) {
        var part=map.Parts.Assets.Single(a=>a.EntityID==portal);
        void Reserve(uint id) {
            if(map.Parts.GetEntries().Any(p=>p.EntityID==id)||map.Regions.GetEntries().Any(r=>r.EntityID==id))throw new Exception($"Portal entity {id} already used");
        }
        foreach(string name in new[]{ArchModel,CarrierModel,RuneModel})if(!map.Models.Assets.Any(m=>m.Name==name))
            map.Models.Assets.Add(new MSBE.Model.Asset{Name=name,SibPath=$@"N:\GR\data\Asset\Environment\geometry\{name[..6]}\{name}\sib\{name}.sib"});
        part.ModelName=CarrierModel;part.Scale=Vector3.One;part.UnkModelMaskAndAnimID=-1;part.AssetSfxParamRelativeID=-1;
        part.InstanceID=map.Parts.Assets.Where(a=>a!=part&&a.ModelName==CarrierModel).Select(a=>a.InstanceID).DefaultIfEmpty(-1).Max()+1;
        foreach(var (id,name,scale) in new[]{(BorderEntity(portal),"tower_portal_border",BorderScale),(RuneEntity(portal),"tower_portal_rune",RuneScale)}) {
            Reserve(id);var carrier=(MSBE.Part.Asset)part.DeepCopy();carrier.EntityID=id;carrier.Name=name;carrier.Scale=scale;
            carrier.Position=part.Position+new Vector3(0,EffectHeight,0);carrier.InstanceID=part.InstanceID+(id==BorderEntity(portal)?1:2);
            if(id==BorderEntity(portal)) {carrier.Position=FlowPosition(part);carrier.Rotation=part.Rotation+new Vector3(90,0,0);}
            if(id==RuneEntity(portal)) {
                carrier.ModelName=RuneModel;
                carrier.InstanceID=map.Parts.Assets.Where(a=>a.ModelName==RuneModel).Select(a=>a.InstanceID).DefaultIfEmpty(-1).Max()+1;
            }
            carrier.EntityGroupIDs=new uint[8];carrier.UnkPartNames=new string[6];carrier.UnkT54PartName=null;map.Parts.Assets.Add(carrier);
        }
        Reserve(RiseRegion(portal));
        map.Regions.SFX.Add(new(){Name="tower_portal_rise",EntityID=RiseRegion(portal),EffectID=AuraFfxAssets.Gold,StartDisabled=1,
            Position=part.Position-new Vector3(0,.12f,0),Shape=new MSB.Shape.Point()});
        int instance=map.Parts.Assets.Where(a=>a.ModelName==ArchModel).Select(a=>a.InstanceID).DefaultIfEmpty(-1).Max()+1;
        MSBE.Part.Asset Arch(uint id,string name,Vector3 position) {
            Reserve(id);var arch=(MSBE.Part.Asset)part.DeepCopy();arch.Name=name;arch.EntityID=id;arch.ModelName=ArchModel;arch.Scale=DoorScale;
            arch.Position=position;arch.InstanceID=instance++;arch.EntityGroupIDs=new uint[8];arch.UnkPartNames=new string[6];arch.UnkT54PartName=null;
            return arch;
        }
        map.Parts.Assets.Add(Arch(ArchEntity(portal),"tower_portal_arch",part.Position));
        for(int step=0;step<Steps;step++)map.Parts.Assets.Add(Arch(StepEntity(portal,step),$"tower_portal_frame_{step}",part.Position-new Vector3(0,Depth*(Steps-step)/Steps,0)));
    }

    // Strip generated visual commands only; preserve victory gates and interaction events.
    static void ClearReveal(EMEVD events,uint portal) {
        var list=Visual(events,portal).Instructions;
        int start=list.FindIndex(i=>Is(i,2006,2,RiseRegion(portal))),glow=list.FindIndex(i=>Anchor(i,portal));
        if(start>=0) {
            if(start>=glow||list.Skip(start).Take(glow-start).Any(i=>!(i.Bank==1001&&i.ID==0||i.Bank==2005&&i.ID==3&&VisualAsset(BitConverter.ToUInt32(i.ArgData),portal)||i.Bank==2006&&i.ID==2&&Region(BitConverter.ToUInt32(i.ArgData),portal))))
                throw new Exception("Unexpected command inside portal reveal; refusing to erase it");
            list.RemoveRange(start,glow-start);
        }
        bool Generated(EMEVD.Instruction i)=>i.ArgData.Length>=4&&(i.Bank==2005&&i.ID==3&&VisualAsset(BitConverter.ToUInt32(i.ArgData),portal)||i.Bank==2006&&i.ID==1&&Region(BitConverter.ToUInt32(i.ArgData),portal)||Is(i,2006,4,RuneEntity(portal))||Is(i,2006,4,BorderEntity(portal))&&i.ArgData.Length==12&&BitConverter.ToInt32(i.ArgData,8)==FlowEffect||Is(i,2006,5,RuneEntity(portal))||Is(i,2006,5,BorderEntity(portal)));
        list.RemoveAll(Generated);events.Events.Single(e=>e.ID==0).Instructions.RemoveAll(Generated);
    }

    public static void Apply(EMEVD events,uint portal) {
        ClearReveal(events,portal);
        var visual=Visual(events,portal);var list=visual.Instructions;
        int at=list.FindIndex(i=>Anchor(i,portal));
        if(at<0)throw new Exception("Portal reveal anchor missing");
        // Keep the migration anchor; the intact native effect uses its original carrier and dummy.
        list[at]=new(1001,0,new object[]{0f});
        list.Insert(at+1,new(2006,4,new object[]{RuneEntity(portal),EffectDummy,RuneEffect}));
        var sequence=new List<EMEVD.Instruction>{new(2006,2,new object[]{RiseRegion(portal)}),new(1001,0,new object[]{RiseDelay})};
        for(int step=0;step<Steps;step++) {
            sequence.Add(new(2005,3,new object[]{StepEntity(portal,step),(byte)1}));
            sequence.Add(new(1001,0,new object[]{StepSeconds}));
            sequence.Add(new(2005,3,new object[]{StepEntity(portal,step),(byte)0}));
        }
        sequence.Add(new(2005,3,new object[]{ArchEntity(portal),(byte)1}));
        sequence.Add(new(2005,3,new object[]{BorderEntity(portal),(byte)1}));
        sequence.Add(new(2005,3,new object[]{RuneEntity(portal),(byte)1}));
        sequence.Add(new(2005,3,new object[]{portal,(byte)1}));list.InsertRange(at,sequence);
        var reset=new List<EMEVD.Instruction>{new(2005,3,new object[]{portal,(byte)0}),new(2005,3,new object[]{ArchEntity(portal),(byte)0}),new(2006,1,new object[]{RiseRegion(portal),(byte)0})};
        foreach(uint entity in new[]{BorderEntity(portal),RuneEntity(portal)}) {
            reset.Add(new(2006,5,new object[]{entity,1}));reset.Add(new(2005,3,new object[]{entity,(byte)0}));
        }
        for(int step=0;step<Steps;step++)reset.Add(new(2005,3,new object[]{StepEntity(portal,step),(byte)0}));
        list.InsertRange(list.FirstOrDefault() is {Bank:2000,ID:2}?1:0,reset);
        events.Events.Single(e=>e.ID==0).Instructions.InsertRange(0,reset.Where(i=>i.Bank==2005));
        PortalPersistence.Apply(visual);
    }

    public static void Verify(MSBE map,EMEVD events,uint portal,string key) {
        var part=map.Parts.Assets.Single(a=>a.EntityID==portal);var arch=map.Parts.Assets.Single(a=>a.EntityID==ArchEntity(portal));
        if(part.ModelName!=CarrierModel||part.Scale!=Vector3.One||arch.ModelName!=ArchModel||arch.Scale!=DoorScale||part.Position!=arch.Position||part.Rotation!=arch.Rotation)throw new Exception(key+": portal frame/carrier mismatch");
        foreach(var name in new[]{ArchModel,CarrierModel,RuneModel})if(!map.Models.Assets.Any(m=>m.Name==name))throw new Exception("Portal model reference missing");
        foreach(var (id,scale) in new[]{(BorderEntity(portal),BorderScale),(RuneEntity(portal),RuneScale)}) {
            var carrier=map.Parts.Assets.Single(a=>a.EntityID==id);
            var position=id==RuneEntity(portal)?part.Position+new Vector3(0,EffectHeight,0):FlowPosition(part);
            var rotation=id==RuneEntity(portal)?part.Rotation:part.Rotation+new Vector3(90,0,0);
            if(carrier.ModelName!=(id==RuneEntity(portal)?RuneModel:CarrierModel)||carrier.Scale!=scale||carrier.Position!=position||carrier.Rotation!=rotation)throw new Exception("Rune/flow carrier mismatch");
        }
        var ground=map.Regions.SFX.Single(r=>r.EntityID==RiseRegion(portal));
        if(ground.EffectID!=AuraFfxAssets.Gold||ground.StartDisabled!=1||Vector3.Distance(ground.Position,part.Position-new Vector3(0,.12f,0))>.001f)throw new Exception("Portal ground light mismatch");
        if(map.Regions.SFX.Any(r=>r.EntityID==RingRegion(portal)))throw new Exception("Obsolete upright ground ring remains");
        var visual=Visual(events,portal);PersonalEvents.Verify(visual);var list=visual.Instructions;
        if(list.Last() is not {Bank:1000,ID:4}||list.Last().ArgData[0]!=0)throw new Exception("Portal visual must remain after interaction until map reload");
        int glow=list.FindIndex(i=>Anchor(i,portal));
        if(glow<4||!Is(list[glow-1],2005,3,portal)||!Is(list[glow-4],2005,3,ArchEntity(portal)))throw new Exception("Rune portal must follow completed arch rise");
        var effects=list.Where(i=>i.Bank==2006&&i.ID==4).ToArray();
        if(effects.Length!=1||!Is(effects[0],2006,4,RuneEntity(portal))||effects[0].ArgData.Length!=12
            ||BitConverter.ToInt32(effects[0].ArgData,4)!=EffectDummy||BitConverter.ToInt32(effects[0].ArgData,8)!=RuneEffect
            ||list.IndexOf(effects[0])!=glow+1)throw new Exception("Portal must use only the intact native rune after the arch rises");
        if(list.Skip(glow).Any(i=>Is(i,2006,5,RuneEntity(portal))||Is(i,2006,5,BorderEntity(portal))))throw new Exception("Portal glow must remain visible after interaction");
        var ids=Enumerable.Range(0,Steps).Select(s=>StepEntity(portal,s)).Concat(new[]{ArchEntity(portal),BorderEntity(portal),RuneEntity(portal),portal}).ToArray();
        foreach(uint id in ids) {
            if(list.Count(i=>Is(i,2005,3,id)&&i.ArgData[4]==1)!=1||list.Count(i=>Is(i,2005,3,id)&&i.ArgData[4]==0)!=(Frame(id,portal)?2:1))throw new Exception("Portal visibility lifecycle mismatch");
            if(!events.Events.Single(e=>e.ID==0).Instructions.Any(i=>Is(i,2005,3,id)&&i.ArgData[4]==0))throw new Exception("Portal visible at map load");
        }
        if(!list.Any(i=>Is(i,2006,5,portal))||!list.Any(i=>Is(i,2006,1,RiseRegion(portal))))throw new Exception("Portal effect cleanup missing");
        for(int step=0;step<Steps;step++) {
            var frame=map.Parts.Assets.Single(a=>a.EntityID==StepEntity(portal,step));
            if(frame.ModelName!=ArchModel||frame.Scale!=DoorScale||frame.Rotation!=arch.Rotation||Vector3.Distance(frame.Position,part.Position-new Vector3(0,Depth*(Steps-step)/Steps,0))>.001f)throw new Exception("Portal rise frame mismatch");
        }
        foreach(var i in list) {
            int expected=(i.Bank,i.ID) switch{(2005,3)=>8,(2006,1)=>8,(2006,2)=>4,(2006,4)=>12,(2006,5)=>8,(1001,0)=>4,_=>-1};
            if(expected>=0&&i.ArgData.Length!=expected)throw new Exception("Portal event instruction layout mismatch");
        }
    }

    public static void Test() {
        const uint portal=45021900;
        var events=new EMEVD();events.Events.Add(new EMEVD.Event(0));
        var visual=new EMEVD.Event(45020986,EMEVD.Event.RestBehaviorType.Restart);PersonalEvents.Local(visual);
        visual.Instructions.Add(new(2006,5,new object[]{portal,1}));
        foreach(uint flag in new uint[]{45020970,45020993})visual.Instructions.Add(new(3,0,new object[]{(sbyte)-1,(byte)1,(byte)0,flag}));
        visual.Instructions.Add(new(0,0,new object[]{(sbyte)0,(byte)1,(sbyte)-1}));
        visual.Instructions.Add(new(2006,4,new object[]{portal,200,806870}));
        foreach(uint flag in new uint[]{45020970,45020993})visual.Instructions.Add(new(3,0,new object[]{(sbyte)0,(byte)0,(byte)0,flag}));
        visual.Instructions.Add(new(1000,4,new object[]{(byte)1}));events.Events.Add(visual);
        string[] Guards()=>visual.Instructions.Where(i=>i.Bank<1001&&!(i.Bank==1000&&i.ID==4)).Select(i=>$"{i.Bank}/{i.ID}/"+Convert.ToHexString(i.ArgData)).ToArray();
        var guards=Guards();
        var map=new MSBE();map.Models.Assets.Add(new MSBE.Model.Asset{Name="AEG099_510"});
        map.Parts.Assets.Add(new MSBE.Part.Asset{Name="portal",ModelName="AEG099_510",EntityID=portal,Position=new(1,2,3)});
        AddRegion(map,portal);Apply(events,portal);Verify(MSBE.Read(map.Write()),EMEVD.Read(events.Write()),portal,"test");
        foreach(var (offset,value) in new[]{(4,101),(8,PortalRuneFfx.Rune),(8,PortalRuneFfx.Border)}) {
            var invalid=EMEVD.Read(events.Write());var effect=Visual(invalid,portal).Instructions.Single(i=>Is(i,2006,4,RuneEntity(portal)));
            BitConverter.GetBytes(value).CopyTo(effect.ArgData,offset);bool rejected=false;
            try {Verify(map,invalid,portal,"unsafe");}catch(Exception){rejected=true;}
            if(!rejected)throw new Exception("Unsafe portal effect or attachment accepted");
        }
        if(visual.Instructions.Any(i=>i.Bank==2006&&i.ID==4&&new[]{PortalRuneFfx.Border,PortalRuneFfx.Rune}.Contains(BitConverter.ToInt32(i.ArgData,8))))throw new Exception("Crash-prone rune portal effects are still callable");
        if(!guards.SequenceEqual(Guards()))throw new Exception("Victory gates changed");
        var end=visual.Instructions.Last();
        if(end.Bank!=1000||end.ID!=4||end.ArgData[0]!=0)throw new Exception("Consumed portal must stay visible until the scene reloads");
        var first=events.Write();Apply(events,portal);if(!first.SequenceEqual(events.Write()))throw new Exception("Reveal generation is not idempotent");
        var enabled=new HashSet<uint>();
        foreach(var i in visual.Instructions.Where(i=>i.Bank==2005&&i.ID==3)) {
            uint id=BitConverter.ToUInt32(i.ArgData);if(!Frame(id,portal)&&id!=ArchEntity(portal))continue;
            if(i.ArgData[4]==1)enabled.Add(id);else enabled.Remove(id);
            if(enabled.Count>1)throw new Exception("More than one arch frame visible");
        }
        if(!enabled.SetEquals(new[]{ArchEntity(portal)}))throw new Exception("Final arch missing");
        foreach(uint interrupted in Enumerable.Range(0,Steps).Select(s=>StepEntity(portal,s)).Append(ArchEntity(portal))) {
            enabled.Clear();enabled.Add(interrupted);
            foreach(var i in visual.Instructions.TakeWhile(i=>i.Bank!=3))if(i.Bank==2005&&i.ID==3&&i.ArgData[4]==0)enabled.Remove(BitConverter.ToUInt32(i.ArgData));
            if(enabled.Count>0)throw new Exception("Interrupted reveal not cleaned up");
        }
        Console.WriteLine("Portal reveal: flags, idempotency, visibility, interrupted reset and binary roundtrip passed");
    }
}
