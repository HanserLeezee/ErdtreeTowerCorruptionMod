using SoulsFormats;

static class CountdownAssets {
    public const int Effect=9870100;
    public static void Patch(string input,string output) {
        if(Path.GetFullPath(input)==Path.GetFullPath(output))throw new Exception("Separate staging required");
        var reg=SoulsFormats.Cryptography.RegulationDecryptor.DecryptERRegulation(Path.Combine(input,"regulation.bin"));
        var file=reg.Files.Single(f=>Path.GetFileNameWithoutExtension(f.Name.Replace('\\','/'))=="SpEffectParam");
        var def=PARAMDEF.XmlDeserialize("work/SoulsRandomizers/diste/Defs/SpEffect.xml");
        var effects=PARAM.Read(file.Bytes);effects.ApplyParamdef(def);
        var before=effects.Rows.Where(r=>r.ID!=Effect).ToDictionary(r=>r.ID,r=>r.Cells.Select(c=>c.Value).ToArray());
        if(effects.Rows.Any(r=>r.ID==Effect&&r.Name!="Tower countdown ground"))throw new Exception("Countdown effect ID collision");
        var row=new PARAM.Row(effects.Rows.Single(r=>r.ID==90400)){ID=Effect,Name="Tower countdown ground"};
        void Set(string key,object value)=>row[key].Value=Convert.ChangeType(value,row[key].Value.GetType());
        foreach(var (key,value) in new (string,object)[]{("effectEndurance",-1f),("spCategory",0),("categoryPriority",0),("saveCategory",-1),("iconId",-1),("dontDeleteOnDead",0),("vfxId",9871021),("vfxId1",-1),("vfxId2",-1)})Set(key,value);
        var vfx=PARAM.Read(reg.Files.Single(f=>Path.GetFileNameWithoutExtension(f.Name.Replace('\\','/'))=="SpEffectVfxParam").Bytes);
        vfx.ApplyParamdef(PARAMDEF.XmlDeserialize("work/SoulsRandomizers/diste/Defs/SpEffectVfx.xml"));
        var visual=vfx.Rows.Single(r=>r.ID==9871021);
        if(Convert.ToInt32(visual["midstSfxId"].Value)!=AuraFfxAssets.Gold||Convert.ToInt32(visual["midstDmyId"].Value)!=-1)throw new Exception("Root-attached countdown VFX missing");
        effects.Rows.RemoveAll(r=>r.ID==Effect);effects.Rows.Add(row);effects.Rows.Sort((a,b)=>a.ID.CompareTo(b.ID));file.Bytes=effects.Write();
        Directory.CreateDirectory(output);var path=Path.Combine(output,"regulation.bin");SoulsFormats.Cryptography.RegulationDecryptor.EncryptERRegulation(path,reg);
        var check=SoulsFormats.Cryptography.RegulationDecryptor.DecryptERRegulation(path);
        foreach(var other in reg.Files.Where(f=>f.Name!=file.Name))if(!other.Bytes.SequenceEqual(check.Files.Single(f=>f.Name==other.Name).Bytes))throw new Exception("Unrelated countdown parameter changed");
        var actual=PARAM.Read(check.Files.Single(f=>f.Name==file.Name).Bytes);actual.ApplyParamdef(def);
        foreach(var r in actual.Rows)if(r.ID!=Effect&&!r.Cells.Select(c=>c.Value).SequenceEqual(before[r.ID]))throw new Exception("Unrelated effect changed");
        if(!actual.Rows.Single(r=>r.ID==Effect).Cells.Select(c=>c.Value).SequenceEqual(row.Cells.Select(c=>c.Value)))throw new Exception("Countdown effect roundtrip mismatch");
        Console.WriteLine("Verified cosmetic root-attached countdown effect 9870100; existing parameters preserved");
    }
}
