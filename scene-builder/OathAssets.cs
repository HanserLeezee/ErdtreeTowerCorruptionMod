using SoulsFormats;
// 幕间誓约重做：每份誓约 I–III 级各一条增益效果，复制旧誓约的增益行（保留原版攻击类型过滤）只改倍率；
// 连击、坚盾换用更轻的代价行。旧行原样保留，旧规则存档不受影响。编号为 9600000+誓约序号×10+3…9。
static class OathAssets {
    const int First=ContractAssets.First;
    static void Set(PARAM.Row r,string key,object value)=>r[key].Value=Convert.ChangeType(value,r[key].Value.GetType());
    static void Damage(PARAM.Row r,float rate) {
        foreach(var element in new[]{"physics","magic","fire","thunder","dark"})Set(r,element+"AttackRate",rate);
        foreach(var target in new[]{"Player","Enemy"})foreach(var element in new[]{"Physics","Magic","Fire","Thunder","Dark"})Set(r,$"atk{target}DmgCorrectRate_{element}",1f);
    }
    // (新行偏移, 复制的旧行偏移, 倍率, 精力恢复)；处刑的旧代价对所有攻击 ×0.9，增益行先除以 0.9
    static readonly (int offset,int source,float rate,int stamina)[] Boons=[
        (3,0,1.35f,0),(4,0,1.55f,0),(5,0,1.80f,0),
        (13,10,1.35f,0),(14,10,1.55f,0),(15,10,1.80f,0),
        (23,20,1.35f,0),(24,20,1.55f,0),(25,20,1.80f,0),
        (33,30,1.35f,0),(34,30,1.55f,0),(35,30,1.80f,0),
        (43,40,1.25f,0),(44,40,1.40f,0),(45,40,1.55f,0),(46,41,1.25f,0),(47,41,1.40f,0),(48,41,1.55f,0),
        (53,50,1.35f,0),(54,50,1.55f,0),(55,50,1.80f,0),
        (63,60,1.35f/0.9f,0),(64,60,1.55f/0.9f,0),(65,60,1.80f/0.9f,0),
        (73,70,1.20f,5),(74,70,1.30f,7),(75,70,1.40f,10),
        (83,80,1.35f,0),(84,80,1.55f,0),(85,80,1.75f,0),
    ];
    static readonly int[] Banes=[29,39];
    static int[] Offsets=>Boons.Select(b=>b.offset).Concat(Banes).ToArray();
    static PARAM.Row Create(PARAM p,int offset) {
        if(offset==29) {
            var r=new PARAM.Row(p.Rows.Single(r=>r.ID==First+21)){ID=First+offset,Name="Tower oath combo drawback"};
            foreach(var element in new[]{"slash","blow","thrust","neutral","magic","fire","thunder","dark"})Set(r,element+"DamageCutRate",1.05f);
            return r;
        }
        if(offset==39) {
            var r=new PARAM.Row(p.Rows.Single(r=>r.ID==First+31)){ID=First+offset,Name="Tower oath guard drawback"};
            Set(r,"consumeStaminaRate",1.1f);return r;
        }
        var spec=Boons.Single(b=>b.offset==offset);
        var row=new PARAM.Row(p.Rows.Single(r=>r.ID==First+spec.source)){ID=First+offset,Name=$"Tower oath level {(offset%10-3)%3+1}"};
        Damage(row,spec.rate);
        if(spec.stamina!=0)Set(row,"staminaRecoverChangeSpeed",spec.stamina);
        return row;
    }
    static void Verify(PARAM p) {
        foreach(int offset in Offsets) {
            var expected=Create(p,offset);var actual=p.Rows.Single(r=>r.ID==First+offset);
            foreach(var cell in expected.Cells)if(!System.Collections.StructuralComparisons.StructuralEqualityComparer.Equals(cell.Value,actual[cell.Def.InternalName].Value))throw new Exception($"Oath effect mismatch: {First+offset} {cell.Def.InternalName}");
            if(Convert.ToInt32(actual["spCategory"].Value)!=0||Convert.ToSingle(actual["effectEndurance"].Value)!=-1f||Convert.ToInt32(actual["saveCategory"].Value)!=-1)throw new Exception($"Oath effect must be resident: {First+offset}");
        }
        foreach(var (offset,source,_,_) in Boons) {
            var row=p.Rows.Single(r=>r.ID==First+offset);var original=p.Rows.Single(r=>r.ID==First+source);
            foreach(var key in new[]{"magicSubCategoryChange1","magicSubCategoryChange2","magicSubCategoryChange3","stateInfo","magParamChange","miracleParamChange"})
                if(!Equals(row[key].Value,original[key].Value))throw new Exception($"Oath lost native attack filtering: {First+offset}");
        }
    }
    public static void Patch(string input,string output) {
        if(Path.GetFullPath(input)==Path.GetFullPath(output))throw new Exception("Separate staging output required");
        var reg=SoulsFormats.Cryptography.RegulationDecryptor.DecryptERRegulation(Path.Combine(input,"regulation.bin"));
        var file=reg.Files.Single(f=>Path.GetFileNameWithoutExtension(f.Name.Replace('\\','/'))=="SpEffectParam");
        var def=PARAMDEF.XmlDeserialize("work/SoulsRandomizers/diste/Defs/SpEffect.xml");
        var original=PARAM.Read(file.Bytes);original.ApplyParamdef(def);
        var effects=PARAM.Read(file.Bytes);effects.ApplyParamdef(def);
        var existing=Offsets.Count(o=>effects.Rows.Any(r=>r.ID==First+o));
        if(existing==Offsets.Length) Verify(effects);
        else if(existing>0) throw new Exception("Oath effect IDs partly in use");
        else {
            foreach(int offset in Offsets)effects.Rows.Add(Create(effects,offset));
            effects.Rows.Sort((a,b)=>a.ID.CompareTo(b.ID));file.Bytes=effects.Write();
        }
        var others=reg.Files.Where(f=>f!=file).ToDictionary(f=>f.Name,f=>f.Bytes.ToArray());
        Directory.CreateDirectory(output);var path=Path.Combine(output,"regulation.bin");
        SoulsFormats.Cryptography.RegulationDecryptor.EncryptERRegulation(path,reg);
        var check=SoulsFormats.Cryptography.RegulationDecryptor.DecryptERRegulation(path);
        foreach(var f in check.Files.Where(f=>f.Name!=file.Name))if(!f.Bytes.SequenceEqual(others[f.Name]))throw new Exception($"Unrelated param changed: {f.Name}");
        var actual=PARAM.Read(check.Files.Single(f=>f.Name==file.Name).Bytes);actual.ApplyParamdef(def);Verify(actual);
        if(actual.Rows.Count!=original.Rows.Count+(existing>0?0:Offsets.Length))throw new Exception("Unexpected effect count");
        var rows=actual.Rows.ToDictionary(r=>r.ID);
        foreach(var row in original.Rows)foreach(var cell in row.Cells)if(!System.Collections.StructuralComparisons.StructuralEqualityComparer.Equals(cell.Value,rows[row.ID][cell.Def.InternalName].Value))throw new Exception($"Existing effect changed: {row.ID}");
        string Hash(string p)=>Convert.ToHexString(System.Security.Cryptography.SHA256.HashData(File.ReadAllBytes(p)));
        File.WriteAllText(Path.Combine(output,"oath-build.json"),System.Text.Json.JsonSerializer.Serialize(new{effects=Offsets.Select(o=>First+o),sourceRegulationSha256=Hash(Path.Combine(input,"regulation.bin")),regulationSha256=Hash(path)},new System.Text.Json.JsonSerializerOptions{WriteIndented=true}));
        Console.WriteLine($"Verified {Offsets.Length} oath effects; all existing regulation data preserved");
    }
}
