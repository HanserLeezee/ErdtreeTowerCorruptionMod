using SoulsFormats;
// 实验：把爬塔弱怪与精英所用 AI 的「团队攻击影响力」调高，让同时在场的敌人轮流进攻（数值越大，同时进攻的人越少）。
// Boss 用到的 AI 行不改，组合 Boss 保持原版同时进攻的设计。原始值写入 team-attack-build.json 以便还原。
static class TeamAttackAssets {
    public const int Value=100;
    public static void Patch(string input,string output) {
        if(Path.GetFullPath(input)==Path.GetFullPath(output))throw new Exception("Separate staging output required");
        var reg=SoulsFormats.Cryptography.RegulationDecryptor.DecryptERRegulation(Path.Combine(input,"regulation.bin"));
        var file=reg.Files.Single(f=>Path.GetFileNameWithoutExtension(f.Name.Replace('\\','/'))=="NpcThinkParam");
        var def=PARAMDEF.XmlDeserialize("work/SoulsRandomizers/diste/Defs/NpcThinkParam.xml");
        var before=PARAM.Read(file.Bytes);before.ApplyParamdef(def);var think=PARAM.Read(file.Bytes);think.ApplyParamdef(def);
        var bossThinks=BossAssets.Parts(input).Select(p=>p.Think).ToHashSet();
        var targets=File.ReadAllLines(Path.Combine(input,"enemy-pools.txt")).Select(l=>l.Split('|')).Where(p=>p[1]!="boss").Select(p=>int.Parse(p[2])*10000).Where(id=>!bossThinks.Contains(id)).Distinct().Order().ToList();
        var original=new Dictionary<int,int>();
        foreach(var id in targets) {
            var row=think.Rows.SingleOrDefault(r=>r.ID==id)??throw new Exception($"Missing NpcThinkParam {id}");
            original[id]=Convert.ToInt32(row["TeamAttackEffectivity"].Value);row["TeamAttackEffectivity"].Value=Convert.ChangeType(Value,row["TeamAttackEffectivity"].Value.GetType());
        }
        file.Bytes=think.Write();var others=reg.Files.Where(f=>f!=file).ToDictionary(f=>f.Name,f=>f.Bytes.ToArray());
        Directory.CreateDirectory(output);var path=Path.Combine(output,"regulation.bin");
        SoulsFormats.Cryptography.RegulationDecryptor.EncryptERRegulation(path,reg);
        var check=SoulsFormats.Cryptography.RegulationDecryptor.DecryptERRegulation(path);
        foreach(var f in check.Files.Where(f=>f.Name!=file.Name))if(!f.Bytes.SequenceEqual(others[f.Name]))throw new Exception($"Unrelated param changed: {f.Name}");
        var after=PARAM.Read(check.Files.Single(f=>f.Name==file.Name).Bytes);after.ApplyParamdef(def);var rows=after.Rows.ToDictionary(r=>r.ID);
        foreach(var row in before.Rows)foreach(var cell in row.Cells) {
            bool ours=targets.Contains(row.ID)&&cell.Def.InternalName=="TeamAttackEffectivity";
            var expected=ours?Convert.ChangeType(Value,cell.Value.GetType()):cell.Value;
            if(!System.Collections.StructuralComparisons.StructuralEqualityComparer.Equals(expected,rows[row.ID][cell.Def.InternalName].Value))throw new Exception($"Unexpected AI change: {row.ID} {cell.Def.InternalName}");
        }
        string Hash(string p)=>Convert.ToHexString(System.Security.Cryptography.SHA256.HashData(File.ReadAllBytes(p)));
        var previous=Path.Combine(input,"team-attack-build.json");
        // 已记录的原始值优先（重复运行时 AI 行已经是 100）；新增的 AI 行补上本次读到的原始值
        var kept=File.Exists(previous)?System.Text.Json.JsonSerializer.Deserialize<Dictionary<int,int>>(System.Text.Json.Nodes.JsonNode.Parse(File.ReadAllText(previous))!["original"])!:new Dictionary<int,int>();
        foreach(var (id,value) in original)kept.TryAdd(id,value);
        File.WriteAllText(Path.Combine(output,"team-attack-build.json"),System.Text.Json.JsonSerializer.Serialize(new{value=Value,thinks=targets.Count,original=kept,sourceRegulationSha256=Hash(Path.Combine(input,"regulation.bin")),regulationSha256=Hash(path)},new System.Text.Json.JsonSerializerOptions{WriteIndented=true}));
        Console.WriteLine($"Set TeamAttackEffectivity={Value} on {targets.Count} weak/elite AI rows ({original.Values.Count(v=>v!=Value)} changed); boss AI and all other params preserved");
    }
}
