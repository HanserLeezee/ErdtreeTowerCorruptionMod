using SoulsFormats;
// 斗蛐蛐实验：复制原版“结晶飞镖·混乱”（6910，阵营 29），持续到效果被移除，阵营分别取原版用过的几个值，
// 由调试菜单挂到刷出的敌人身上，实机确认哪个阵营能让两只敌人互相攻击。编号为 9680000+阵营值。
static class BoutAssets {
    public const int First=9680000;
    public static readonly int[] Teams=[24,26,27,29,30,34];
    static void Set(PARAM.Row r,string k,object v)=>r[k].Value=Convert.ChangeType(v,r[k].Value.GetType());
    static PARAM.Row Create(PARAM p,int team) {
        var r=new PARAM.Row(p.Rows.Single(x=>x.ID==6910)){ID=First+team,Name=$"Tower bout team {team}"};
        Set(r,"changeTeamType",team);Set(r,"effectEndurance",-1f);Set(r,"saveCategory",-1);
        return r;
    }
    public static void Patch(string input,string output) {
        if(Path.GetFullPath(input)==Path.GetFullPath(output))throw new Exception("Separate staging output required");
        var reg=SoulsFormats.Cryptography.RegulationDecryptor.DecryptERRegulation(Path.Combine(input,"regulation.bin"));
        var file=reg.Files.Single(f=>Path.GetFileNameWithoutExtension(f.Name.Replace('\\','/'))=="SpEffectParam");
        var def=PARAMDEF.XmlDeserialize("work/SoulsRandomizers/diste/Defs/SpEffect.xml");
        var original=PARAM.Read(file.Bytes);original.ApplyParamdef(def);
        var effects=PARAM.Read(file.Bytes);effects.ApplyParamdef(def);
        foreach(var team in Teams) {
            var expected=Create(effects,team);int index=effects.Rows.FindIndex(r=>r.ID==expected.ID);
            if(index>=0)effects.Rows[index]=expected;else effects.Rows.Add(expected);
        }
        if(effects.Rows.Any(r=>r.ID>=First&&r.ID<First+100&&!Teams.Contains(r.ID-First)))throw new Exception("Bout effect range 9680000-9680099 already in use");
        effects.Rows.Sort((a,b)=>a.ID.CompareTo(b.ID));file.Bytes=effects.Write();
        var others=reg.Files.Where(f=>f!=file).ToDictionary(f=>f.Name,f=>f.Bytes.ToArray());
        Directory.CreateDirectory(output);var path=Path.Combine(output,"regulation.bin");
        SoulsFormats.Cryptography.RegulationDecryptor.EncryptERRegulation(path,reg);
        var check=SoulsFormats.Cryptography.RegulationDecryptor.DecryptERRegulation(path);
        foreach(var f in check.Files.Where(f=>f.Name!=file.Name))if(!f.Bytes.SequenceEqual(others[f.Name]))throw new Exception($"Unrelated param changed: {f.Name}");
        var actual=PARAM.Read(check.Files.Single(f=>f.Name==file.Name).Bytes);actual.ApplyParamdef(def);var rows=actual.Rows.ToDictionary(r=>r.ID);
        foreach(var team in Teams) {var expected=Create(actual,team);if(!expected.Cells.All(c=>System.Collections.StructuralComparisons.StructuralEqualityComparer.Equals(c.Value,rows[expected.ID][c.Def.InternalName].Value)))throw new Exception($"Bout effect mismatch {expected.ID}");}
        foreach(var row in original.Rows.Where(r=>r.ID<First||r.ID>=First+100))foreach(var cell in row.Cells)if(!System.Collections.StructuralComparisons.StructuralEqualityComparer.Equals(cell.Value,rows[row.ID][cell.Def.InternalName].Value))throw new Exception($"Existing effect changed: {row.ID}");
        Console.WriteLine($"Verified {Teams.Length} bout team effects; all existing regulation data preserved");
    }
}
