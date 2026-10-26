using SoulsFormats;
// 余烬：誓约三选一里的机制增益。参数类效果放在 9670000 段，其余由运行时按事件施加。
// 无声处刑复制原版暗杀者匕首护符的三段触发（常驻 → 致命一击时触发 → 生效），保持相同的编号间隔。
static class EmberAssets {
    public const int First=9670000;
    const float ExecutionSeconds=5f,ExecutionStamina=0.5f,RevengeRate=1.3f,FrenzyStep=0.02f;
    const int Buildup=12,FrenzyStacks=8,FlashVfx=1605000;
    static readonly string[] Kept=["spCategory","categoryPriority","effectTargetOpposeTarget","effectTargetFriendlyTarget","effectTargetSelfTarget","effectTargetSelf","effectTargetFriend","effectTargetEnemy","effectTargetPlayer","effectTargetAI","effectTargetLive","effectTargetGhost"];
    static void Set(PARAM.Row r,string key,object value)=>r[key].Value=Convert.ChangeType(value,r[key].Value.GetType());
    static PARAM.Row Player(PARAM p,int id,string name,float attack,float endurance) {
        var r=new PARAM.Row(p.Rows.Single(r=>r.ID==321300)){ID=id,Name=name};
        foreach(var (key,value) in new (string,object)[]{("iconId",-1),("effectEndurance",endurance),("spCategory",0),("categoryPriority",0),("saveCategory",-1),("dontDeleteOnDead",0),("magicSubCategoryChange1",0)})Set(r,key,value);
        foreach(var element in new[]{"physics","magic","fire","thunder","dark"})Set(r,element+"AttackRate",attack);
        foreach(var target in new[]{"Player","Enemy"})foreach(var element in new[]{"Physics","Magic","Fire","Thunder","Dark"})Set(r,$"atk{target}DmgCorrectRate_{element}",1f);
        return r;
    }
    static PARAM.Row Enemy(PARAM p,int id,string name,float endurance) {
        var neutral=p.Rows.Single(r=>r.ID==90400);
        var r=new PARAM.Row(p.Rows.Single(r=>r.ID==7030)){ID=id,Name=name};
        foreach(var cell in r.Cells.Where(c=>!Kept.Contains(c.Def.InternalName)))cell.Value=neutral[cell.Def.InternalName].Value;
        foreach(var (key,value) in new (string,object)[]{("effectEndurance",endurance),("spCategory",0),("categoryPriority",0),("saveCategory",-1)})Set(r,key,value);
        return r;
    }
    static List<PARAM.Row> Build(PARAM p) {
        var rows=new List<PARAM.Row>();
        // 无声处刑：常驻（stateInfo 288）→ 触发 → 致命一击后 5 秒精力消耗减半
        var resident=new PARAM.Row(p.Rows.Single(r=>r.ID==350500)){ID=First,Name="Tower ember execution"};
        Set(resident,"iconId",-1);Set(resident,"spCategory",0);Set(resident,"categoryPriority",0);rows.Add(resident);
        var trigger=new PARAM.Row(p.Rows.Single(r=>r.ID==350501)){ID=First+1,Name="Tower ember execution trigger"};
        Set(trigger,"replaceSpEffectId",First+2);Set(trigger,"spCategory",0);Set(trigger,"categoryPriority",0);rows.Add(trigger);
        var relief=new PARAM.Row(p.Rows.Single(r=>r.ID==350502)){ID=First+2,Name="Tower ember execution relief"};
        foreach(var (key,value) in new (string,object)[]{("changeHpRate",0f),("changeHpPoint",0),("motionInterval",0f),("effectEndurance",ExecutionSeconds),("consumeStaminaRate",ExecutionStamina),("spCategory",0),("categoryPriority",0)})Set(relief,key,value);
        rows.Add(relief);
        var blood=Enemy(p,First+10,"Tower ember blood buildup",0f);Set(blood,"bloodAttackPower",Buildup);rows.Add(blood);
        var frost=Enemy(p,First+11,"Tower ember frost buildup",0f);Set(frost,"freezeAttackPower",Buildup);rows.Add(frost);
        rows.Add(Player(p,First+20,"Tower ember revenge",RevengeRate,-1f));
        for(int stack=1;stack<=FrenzyStacks;stack++)rows.Add(Player(p,First+30+stack,$"Tower ember frenzy {stack}",1f+FrenzyStep*stack,-1f));
        var flash=Enemy(p,First+40,"Tower ember flash",1f);Set(flash,"vfxId",FlashVfx);rows.Add(flash);
        return rows;
    }
    static void Verify(PARAM p) {
        foreach(var expected in Build(p)) {
            var actual=p.Rows.Single(r=>r.ID==expected.ID);
            foreach(var cell in expected.Cells)if(!System.Collections.StructuralComparisons.StructuralEqualityComparer.Equals(cell.Value,actual[cell.Def.InternalName].Value))throw new Exception($"Ember effect mismatch: {expected.ID} {cell.Def.InternalName}");
        }
        if(Convert.ToInt32(p.Rows.Single(r=>r.ID==First)["stateInfo"].Value)!=288||Convert.ToInt32(p.Rows.Single(r=>r.ID==First+1)["invocationConditionsStateChange1"].Value)!=288)throw new Exception("Execution trigger lost its critical-hit condition");
    }
    public static void Patch(string input,string output) {
        if(Path.GetFullPath(input)==Path.GetFullPath(output))throw new Exception("Separate staging output required");
        var reg=SoulsFormats.Cryptography.RegulationDecryptor.DecryptERRegulation(Path.Combine(input,"regulation.bin"));
        var file=reg.Files.Single(f=>Path.GetFileNameWithoutExtension(f.Name.Replace('\\','/'))=="SpEffectParam");
        var def=PARAMDEF.XmlDeserialize("work/SoulsRandomizers/diste/Defs/SpEffect.xml");
        var original=PARAM.Read(file.Bytes);original.ApplyParamdef(def);
        var effects=PARAM.Read(file.Bytes);effects.ApplyParamdef(def);
        var count=Build(effects).Count;
        var existing=effects.Rows.Count(r=>r.ID>=First&&r.ID<First+100);
        if(existing==count) Verify(effects);
        else if(existing>0) throw new Exception("Ember effect range 9670000-9670099 already in use");
        else {foreach(var row in Build(effects))effects.Rows.Add(row);effects.Rows.Sort((a,b)=>a.ID.CompareTo(b.ID));file.Bytes=effects.Write();}
        var others=reg.Files.Where(f=>f!=file).ToDictionary(f=>f.Name,f=>f.Bytes.ToArray());
        Directory.CreateDirectory(output);var path=Path.Combine(output,"regulation.bin");
        SoulsFormats.Cryptography.RegulationDecryptor.EncryptERRegulation(path,reg);
        var check=SoulsFormats.Cryptography.RegulationDecryptor.DecryptERRegulation(path);
        foreach(var f in check.Files.Where(f=>f.Name!=file.Name))if(!f.Bytes.SequenceEqual(others[f.Name]))throw new Exception($"Unrelated param changed: {f.Name}");
        var actual=PARAM.Read(check.Files.Single(f=>f.Name==file.Name).Bytes);actual.ApplyParamdef(def);Verify(actual);
        if(actual.Rows.Count!=original.Rows.Count+(existing>0?0:count))throw new Exception("Unexpected effect count");
        var rows=actual.Rows.ToDictionary(r=>r.ID);
        foreach(var row in original.Rows)foreach(var cell in row.Cells)if(!System.Collections.StructuralComparisons.StructuralEqualityComparer.Equals(cell.Value,rows[row.ID][cell.Def.InternalName].Value))throw new Exception($"Existing effect changed: {row.ID}");
        string Hash(string p)=>Convert.ToHexString(System.Security.Cryptography.SHA256.HashData(File.ReadAllBytes(p)));
        File.WriteAllText(Path.Combine(output,"ember-build.json"),System.Text.Json.JsonSerializer.Serialize(new{effects=actual.Rows.Where(r=>r.ID>=First&&r.ID<First+100).Select(r=>r.ID),sourceRegulationSha256=Hash(Path.Combine(input,"regulation.bin")),regulationSha256=Hash(path)},new System.Text.Json.JsonSerializerOptions{WriteIndented=true}));
        Console.WriteLine($"Verified {count} ember effects; all existing regulation data preserved");
    }
}
