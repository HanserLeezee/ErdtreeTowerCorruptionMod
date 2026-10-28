using SoulsFormats;
// 角色光环：局外外观奖励，本机角色身上常驻一个原版增益的身体光效（只有视觉，不加任何数值）。
// 效果放在 9870000 段（编号 = 9870000 + 光环编号，与 src/auras.rs 的 AURAS 一致），复制中性的 90400 后只改持续时间、类别与光效；
// 由原生 DLL 挂在本机角色身上（同步给联机队友），换光环或摘下时撤下。
static class AuraAssets {
    public const int First=9870000;
    const int Neutral=90400;
    // (编号, 名字, 原版光效行)：火焰赐我力量、赐福之恩、风之结晶露滴、刺客的手法、鲜血君王的欢愉、神圣加护、黑焰庇佑、黄金誓言、黄金雷电
    static (int id,string name,int vfx)[] Rows=>[
        (First+1,"Tower aura ember",1605000),(First+2,"Tower aura blessing",1643000),(First+3,"Tower aura wind",5846),
        (First+4,"Tower aura assassin",1651000),(First+5,"Tower aura blood",321600),(First+6,"Tower aura holy",1648000),
        (First+7,"Tower aura black flame",1627000),(First+8,"Tower aura golden vow",1660000),(First+9,"Tower aura golden lightning",1790300),
    ];
    static void Set(PARAM.Row r,string key,object value)=>r[key].Value=Convert.ChangeType(value,r[key].Value.GetType());
    static PARAM.Row Create(PARAM effects,(int id,string name,int vfx) spec) {
        var row=new PARAM.Row(effects.Rows.Single(r=>r.ID==Neutral)){ID=spec.id,Name=spec.name};
        // 不占任何增益类别，不会顶掉玩家自己的祷告或道具效果；不存档、无图标，持续到原生 DLL 撤下
        foreach(var (key,value) in new (string,object)[]{("effectEndurance",-1f),("spCategory",0),("categoryPriority",0),("saveCategory",-1),("iconId",-1),("dontDeleteOnDead",0),("vfxId",spec.vfx)})Set(row,key,value);
        return row;
    }
    static void Verify(PARAM effects,PARAM vfx) {
        foreach(var spec in Rows) {
            var row=effects.Rows.Single(r=>r.ID==spec.id);var expected=Create(effects,spec);
            foreach(var cell in expected.Cells)if(!System.Collections.StructuralComparisons.StructuralEqualityComparer.Equals(cell.Value,row[cell.Def.InternalName].Value))throw new Exception($"Aura effect mismatch: {spec.id} {cell.Def.InternalName}");
            var look=vfx.Rows.Single(r=>r.ID==spec.vfx);
            // 光效必须是挂在身上的常驻光效（effectType 0、有持续特效），不是刀身附魔
            if(Convert.ToInt32(look["effectType"].Value)!=0||Convert.ToInt32(look["midstSfxId"].Value)<=0||Convert.ToInt32(look["enchantStartDmyId_0"].Value)>=0)throw new Exception($"Aura must use a body visual: {spec.id}");
            foreach(var f in new[]{"spCategory","iconId"})if(Convert.ToInt32(row[f].Value)!=(f=="iconId"?-1:0))throw new Exception($"Aura must stay cosmetic: {spec.id} {f}");
        }
    }
    public static void Patch(string input,string output) {
        if(Path.GetFullPath(input)==Path.GetFullPath(output))throw new Exception("Separate staging output required");
        var reg=SoulsFormats.Cryptography.RegulationDecryptor.DecryptERRegulation(Path.Combine(input,"regulation.bin"));
        var file=reg.Files.Single(f=>Path.GetFileNameWithoutExtension(f.Name.Replace('\\','/'))=="SpEffectParam");
        var vfxFile=reg.Files.Single(f=>Path.GetFileNameWithoutExtension(f.Name.Replace('\\','/'))=="SpEffectVfxParam");
        var def=PARAMDEF.XmlDeserialize("work/SoulsRandomizers/diste/Defs/SpEffect.xml");
        var vfxDef=PARAMDEF.XmlDeserialize("work/SoulsRandomizers/diste/Defs/SpEffectVfx.xml");
        var vfx=PARAM.Read(vfxFile.Bytes);vfx.ApplyParamdef(vfxDef);
        var original=PARAM.Read(file.Bytes);original.ApplyParamdef(def);
        var effects=PARAM.Read(file.Bytes);effects.ApplyParamdef(def);
        // 本段只放角色光环：已打过补丁时原地更新，不再使用的行删除
        var ids=Rows.Select(r=>r.id).ToHashSet();
        var existing=effects.Rows.Count(r=>r.ID>=First&&r.ID<First+100);
        var removed=effects.Rows.RemoveAll(r=>r.ID>=First&&r.ID<First+100&&!ids.Contains(r.ID));
        var (added,replaced)=(0,0);
        foreach(var spec in Rows) {
            var row=Create(effects,spec);int index=effects.Rows.FindIndex(r=>r.ID==spec.id);
            if(index<0){effects.Rows.Add(row);added++;}
            else if(!row.Cells.All(c=>System.Collections.StructuralComparisons.StructuralEqualityComparer.Equals(c.Value,effects.Rows[index][c.Def.InternalName].Value))){effects.Rows[index]=row;replaced++;}
        }
        effects.Rows.Sort((a,b)=>a.ID.CompareTo(b.ID));file.Bytes=effects.Write();
        Console.WriteLine($"Aura effects: {added} added, {replaced} replaced, {removed} removed");
        var others=reg.Files.Where(f=>f!=file).ToDictionary(f=>f.Name,f=>f.Bytes.ToArray());
        Directory.CreateDirectory(output);var path=Path.Combine(output,"regulation.bin");
        SoulsFormats.Cryptography.RegulationDecryptor.EncryptERRegulation(path,reg);
        var check=SoulsFormats.Cryptography.RegulationDecryptor.DecryptERRegulation(path);
        foreach(var f in check.Files.Where(f=>f.Name!=file.Name))if(!f.Bytes.SequenceEqual(others[f.Name]))throw new Exception($"Unrelated param changed: {f.Name}");
        var actual=PARAM.Read(check.Files.Single(f=>f.Name==file.Name).Bytes);actual.ApplyParamdef(def);Verify(actual,vfx);
        if(actual.Rows.Count!=original.Rows.Count-existing+Rows.Length)throw new Exception("Unexpected effect count");
        var rows=actual.Rows.ToDictionary(r=>r.ID);
        foreach(var row in original.Rows.Where(r=>r.ID<First||r.ID>=First+100))foreach(var cell in row.Cells)if(!System.Collections.StructuralComparisons.StructuralEqualityComparer.Equals(cell.Value,rows[row.ID][cell.Def.InternalName].Value))throw new Exception($"Existing effect changed: {row.ID}");
        string Hash(string p)=>Convert.ToHexString(System.Security.Cryptography.SHA256.HashData(File.ReadAllBytes(p)));
        File.WriteAllText(Path.Combine(output,"aura-build.json"),System.Text.Json.JsonSerializer.Serialize(new{effects=Rows.Select(r=>new{r.id,r.vfx}),sourceRegulationSha256=Hash(Path.Combine(input,"regulation.bin")),regulationSha256=Hash(path)},new System.Text.Json.JsonSerializerOptions{WriteIndented=true}));
        Console.WriteLine($"Verified {Rows.Length} aura effects; all existing regulation data preserved");
    }
}
