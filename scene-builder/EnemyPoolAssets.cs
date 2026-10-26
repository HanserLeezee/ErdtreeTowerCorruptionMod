using SoulsFormats;
// 怪物池参数：为 enemy-pools.txt 中尚无参数行的敌人补上 900000000+模型号×10000，
// 做法与 NativeAssets（复制原版基础行，去掉掉落，按池内卢恩给魂）及 BalanceAssets（生命、卢恩倍率）相同。已有行不动，可重复运行。
static class EnemyPoolAssets {
    static void Set(PARAM.Row r,string k,object v)=>r[k].Value=Convert.ChangeType(v,r[k].Value.GetType());
    public static void Patch(string input,string output) {
        if(Path.GetFullPath(input)==Path.GetFullPath(output))throw new Exception("Separate staging output required");
        var reg=SoulsFormats.Cryptography.RegulationDecryptor.DecryptERRegulation(Path.Combine(input,"regulation.bin"));
        BinderFile File_(string name)=>reg.Files.Single(f=>Path.GetFileNameWithoutExtension(f.Name.Replace('\\','/'))==name);
        PARAM Read(string name) {var p=PARAM.Read(File_(name).Bytes);p.ApplyParamdef(PARAMDEF.XmlDeserialize($"work/SoulsRandomizers/diste/Defs/{name}.xml"));return p;}
        var npcs=Read("NpcParam");var think=Read("NpcThinkParam");var before=npcs.Rows.Count;
        var added=new List<int>();
        foreach(var p in File.ReadAllLines(Path.Combine(input,"enemy-pools.txt")).Select(l=>l.Split('|'))) {
            int model=int.Parse(p[2]),id=900000000+model*10000,runes=int.Parse(p[3]);
            if(!think.Rows.Any(r=>r.ID==model*10000))throw new Exception($"Missing enemy AI: {model}");
            var source=npcs.Rows.SingleOrDefault(r=>r.ID==model*10000)??throw new Exception($"Missing enemy base parameters: {model}");
            if(npcs.Rows.Any(r=>r.ID==id))continue;
            var row=new PARAM.Row(source){ID=id,Name="Tower rune-only enemy"};
            Set(row,"itemLotId_enemy",-1);Set(row,"itemLotId_map",-1);Set(row,"isSoulGetByBoss",0);
            Set(row,"hp",BalanceAssets.ScaleHp(Convert.ToInt64(source["hp"].Value)));Set(row,"getSoul",BalanceAssets.Scale(runes));
            npcs.Rows.Add(row);added.Add(id);
        }
        npcs.Rows.Sort((a,b)=>a.ID.CompareTo(b.ID));File_("NpcParam").Bytes=npcs.Write();
        var others=reg.Files.Where(f=>Path.GetFileNameWithoutExtension(f.Name.Replace('\\','/'))!="NpcParam").ToDictionary(f=>f.Name,f=>f.Bytes.ToArray());
        Directory.CreateDirectory(output);var path=Path.Combine(output,"regulation.bin");
        SoulsFormats.Cryptography.RegulationDecryptor.EncryptERRegulation(path,reg);
        if(new FileInfo(path).Length>4799*1024)throw new Exception("Regulation exceeds the game's size limit");
        var check=SoulsFormats.Cryptography.RegulationDecryptor.DecryptERRegulation(path);
        foreach(var f in check.Files.Where(f=>others.ContainsKey(f.Name)))if(!f.Bytes.SequenceEqual(others[f.Name]))throw new Exception($"Unrelated param changed: {f.Name}");
        var after=PARAM.Read(check.Files.Single(f=>!others.ContainsKey(f.Name)).Bytes);after.ApplyParamdef(npcs.AppliedParamdef);
        if(after.Rows.Count!=before+added.Count)throw new Exception("Unexpected NpcParam row count");
        Console.WriteLine($"Added {added.Count} tower enemy parameter rows ({string.Join(",",added)}); existing rows and other params unchanged");
    }
}
