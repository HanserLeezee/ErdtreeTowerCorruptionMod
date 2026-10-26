using SoulsFormats;
// 装备词条池重做：原来的属性点词条（7–11）原地换成新效果，并把词条池从 12 种扩到 AffixAssets.Kinds 种。
// 用与完整重建相同的种子给每件词条装备重新抽词条，只改常驻效果槽；同步更新奖励详情，
// 并生成 affix-effect-remap.tsv（旧词条行 → 新词条行），由 Windows 上的 text-update 改写游戏内文本。
static class AffixPoolPatch {
    const int EffectFirst=AffixAssets.EffectFirst;
    static int I(PARAM.Row r,string k)=>Convert.ToInt32(r[k].Value);
    static readonly string[] WeaponSlots=["residentSpEffectId","residentSpEffectId1","residentSpEffectId2"];
    static readonly string[] ArmorSlots=["residentSpEffectId","residentSpEffectId2","residentSpEffectId3"];
    static bool Affix(int id)=>id>=EffectFirst&&id<EffectFirst+AffixAssets.Kinds*10;
    record Variant(bool Weapon,int Id,int Base,int Tier,int? Profile);
    static IEnumerable<Variant> Variants(string input)=>
        File.ReadAllLines(Path.Combine(input,"affix-catalog.txt")).Select(l=>l.Split('|')).Select(p=>new Variant(p[0]=="weapon",int.Parse(p[1]),int.Parse(p[2]),int.Parse(p[3]),null))
        .Concat(File.ReadAllLines(Path.Combine(input,"affix-v2-catalog.txt")).Select(l=>l.Split('|')).Select(p=>new Variant(p[0]=="weapon",int.Parse(p[1]),int.Parse(p[2]),int.Parse(p[3]),int.Parse(p[5]))));
    // 与 AffixAssets / AdvancedAffixAssets 的抽取顺序一致，只是候选数量不同
    static int[] Choices(Variant v,int kinds,int count) {
        var mix=AffixAssets.Mix;
        if(v.Profile is int profile) {
            ulong seed=mix((ulong)v.Base^((ulong)v.Tier<<32)^((ulong)profile<<40)^0xaff22026UL);
            return Enumerable.Range(0,kinds).OrderBy(i=>mix(seed^(ulong)(100+i))).Take(count).ToArray();
        } else {
            ulong seed=mix((ulong)v.Base^((ulong)v.Tier<<32)^0xa771f1UL);
            return Enumerable.Range(0,kinds).OrderBy(i=>mix(seed^(ulong)i)).Take(count).ToArray();
        }
    }
    static string Block(int[] kinds,int tier,Func<int,int,string> text,string separator)=>string.Join(separator,kinds.Select(k=>text(k,tier)));
    static bool Same(PARAM.Row a,PARAM.Row b)=>a.Cells.All(c=>System.Collections.StructuralComparisons.StructuralEqualityComparer.Equals(c.Value,b[c.Def.InternalName].Value));
    public static void Patch(string input,string output) {
        if(Path.GetFullPath(input)==Path.GetFullPath(output))throw new Exception("Separate staging output required");
        var reg=SoulsFormats.Cryptography.RegulationDecryptor.DecryptERRegulation(Path.Combine(input,"regulation.bin"));
        var before=reg.Files.ToDictionary(f=>f.Name,f=>f.Bytes.ToArray());
        BinderFile File_(string name)=>reg.Files.Single(f=>Path.GetFileNameWithoutExtension(f.Name.Replace('\\','/'))==name);
        PARAM Read(string name,string def) {var p=PARAM.Read(File_(name).Bytes);p.ApplyParamdef(PARAMDEF.XmlDeserialize($"work/SoulsRandomizers/diste/Defs/{def}.xml"));return p;}
        var effects=Read("SpEffectParam","SpEffect");var weapons=Read("EquipParamWeapon","EquipParamWeapon");var armor=Read("EquipParamProtector","EquipParamProtector");
        // 效果行：0–6 原样（顺便校验定义与已有行一致），7–11 原地替换，12 起新增
        for(int kind=0;kind<AffixAssets.Kinds;kind++)for(int tier=0;tier<4;tier++) {
            var expected=AffixAssets.CreateEffect(effects,kind,tier);
            int index=effects.Rows.FindIndex(r=>r.ID==expected.ID);
            if(kind<7) {if(index<0||!Same(expected,effects.Rows[index]))throw new Exception($"Existing affix effect differs from its definition: {expected.ID}");continue;}
            if(index>=0)effects.Rows[index]=expected;else effects.Rows.Add(expected);
        }
        effects.Rows.Sort((a,b)=>a.ID.CompareTo(b.ID));
        // 装备：按变体重新抽词条，替换常驻效果槽
        var remap=new List<string>();var tsv=new Dictionary<uint,(string old,string @new)>();var audit=new Dictionary<uint,string>();
        int rerolled=0,rows=0,already=0;
        foreach(var v in Variants(input)) {
            var param=v.Weapon?weapons:armor;var slots=v.Weapon?WeaponSlots:ArmorSlots;
            var family=param.Rows.Where(r=>r.ID>=v.Id&&r.ID<v.Id+(v.Weapon?10000:1)).ToList();
            var head=family.SingleOrDefault(r=>r.ID==v.Id)??throw new Exception($"Missing affix variant {v.Id}");
            int count=slots.Count(k=>Affix(I(head,k)));if(count==0)continue;
            var legacy=Choices(v,AffixAssets.LegacyKinds,count);var fresh=Choices(v,AffixAssets.Kinds,count);
            int[] Ids(int[] kinds)=>kinds.Select(k=>EffectFirst+k*10+v.Tier).ToArray();
            var current=slots.Select(k=>I(head,k)).Where(Affix).ToArray();
            if(current.SequenceEqual(Ids(fresh))) already++;
            else if(!current.SequenceEqual(Ids(legacy))) throw new Exception($"Affix slots of {v.Id} match neither the old nor the new pool");
            foreach(var row in family) {
                int j=0;
                foreach(var slot in slots)if(Affix(I(row,slot))){row[slot].Value=Convert.ChangeType(Ids(fresh)[j++],row[slot].Value.GetType());}
                if(j!=count)throw new Exception($"Affix slot count differs inside family {v.Id}:{row.ID}");
                rows++;
            }
            rerolled++;
            uint tag=v.Weapon?0u:0x10000000u;
            string oldText=Block(legacy,v.Tier,AffixAssets.LegacyText,"\\n"),newText=Block(fresh,v.Tier,AffixAssets.DescribeEffect,"\\n");
            tsv[tag|(uint)v.Id]=(oldText,newText);
            if(v.Profile!=null)audit[tag|(uint)v.Id]=string.Join(',',Ids(fresh));
            if(oldText!=newText)remap.Add($"{v.Id}\t{oldText}\t{newText}");
        }
        File_("SpEffectParam").Bytes=effects.Write();File_("EquipParamWeapon").Bytes=weapons.Write();File_("EquipParamProtector").Bytes=armor.Write();
        Directory.CreateDirectory(output);var path=Path.Combine(output,"regulation.bin");
        SoulsFormats.Cryptography.RegulationDecryptor.EncryptERRegulation(path,reg);
        if(new FileInfo(path).Length>4799*1024)throw new Exception("Regulation exceeds the game's size limit");
        // 校验：只改了词条效果行与装备的常驻效果槽
        var check=SoulsFormats.Cryptography.RegulationDecryptor.DecryptERRegulation(path);
        var changed=new[]{"SpEffectParam","EquipParamWeapon","EquipParamProtector"};
        foreach(var f in check.Files.Where(f=>!changed.Contains(Path.GetFileNameWithoutExtension(f.Name.Replace('\\','/')))))if(!f.Bytes.SequenceEqual(before[f.Name]))throw new Exception($"Unrelated param changed: {f.Name}");
        foreach(var (name,def,allowed) in new[]{("SpEffectParam","SpEffect",(Func<PARAM.Row,string,bool>)((r,_)=>Affix(r.ID))),("EquipParamWeapon","EquipParamWeapon",(r,k)=>WeaponSlots.Contains(k)),("EquipParamProtector","EquipParamProtector",(r,k)=>ArmorSlots.Contains(k))}) {
            var file=check.Files.Single(f=>Path.GetFileNameWithoutExtension(f.Name.Replace('\\','/'))==name);
            var now=PARAM.Read(file.Bytes);now.ApplyParamdef(PARAMDEF.XmlDeserialize($"work/SoulsRandomizers/diste/Defs/{def}.xml"));
            var old=PARAM.Read(before[file.Name]);old.ApplyParamdef(now.AppliedParamdef);
            var nowRows=now.Rows.ToDictionary(r=>r.ID);
            foreach(var r in old.Rows)foreach(var c in r.Cells)
                if(!allowed(r,c.Def.InternalName)&&!System.Collections.StructuralComparisons.StructuralEqualityComparer.Equals(c.Value,nowRows[r.ID][c.Def.InternalName].Value))throw new Exception($"Unexpected change {name} {r.ID} {c.Def.InternalName}");
            if(name!="SpEffectParam")foreach(var r in now.Rows)foreach(var k in name=="EquipParamWeapon"?WeaponSlots:ArmorSlots)if(Affix(I(r,k))&&!effects.Rows.Any(e=>e.ID==I(r,k)))throw new Exception($"Dangling affix effect {r.ID}:{k}");
            if(name=="SpEffectParam"&&now.Rows.Count(r=>Affix(r.ID))!=AffixAssets.Kinds*4)throw new Exception("Affix effect count mismatch");
        }
        // 奖励详情：替换说明与介绍末尾的词条块
        foreach(var file in new[]{"affix-details.tsv","affix-v2-details.tsv"}) {
            var lines=File.ReadAllLines(Path.Combine(input,file)).Select(l=>{
                var p=l.Split('\t');if(!tsv.TryGetValue(uint.Parse(p[0]),out var t)||t.old==t.@new)return l;
                foreach(int column in new[]{3,4}) {
                    if(p[column].EndsWith(t.@new))continue;
                    if(!p[column].EndsWith(t.old))throw new Exception($"Affix text block not found in {file}:{p[0]}:{column}");
                    p[column]=p[column][..^t.old.Length]+t.@new;
                }
                return string.Join('\t',p);
            }).ToArray();
            File.WriteAllLines(Path.Combine(output,file),lines);
        }
        var auditLines=File.ReadAllLines(Path.Combine(input,"affix-v2-audit.tsv")).Select((l,i)=>{
            if(i==0)return l;var p=l.Split('\t');if(audit.TryGetValue(uint.Parse(p[0]),out var ids))p[8]=ids;return string.Join('\t',p);
        });
        File.WriteAllLines(Path.Combine(output,"affix-v2-audit.tsv"),auditLines);
        // 已有的对照表保留（文本可能尚未更新），本次新算出的覆盖同一编号
        var remapPath=Path.Combine(input,"affix-effect-remap.tsv");
        var merged=(File.Exists(remapPath)?File.ReadAllLines(remapPath):[]).Select(l=>l.Split('\t')).ToDictionary(p=>p[0],p=>(p[1],p[2]));
        foreach(var line in remap){var p=line.Split('\t');merged[p[0]]=(p[1],p[2]);}
        File.WriteAllLines(Path.Combine(output,"affix-effect-remap.tsv"),merged.OrderBy(e=>long.Parse(e.Key)).Select(e=>$"{e.Key}\t{e.Value.Item1}\t{e.Value.Item2}"));
        string Hash(string p)=>Convert.ToHexString(System.Security.Cryptography.SHA256.HashData(File.ReadAllBytes(p)));
        File.WriteAllText(Path.Combine(output,"affix-pool-build.json"),System.Text.Json.JsonSerializer.Serialize(new{kinds=AffixAssets.Kinds,variants=rerolled,rows,alreadyCurrent=already,textRemaps=merged.Count,sourceRegulationSha256=Hash(Path.Combine(input,"regulation.bin")),regulationSha256=Hash(path)},new System.Text.Json.JsonSerializerOptions{WriteIndented=true}));
        Console.WriteLine($"Rerolled {rerolled} affix variants ({rows} rows, {already} already current) over {AffixAssets.Kinds} kinds; {merged.Count} text remaps; only affix effects and resident slots changed");
    }
}
