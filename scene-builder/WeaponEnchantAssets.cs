using SoulsFormats;

// 解除武器附魔限制：原版里特殊武器（大部分 Boss 武器）与带魔力、火焰、雷电等属性的武器不能涂油、不能用附魔祷告（EquipParamWeapon.isEnhance=0）。
// 这里把近战武器的 isEnhance 都打开；弓弩、法杖、圣印记、盾、火把、香水瓶不动。
// enchant-patch：只改 regulation（Linux 上也能运行），其余参数逐字节校验不变；可重复运行。
static class WeaponEnchantAssets {
    /// 近战武器类别（wepType）：短剑、直剑、大剑、特大剑、曲剑、大曲剑、刀、双头剑、刺剑、重刺剑、斧、大斧、锤、大锤、连枷、矛、大矛、斧枪、镰刀、拳头、钩爪、鞭子、特大武器，
    /// 以及 DLC 的格斗、投掷刀、反手剑、轻型大剑、大太刀、兽爪
    public static readonly HashSet<int> Melee=[1,3,5,7,9,11,13,14,15,16,17,19,21,23,24,25,28,29,31,35,37,39,41,88,91,92,93,94,95];
    static int I(PARAM.Row row,string field)=>Convert.ToInt32(row[field].Value);
    static void Set(PARAM.Row row,string field,int value)=>row[field].Value=Convert.ChangeType(value,row[field].Value.GetType());
    static (BND4 reg,BinderFile file,PARAM weapons) Load(string input) {
        var reg=SoulsFormats.Cryptography.RegulationDecryptor.DecryptERRegulation(Path.Combine(input,"regulation.bin"));
        var file=reg.Files.Single(f=>Path.GetFileNameWithoutExtension(f.Name.Replace('\\','/'))=="EquipParamWeapon");
        var weapons=PARAM.Read(file.Bytes);weapons.ApplyParamdef(PARAMDEF.XmlDeserialize("work/SoulsRandomizers/diste/Defs/EquipParamWeapon.xml"));
        return (reg,file,weapons);
    }
    /// 要解锁的行：近战武器里不能附魔的
    static List<PARAM.Row> Plan(PARAM weapons)=>weapons.Rows.Where(r=>r.ID>0&&Melee.Contains(I(r,"wepType"))&&I(r,"isEnhance")==0).ToList();
    public static void Patch(string input,string output) {
        if(Path.GetFullPath(input)==Path.GetFullPath(output))throw new Exception("Separate staging output required");
        var (reg,file,weapons)=Load(input);
        var originalFiles=reg.Files.ToDictionary(f=>f.Name,f=>f.Bytes.ToArray());
        var original=PARAM.Read(file.Bytes);original.ApplyParamdef(weapons.AppliedParamdef);
        var plan=Plan(weapons);
        int melee=weapons.Rows.Count(r=>r.ID>0&&Melee.Contains(I(r,"wepType")));
        Console.WriteLine($"Weapon rows {weapons.Rows.Count}, melee {melee}, locked melee {plan.Count}");
        foreach(var size in weapons.Rows.Where(r=>Melee.Contains(I(r,"wepType"))).GroupBy(r=>(I(r,"isEnhance"),I(r,"enchantSfxSize"))).OrderBy(g=>g.Key))Console.WriteLine($"  isEnhance={size.Key.Item1} enchantSfxSize={size.Key.Item2}: {size.Count()}");
        foreach(var row in plan)Set(row,"isEnhance",1);
        if(plan.Count>0)file.Bytes=weapons.Write();
        // 校验：只有近战武器的 isEnhance 变了，其他参数文件一个字节都不变
        var rows=weapons.Rows.ToDictionary(r=>r.ID);
        foreach(var old in original.Rows)foreach(var cell in old.Cells) {
            bool allowed=Melee.Contains(I(old,"wepType"))&&cell.Def.InternalName=="isEnhance";
            if(!allowed&&!System.Collections.StructuralComparisons.StructuralEqualityComparer.Equals(cell.Value,rows[old.ID][cell.Def.InternalName].Value))throw new Exception($"Unrelated weapon field changed: {old.ID} {cell.Def.InternalName}");
        }
        Directory.CreateDirectory(output);var path=Path.Combine(output,"regulation.bin");
        SoulsFormats.Cryptography.RegulationDecryptor.EncryptERRegulation(path,reg);
        if(new FileInfo(path).Length>4799*1024)throw new Exception("Regulation size limit exceeded");
        var check=SoulsFormats.Cryptography.RegulationDecryptor.DecryptERRegulation(path);
        foreach(var f in check.Files.Where(f=>f.Name!=file.Name))if(!f.Bytes.SequenceEqual(originalFiles[f.Name]))throw new Exception($"Unrelated param changed: {f.Name}");
        var actual=PARAM.Read(check.Files.Single(f=>f.Name==file.Name).Bytes);actual.ApplyParamdef(weapons.AppliedParamdef);
        var locked=actual.Rows.Where(r=>r.ID>0&&Melee.Contains(I(r,"wepType"))&&I(r,"isEnhance")==0).Select(r=>r.ID).ToList();
        if(locked.Count>0)throw new Exception($"Melee weapons still locked: {string.Join(",",locked.Take(10))}");
        string Hash(string p)=>Convert.ToHexString(System.Security.Cryptography.SHA256.HashData(File.ReadAllBytes(p)));
        File.WriteAllText(Path.Combine(output,"weapon-enchant-build.json"),System.Text.Json.JsonSerializer.Serialize(new{unlocked=plan.Count,meleeTypes=Melee.Order(),sourceRegulationSha256=Hash(Path.Combine(input,"regulation.bin")),regulationSha256=Hash(path)},new System.Text.Json.JsonSerializerOptions{WriteIndented=true}));
        Console.WriteLine($"Verified: every melee weapon can be buffed; {plan.Count} rows changed, all other params unchanged");
    }
}
