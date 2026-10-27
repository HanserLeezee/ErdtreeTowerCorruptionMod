using SoulsFormats;
// 职业重做后的职业机制：六个职业的常驻效果放在 9810000 段（编号 = 9810000 + 职业序号×10 + 序号），
// 只对某类攻击生效的复制词条系统用过的原版护符行（保留攻击类型过滤）只改倍率，其余复制 9600011。运行时按职业挂上。
static class ClassAssets {
    public const int First=9810000;
    static readonly string[] Cuts=["slashDamageCutRate","blowDamageCutRate","thrustDamageCutRate","neutralDamageCutRate","magicDamageCutRate","fireDamageCutRate","thunderDamageCutRate","darkDamageCutRate"];
    static readonly string[] Attacks=["physicsAttackPowerRate","magicAttackPowerRate","fireAttackPowerRate","thunderAttackPowerRate","darkAttackPowerRate"];
    static readonly string[] Kept=["spCategory","categoryPriority","effectTargetOpposeTarget","effectTargetFriendlyTarget","effectTargetSelfTarget","effectTargetSelf","effectTargetFriend","effectTargetEnemy","effectTargetPlayer","effectTargetAI","effectTargetLive","effectTargetGhost","bCurrHPIndependeMaxHP"];
    static (string,object)[] All(string[] fields,float value)=>fields.Select(f=>(f,(object)value)).ToArray();
    // (编号, 名字, 复制的原版行, 攻击倍率, 其他字段)
    static (int id,string name,int template,float attack,(string,object)[] fields)[] Rows=>[
        (First+0,"Tower class oathsworn guard counter",322000,1.3f,[]),
        (First+1,"Tower class oathsworn guard stamina",341000,1f,[("guardStaminaMult",0.7f)]),
        (First+10,"Tower class berserker low health",321300,1.2f,[]),
        (First+20,"Tower class shadow critical",320900,1.3f,[]),
        (First+21,"Tower class shadow stamina",9600011,0f,[("staminaRecoverChangeSpeed",5),("artsConsumptionRate",1f)]),
        (First+30,"Tower class sorcerer focus",9600011,0f,[("magicConsumptionRate",0.85f),("artsConsumptionRate",1f)]),
        (First+40,"Tower class cleric focus",9600011,0f,[("miracleConsumptionRate",0.85f),("artsConsumptionRate",1f)]),
    ];
    static void Set(PARAM.Row r,string key,object value)=>r[key].Value=Convert.ChangeType(value,r[key].Value.GetType());
    static PARAM.Row Create(PARAM effects,(int id,string name,int template,float attack,(string,object)[] fields) spec) {
        var row=new PARAM.Row(effects.Rows.Single(r=>r.ID==spec.template)){ID=spec.id,Name=spec.name};
        foreach(var (key,value) in new (string,object)[]{("iconId",-1),("effectEndurance",-1f),("spCategory",0),("categoryPriority",0),("saveCategory",-1),("dontDeleteOnDead",0)})Set(row,key,value);
        if(spec.attack>0) {
            foreach(var element in new[]{"physics","magic","fire","thunder","dark"})Set(row,element+"AttackRate",spec.attack);
            foreach(var target in new[]{"Player","Enemy"})foreach(var element in new[]{"Physics","Magic","Fire","Thunder","Dark"})Set(row,$"atk{target}DmgCorrectRate_{element}",1f);
        }
        foreach(var (key,value) in spec.fields)Set(row,key,value);
        return row;
    }
    static void Verify(PARAM effects) {
        foreach(var spec in Rows) {
            var row=effects.Rows.Single(r=>r.ID==spec.id);var expected=Create(effects,spec);
            foreach(var cell in expected.Cells)if(!System.Collections.StructuralComparisons.StructuralEqualityComparer.Equals(cell.Value,row[cell.Def.InternalName].Value))throw new Exception($"Class effect mismatch: {spec.id} {cell.Def.InternalName}");
            if(Convert.ToInt32(row["spCategory"].Value)!=0||Convert.ToSingle(row["effectEndurance"].Value)!=-1f)throw new Exception($"Class effect must be resident and unstacked: {spec.id}");
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
        if(existing.Count>0) {
            // 已经打过补丁：只校验，原样写出，保证可以重复运行
            if(existing.Count!=Rows.Length)throw new Exception("Class effect range 9810000-9810099 already in use");
            Verify(effects);
        } else {
            foreach(var spec in Rows)effects.Rows.Add(Create(effects,spec));
            effects.Rows.Sort((a,b)=>a.ID.CompareTo(b.ID));file.Bytes=effects.Write();
        }
        var others=reg.Files.Where(f=>f!=file).ToDictionary(f=>f.Name,f=>f.Bytes.ToArray());
        Directory.CreateDirectory(output);var path=Path.Combine(output,"regulation.bin");
        SoulsFormats.Cryptography.RegulationDecryptor.EncryptERRegulation(path,reg);
        var check=SoulsFormats.Cryptography.RegulationDecryptor.DecryptERRegulation(path);
        foreach(var f in check.Files.Where(f=>f.Name!=file.Name))if(!f.Bytes.SequenceEqual(others[f.Name]))throw new Exception($"Unrelated param changed: {f.Name}");
        var actual=PARAM.Read(check.Files.Single(f=>f.Name==file.Name).Bytes);actual.ApplyParamdef(def);Verify(actual);
        if(actual.Rows.Count!=original.Rows.Count+(existing.Count>0?0:Rows.Length))throw new Exception("Unexpected effect count");
        var rows=actual.Rows.ToDictionary(r=>r.ID);
        foreach(var row in original.Rows)foreach(var cell in row.Cells)if(!System.Collections.StructuralComparisons.StructuralEqualityComparer.Equals(cell.Value,rows[row.ID][cell.Def.InternalName].Value))throw new Exception($"Existing effect changed: {row.ID}");
        string Hash(string p)=>Convert.ToHexString(System.Security.Cryptography.SHA256.HashData(File.ReadAllBytes(p)));
        File.WriteAllText(Path.Combine(output,"class-build.json"),System.Text.Json.JsonSerializer.Serialize(new{effects=Rows.Select(r=>r.id),sourceRegulationSha256=Hash(Path.Combine(input,"regulation.bin")),regulationSha256=Hash(path)},new System.Text.Json.JsonSerializerOptions{WriteIndented=true}));
        Console.WriteLine($"Verified {Rows.Length} class effects; all existing regulation data preserved");
    }
}
