using SoulsFormats;

// 解除武器强化上限：原版强化参数（ReinforceParamWeapon）每种强化类型只有 +0～+25（失色 +0～+10，少数特殊类型 +0～+5）。
// 这里给每种类型往后补等级行，按最后一级的增量线性外推（攻击、补正等倍率；防御、抗性停在原版满级），最高到 floor(原满级×99/25)：
// 普通 +25 → +99（武器编号的个位两位能表示的最高等级），失色 +10 → +39，+5 的类型 → +19，换算后与普通 +99 同一水平。
// 素材行（materialSetId）沿用“等级号”的规律，模组的强化石素材组 700000–700199 都是“强化石 ×1”；
// maxReinforceLevel（换算成普通武器的等效等级，失色每级 2.5）同样按比例外推。原有行一个字节都不改，可重复运行。
// weapon-level-patch：只改 regulation（Linux 上也能运行）。
static class WeaponLevelAssets {
    public const int NormalMax=99;
    static int I(PARAM.Row row,string field)=>Convert.ToInt32(row[field].Value);
    /// 原版满级 → 新的满级
    public static int NewMax(int vanilla)=>vanilla*NormalMax/25;
    static List<PARAM.Row> Build(PARAM reinforce) {
        var rows=reinforce.Rows.ToDictionary(r=>r.ID);var added=new List<PARAM.Row>();
        foreach(var group in reinforce.Rows.Where(r=>r.ID>=0).GroupBy(r=>r.ID/100*100).OrderBy(g=>g.Key)) {
            int type=group.Key;
            // 原版等级：从 +0 起连续的那一段（之前打过补丁的话，新增的行不算原版）
            int vanilla=0;while(rows.ContainsKey(type+vanilla+1)&&rows[type+vanilla+1].Name!="Tower weapon level")vanilla++;
            if(vanilla<2)continue;
            var last=rows[type+vanilla];var previous=rows[type+vanilla-1];
            for(int level=vanilla+1;level<=NewMax(vanilla);level++) {
                if(rows.TryGetValue(type+level,out var existing)&&existing.Name!="Tower weapon level")throw new Exception($"Reinforce row collision: {type+level}");
                var row=new PARAM.Row(last){ID=type+level,Name="Tower weapon level"};
                int steps=level-vanilla;
                foreach(var cell in row.Cells) {
                    string key=cell.Def.InternalName;
                    // 防御、抗性倍率停在原版满级（盾牌的减伤不能超过 100%）；攻击、补正、韧性削减等继续增长
                    if(cell.Value is float f&&!key.Contains("Guard")&&!key.Contains("Price")) {
                        float delta=Convert.ToSingle(last[key].Value)-Convert.ToSingle(previous[key].Value);
                        cell.Value=MathF.Round(f+delta*steps,4);
                    }
                }
                row["materialSetId"].Value=Convert.ChangeType(I(last,"materialSetId")+steps,row["materialSetId"].Value.GetType());
                row["maxReinforceLevel"].Value=Convert.ChangeType(Math.Min(255,I(last,"maxReinforceLevel")*level/vanilla),row["maxReinforceLevel"].Value.GetType());
                added.Add(row);
            }
        }
        return added;
    }
    public static void Patch(string input,string output) {
        if(Path.GetFullPath(input)==Path.GetFullPath(output))throw new Exception("Separate staging output required");
        var reg=SoulsFormats.Cryptography.RegulationDecryptor.DecryptERRegulation(Path.Combine(input,"regulation.bin"));
        BinderFile File_(string name)=>reg.Files.Single(f=>Path.GetFileNameWithoutExtension(f.Name.Replace('\\','/'))==name);
        PARAM Read(string name){var p=PARAM.Read(File_(name).Bytes);p.ApplyParamdef(PARAMDEF.XmlDeserialize($"work/SoulsRandomizers/diste/Defs/{name}.xml"));return p;}
        var reinforce=Read("ReinforceParamWeapon");var materials=Read("EquipMtrlSetParam");
        var originalFiles=reg.Files.ToDictionary(f=>f.Name,f=>f.Bytes.ToArray());
        var original=Read("ReinforceParamWeapon");
        var expected=Build(reinforce);
        var (added,replaced)=(0,0);
        foreach(var row in expected) {
            int index=reinforce.Rows.FindIndex(r=>r.ID==row.ID);
            if(index<0){reinforce.Rows.Add(row);added++;}
            else if(!row.Cells.All(c=>System.Collections.StructuralComparisons.StructuralEqualityComparer.Equals(c.Value,reinforce.Rows[index][c.Def.InternalName].Value))){reinforce.Rows[index]=row;replaced++;}
        }
        reinforce.Rows.Sort((a,b)=>a.ID.CompareTo(b.ID));
        if(added+replaced>0)File_("ReinforceParamWeapon").Bytes=reinforce.Write();
        // 强化石素材组必须覆盖到新的最高等级
        int topMaterial=expected.Count==0?0:expected.Max(r=>I(r,"materialSetId"));
        foreach(int set in Enumerable.Range(700000,topMaterial+1))
            if(!materials.Rows.Any(r=>r.ID==set&&I(r,"materialId01")==7500100&&I(r,"itemNum01")==1))throw new Exception($"Missing upgrade stone material set {set}");
        Directory.CreateDirectory(output);var path=Path.Combine(output,"regulation.bin");
        SoulsFormats.Cryptography.RegulationDecryptor.EncryptERRegulation(path,reg);
        if(new FileInfo(path).Length>4799*1024)throw new Exception("Regulation size limit exceeded");
        var check=SoulsFormats.Cryptography.RegulationDecryptor.DecryptERRegulation(path);
        var name=File_("ReinforceParamWeapon").Name;
        foreach(var f in check.Files.Where(f=>f.Name!=name))if(!f.Bytes.SequenceEqual(originalFiles[f.Name]))throw new Exception($"Unrelated param changed: {f.Name}");
        var actual=PARAM.Read(check.Files.Single(f=>f.Name==name).Bytes);actual.ApplyParamdef(reinforce.AppliedParamdef);var byId=actual.Rows.ToDictionary(r=>r.ID);
        foreach(var row in original.Rows.Where(r=>r.Name!="Tower weapon level"))foreach(var cell in row.Cells)
            if(!System.Collections.StructuralComparisons.StructuralEqualityComparer.Equals(cell.Value,byId[row.ID][cell.Def.InternalName].Value))throw new Exception($"Existing reinforce row changed: {row.ID}");
        foreach(var row in expected)foreach(var cell in row.Cells)
            if(!System.Collections.StructuralComparisons.StructuralEqualityComparer.Equals(cell.Value,byId[row.ID][cell.Def.InternalName].Value))throw new Exception($"Weapon level row mismatch: {row.ID}");
        // 每种类型的等级连续、倍率不下降
        foreach(var group in actual.Rows.Where(r=>r.ID>=0).GroupBy(r=>r.ID/100*100)) {
            var ids=group.Select(r=>r.ID%100).OrderBy(x=>x).ToList();
            if(!ids.SequenceEqual(Enumerable.Range(0,ids.Count)))throw new Exception($"Noncontiguous reinforce levels: {group.Key}");
        }
        foreach(var row in expected)if(Convert.ToSingle(row["physicsAtkRate"].Value)<Convert.ToSingle(byId[row.ID-1]["physicsAtkRate"].Value))throw new Exception($"Attack rate decreased: {row.ID}");
        var summary=expected.GroupBy(r=>r.ID/100*100).Select(g=>$"{g.Key}:+{g.Max(r=>r.ID%100)}");
        string Hash(string p)=>Convert.ToHexString(System.Security.Cryptography.SHA256.HashData(File.ReadAllBytes(p)));
        File.WriteAllText(Path.Combine(output,"weapon-level-build.json"),System.Text.Json.JsonSerializer.Serialize(new{rows=expected.Count,maxLevels=expected.GroupBy(r=>r.ID/100*100).ToDictionary(g=>g.Key.ToString(),g=>g.Max(r=>r.ID%100)),sourceRegulationSha256=Hash(Path.Combine(input,"regulation.bin")),regulationSha256=Hash(path)},new System.Text.Json.JsonSerializerOptions{WriteIndented=true}));
        Console.WriteLine($"Weapon levels: {added} added, {replaced} replaced ({expected.Count} total); {string.Join(" ",summary)}");
        var sample=new[]{25,26,50,99}.Select(l=>byId.TryGetValue(l,out var r)?$"+{l} phys {r["physicsAtkRate"].Value} str {r["correctStrengthRate"].Value} max {r["maxReinforceLevel"].Value} mat {r["materialSetId"].Value}":"");
        Console.WriteLine("Standard type: "+string.Join(" | ",sample));
        var somber=new[]{10,11,25,39}.Select(l=>byId.TryGetValue(2200+l,out var r)?$"+{l} phys {r["physicsAtkRate"].Value} max {r["maxReinforceLevel"].Value}":"");
        Console.WriteLine("Somber type 2200: "+string.Join(" | ",somber));
        Console.WriteLine("Verified: original reinforce rows and all other params unchanged");
    }
}
