using SoulsFormats;
// 传说武器流光：握在手里的传说品质词条武器，刀身挂上原版的金色附魔光效（只有视觉，不加任何数值）。
// 效果放在 9830000 段（编号 = 9830000 + 光效×10 + 手，0 右手、1 左手），复制中性的 90400 后只改持续时间、类别与光效，
// 光效直接引用原版涂油 / 战技的附魔光效行（刀身特效、刀身染色与挥砍轨迹颜色一起生效）。由原生 DLL 按手上的武器挂上并轮换。
static class WeaponGlowAssets {
    public const int First=9830000;
    const int Neutral=90400;
    // (编号, 名字, 原版光效行)：黄金淬炼 8760/8761、神圣涂油 61/5140、雷电涂油 151/5105
    static (int id,string name,int vfx)[] Rows=>[
        (First,"Tower legendary glow tempering right",8760),
        (First+1,"Tower legendary glow tempering left",8761),
        (First+10,"Tower legendary glow holy right",61),
        (First+11,"Tower legendary glow holy left",5140),
        (First+20,"Tower legendary glow lightning right",151),
        (First+21,"Tower legendary glow lightning left",5105),
    ];
    static void Set(PARAM.Row r,string key,object value)=>r[key].Value=Convert.ChangeType(value,r[key].Value.GetType());
    static PARAM.Row Create(PARAM effects,(int id,string name,int vfx) spec) {
        var row=new PARAM.Row(effects.Rows.Single(r=>r.ID==Neutral)){ID=spec.id,Name=spec.name};
        // 不占涂油所在的 162 / 163 类别，不会顶掉玩家自己涂的油；不存档、无图标，持续到原生 DLL 撤下
        foreach(var (key,value) in new (string,object)[]{("effectEndurance",-1f),("spCategory",0),("categoryPriority",0),("saveCategory",-1),("iconId",-1),("dontDeleteOnDead",0),("vfxId",spec.vfx)})Set(row,key,value);
        return row;
    }
    static void Verify(PARAM effects,PARAM vfx) {
        foreach(var spec in Rows) {
            var row=effects.Rows.Single(r=>r.ID==spec.id);var expected=Create(effects,spec);
            foreach(var cell in expected.Cells)if(!System.Collections.StructuralComparisons.StructuralEqualityComparer.Equals(cell.Value,row[cell.Def.InternalName].Value))throw new Exception($"Weapon glow effect mismatch: {spec.id} {cell.Def.InternalName}");
            var look=vfx.Rows.Single(r=>r.ID==spec.vfx);
            // 光效必须是对应那只手的刀身附魔（effectType 1 右手 / 2 左手，从刀身挂点起播）
            if(Convert.ToInt32(look["effectType"].Value)!=1+spec.id%2||Convert.ToInt32(look["enchantStartDmyId_0"].Value)<0)throw new Exception($"Weapon glow must use a weapon enchant visual for its hand: {spec.id}");
            foreach(var f in new[]{"spCategory","iconId"})if(Convert.ToInt32(row[f].Value)!=(f=="iconId"?-1:0))throw new Exception($"Weapon glow must stay cosmetic: {spec.id} {f}");
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
        // 本段只放武器流光：已打过补丁时原地更新，不再使用的行删除
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
        Console.WriteLine($"Weapon glow effects: {added} added, {replaced} replaced, {removed} removed");
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
        File.WriteAllText(Path.Combine(output,"weapon-glow-build.json"),System.Text.Json.JsonSerializer.Serialize(new{effects=Rows.Select(r=>new{r.id,r.vfx}),sourceRegulationSha256=Hash(Path.Combine(input,"regulation.bin")),regulationSha256=Hash(path)},new System.Text.Json.JsonSerializerOptions{WriteIndented=true}));
        Console.WriteLine($"Verified {Rows.Length} weapon glow effects; all existing regulation data preserved");
    }
}
