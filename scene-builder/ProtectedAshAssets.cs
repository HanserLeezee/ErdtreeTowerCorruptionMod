using SoulsFormats;

static class ProtectedAshAssets {
    static readonly Dictionary<int,string> Types=new(){[50]="BowSmall",[51]="BowNormal",[53]="BowLarge",[55]="ClossBow",[56]="Ballista",[65]="ShieldSmall",[67]="ShieldNormal",[69]="ShieldLarge",[90]="ThrustingShield"};
    static int I(PARAM.Row row,string field)=>Convert.ToInt32(row[field].Value);
    public static void Build(string game,string input,string output) {
        var vanilla=SoulsFormats.Cryptography.RegulationDecryptor.DecryptERRegulation(Path.Combine(game,"regulation.bin"));
        var current=SoulsFormats.Cryptography.RegulationDecryptor.DecryptERRegulation(Path.Combine(input,"regulation.bin"));
        PARAM Read(BND4 reg,string name) {var p=PARAM.Read(reg.Files.Single(f=>Path.GetFileNameWithoutExtension(f.Name.Replace('\\','/'))==name).Bytes);p.ApplyParamdef(PARAMDEF.XmlDeserialize($"work/SoulsRandomizers/diste/Defs/{name}.xml"));return p;}
        var originals=Read(vanilla,"EquipParamWeapon").Rows.ToDictionary(r=>r.ID);
        var gems=Read(vanilla,"EquipParamGem").Rows.ToDictionary(r=>r.ID);
        var pool=File.ReadAllLines(Path.Combine(input,"unrestricted-ashes.txt")).Select(int.Parse).ToArray();
        var bases=new[]{"affix-catalog.txt","affix-v2-catalog.txt"}.SelectMany(f=>File.ReadAllLines(Path.Combine(input,f))).Select(l=>l.Split('|')).Where(p=>p[0]=="weapon").ToDictionary(p=>int.Parse(p[1]),p=>int.Parse(p[2]));
        var lines=new List<string>();int empty=0;
        foreach(var row in Read(current,"EquipParamWeapon").Rows.OrderBy(r=>r.ID)) {
            int type=I(row,"wepType");if(!Types.TryGetValue(type,out var field))continue;
            int baseId=bases.GetValueOrDefault(row.ID/10000*10000,row.ID);
            if(!originals.TryGetValue(baseId,out var basis))throw new Exception($"No vanilla weapon for protected row {row.ID}, base {baseId}");
            if(I(basis,"wepType")!=type)throw new Exception("Protected weapon type mismatch");
            var allowed=pool.Where(id=>I(basis,"gemMountType")==2&&gems.TryGetValue(id,out var gem)&&I(gem,"canMountWep_"+field)!=0&&(I(basis,"restrictSpecialSwordArt")==0||I(gem,"isSpecialSwordArt")==0)).ToArray();
            if(allowed.Length==0)empty++;
            lines.Add($"{row.ID}\t{type}\t{string.Join(',',allowed)}");
        }
        if(lines.Count==0||empty==0)throw new Exception("Protected weapon coverage incomplete");
        Directory.CreateDirectory(output);File.WriteAllLines(Path.Combine(output,"protected-ash-pools.tsv"),lines);
        Console.WriteLine($"Verified {lines.Count} shield/bow/crossbow rows against vanilla compatibility; {empty} rows retain their default skill");
    }
}
