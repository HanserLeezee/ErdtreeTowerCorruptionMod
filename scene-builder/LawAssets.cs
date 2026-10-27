using SoulsFormats;
// 苦难律法：开局自选的难度节点。敌人侧沿用每层递增效果的模板（7030 清空为 90400），玩家侧沿用幕间誓约验证过的模板（321300）。
// 运行时按律法给敌人与玩家挂这些常驻效果；超时失血只在限时战斗超时后施加。
static class LawAssets {
    public const int First=9660000;
    static readonly string[] Cuts=["slashDamageCutRate","blowDamageCutRate","thrustDamageCutRate","neutralDamageCutRate","magicDamageCutRate","fireDamageCutRate","thunderDamageCutRate","darkDamageCutRate"];
    static readonly string[] Attacks=["physicsAttackPowerRate","magicAttackPowerRate","fireAttackPowerRate","thunderAttackPowerRate","darkAttackPowerRate"];
    static readonly string[] Elements=["magicDamageCutRate","fireDamageCutRate","thunderDamageCutRate","darkDamageCutRate"];
    static readonly string[] Status=["registPoizonChangeRate","registDiseaseChangeRate","registBloodChangeRate","registFreezeChangeRate","registSleepChangeRate","registMadnessChangeRate","registCurseChangeRate"];
    static readonly string[] Kept=["spCategory","categoryPriority","effectTargetOpposeTarget","effectTargetFriendlyTarget","effectTargetSelfTarget","effectTargetSelf","effectTargetFriend","effectTargetEnemy","effectTargetPlayer","effectTargetAI","effectTargetLive","effectTargetGhost","bCurrHPIndependeMaxHP"];
    static (string,object)[] All(string[] fields,float value)=>fields.Select(f=>(f,(object)value)).ToArray();
    static (int id,string name,bool player,(string,object)[] fields)[] Rows=>[
        (First+1,"Tower law strong enemies 1",false,[("maxHpRate",1.10f),..All(Attacks,1.06f)]),
        (First+2,"Tower law strong enemies 2",false,[("maxHpRate",1.21f),..All(Attacks,1.13f)]),
        (First+3,"Tower law strong enemies 3",false,[("maxHpRate",1.45f),..All(Attacks,1.28f)]),
        (First+6,"Tower law unshakable 1",false,[("toughnessDamageCutRate",0.7f)]),
        (First+7,"Tower law unshakable 2",false,[("toughnessDamageCutRate",0.56f)]),
        (First+11,"Tower law fragile 1",true,[..All(Cuts,1.10f)]),
        (First+12,"Tower law fragile 2",true,[..All(Cuts,1.20f)]),
        (First+13,"Tower law fragile 3",true,[..All(Cuts,1.35f)]),
        (First+21,"Tower law fatigue 1",true,[("staminaRecoverChangeSpeed",-9)]),
        (First+22,"Tower law fatigue 2",true,[("staminaRecoverChangeSpeed",-18)]),
        (First+31,"Tower law focus 1",true,[("artsConsumptionRate",1.25f),("magicConsumptionRate",1.25f),("miracleConsumptionRate",1.25f)]),
        (First+32,"Tower law focus 2",true,[("artsConsumptionRate",1.5f),("magicConsumptionRate",1.5f),("miracleConsumptionRate",1.5f)]),
        (First+41,"Tower law poverty 1",true,[("soulRate",0.85f)]),
        (First+42,"Tower law poverty 2",true,[("soulRate",0.7f)]),
        (First+50,"Tower law overtime bleed",true,[("motionInterval",1f),("changeHpRate",2f),("vfxId",-1)]),
        (First+61,"Tower law elemental resistance 1",false,[..All(Elements,0.9f)]),
        (First+62,"Tower law elemental resistance 2",false,[..All(Elements,0.8f)]),
        (First+71,"Tower law status resistance 1",false,[..All(Status,1.15f)]),
        (First+72,"Tower law status resistance 2",false,[..All(Status,1.3f)]),
    ];
    static void Set(PARAM.Row r,string key,object value)=>r[key].Value=Convert.ChangeType(value,r[key].Value.GetType());
    static PARAM.Row Create(PARAM effects,(int id,string name,bool player,(string,object)[] fields) spec) {
        PARAM.Row row;
        if(spec.player) {
            row=new PARAM.Row(effects.Rows.Single(r=>r.ID==321300)){ID=spec.id,Name=spec.name};
            foreach(var (key,value) in new (string,object)[]{("iconId",-1),("effectEndurance",-1f),("spCategory",0),("categoryPriority",0),("saveCategory",-1),("dontDeleteOnDead",0),("magicSubCategoryChange1",0)})Set(row,key,value);
            foreach(var element in new[]{"physics","magic","fire","thunder","dark"})Set(row,element+"AttackRate",1f);
            foreach(var target in new[]{"Player","Enemy"})foreach(var element in new[]{"Physics","Magic","Fire","Thunder","Dark"})Set(row,$"atk{target}DmgCorrectRate_{element}",1f);
        } else {
            var neutral=effects.Rows.Single(r=>r.ID==90400);
            row=new PARAM.Row(effects.Rows.Single(r=>r.ID==7030)){ID=spec.id,Name=spec.name};
            foreach(var cell in row.Cells.Where(c=>!Kept.Contains(c.Def.InternalName)))cell.Value=neutral[cell.Def.InternalName].Value;
            foreach(var (key,value) in new (string,object)[]{("effectEndurance",-1f),("spCategory",0),("categoryPriority",0),("saveCategory",-1)})Set(row,key,value);
        }
        foreach(var (key,value) in spec.fields)Set(row,key,value);
        return row;
    }
    static void Verify(PARAM effects) {
        foreach(var spec in Rows) {
            var row=effects.Rows.Single(r=>r.ID==spec.id);var expected=Create(effects,spec);
            foreach(var cell in expected.Cells)if(!System.Collections.StructuralComparisons.StructuralEqualityComparer.Equals(cell.Value,row[cell.Def.InternalName].Value))throw new Exception($"Law effect mismatch: {spec.id} {cell.Def.InternalName}");
            if(Convert.ToInt32(row["spCategory"].Value)!=0||Convert.ToSingle(row["effectEndurance"].Value)!=-1f)throw new Exception($"Law effect must be resident and unstacked: {spec.id}");
        }
    }
    public static void Patch(string input,string output) {
        if(Path.GetFullPath(input)==Path.GetFullPath(output))throw new Exception("Separate staging output required");
        var reg=SoulsFormats.Cryptography.RegulationDecryptor.DecryptERRegulation(Path.Combine(input,"regulation.bin"));
        var file=reg.Files.Single(f=>Path.GetFileNameWithoutExtension(f.Name.Replace('\\','/'))=="SpEffectParam");
        var def=PARAMDEF.XmlDeserialize("work/SoulsRandomizers/diste/Defs/SpEffect.xml");
        var original=PARAM.Read(file.Bytes);original.ApplyParamdef(def);
        var effects=PARAM.Read(file.Bytes);effects.ApplyParamdef(def);
        var existing=effects.Rows.Where(r=>r.ID>=First&&r.ID<First+100).ToList();
        var owned=Rows.Select(r=>r.id).ToHashSet();
        if(existing.Any(r=>!owned.Contains(r.ID)))throw new Exception("Law effect range 9660000-9660099 already in use");
        foreach(var row in existing) {
            var spec=Rows.Single(r=>r.id==row.ID);var expected=Create(effects,spec);
            foreach(var cell in expected.Cells) {
                var key=cell.Def.InternalName;
                if(System.Collections.StructuralComparisons.StructuralEqualityComparer.Equals(cell.Value,row[key].Value))continue;
                if(key=="soulRate"&&((row.ID==First+41&&Convert.ToSingle(row[key].Value)==0.75f)||(row.ID==First+42&&Convert.ToSingle(row[key].Value)==0.5f)))continue;
                throw new Exception($"Unexpected existing law effect: {row.ID} {key}");
            }
        }
        effects.Rows.RemoveAll(r=>owned.Contains(r.ID));
        foreach(var spec in Rows)effects.Rows.Add(Create(effects,spec));
        effects.Rows.Sort((a,b)=>a.ID.CompareTo(b.ID));file.Bytes=effects.Write();
        var others=reg.Files.Where(f=>f!=file).ToDictionary(f=>f.Name,f=>f.Bytes.ToArray());
        Directory.CreateDirectory(output);var path=Path.Combine(output,"regulation.bin");
        SoulsFormats.Cryptography.RegulationDecryptor.EncryptERRegulation(path,reg);
        var check=SoulsFormats.Cryptography.RegulationDecryptor.DecryptERRegulation(path);
        foreach(var f in check.Files.Where(f=>f.Name!=file.Name))if(!f.Bytes.SequenceEqual(others[f.Name]))throw new Exception($"Unrelated param changed: {f.Name}");
        var actual=PARAM.Read(check.Files.Single(f=>f.Name==file.Name).Bytes);actual.ApplyParamdef(def);Verify(actual);
        if(actual.Rows.Count!=original.Rows.Count+Rows.Length-existing.Count)throw new Exception("Unexpected effect count");
        var rows=actual.Rows.ToDictionary(r=>r.ID);
        foreach(var row in original.Rows.Where(r=>!owned.Contains(r.ID)))foreach(var cell in row.Cells)if(!System.Collections.StructuralComparisons.StructuralEqualityComparer.Equals(cell.Value,rows[row.ID][cell.Def.InternalName].Value))throw new Exception($"Existing effect changed: {row.ID}");
        string Hash(string p)=>Convert.ToHexString(System.Security.Cryptography.SHA256.HashData(File.ReadAllBytes(p)));
        File.WriteAllText(Path.Combine(output,"law-build.json"),System.Text.Json.JsonSerializer.Serialize(new{effects=Rows.Select(r=>r.id),sourceRegulationSha256=Hash(Path.Combine(input,"regulation.bin")),regulationSha256=Hash(path)},new System.Text.Json.JsonSerializerOptions{WriteIndented=true}));
        Console.WriteLine($"Verified {Rows.Length} law effects; all unrelated regulation data preserved");
    }
}
