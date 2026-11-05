using SoulsFormats;
using System.Text.RegularExpressions;

// 词条武器的品质对武器本身数值的影响：每高一档基础攻击 +AttackStep、属性补正 +ScalingStep（原来分别为 4%、3%（新版武器 4%））。
// affix-quality-patch：按生成器（AffixAssets、AdvancedAffixAssets）的同一套种子和公式，从原版武器行重新算出每个词条武器的基础攻击与补正；
// 先用旧系数重算一遍，确认和当前 regulation 一致（说明没有被别的补丁改过），再换成新系数写回。只改这些字段，其余参数逐字节不变，可重复运行。
// 奖励详情（affix-details.tsv、affix-v2-details.tsv）里的“基础攻击力”数字同步更新。
static class AffixQualityAssets {
    public const double AttackStep=0.08,ScalingStep=0.06;
    const double OldAttack=0.04,OldScalingV1=0.03,OldScalingV2=0.04;
    static readonly string[] Damage=["Physics","Magic","Fire","Thunder","Dark"];
    static readonly string[] Scaling=["correctStrength","correctAgility","correctMagic","correctFaith","correctLuck"];
    static int I(PARAM.Row r,string k)=>Convert.ToInt32(r[k].Value);
    static double F(PARAM.Row r,string k)=>Convert.ToDouble(r[k].Value);
    static ulong Mix(ulong n)=>AffixAssets.Mix(n);
    /// 一个词条武器（含质变行）的目标数值：行号 → (五种基础攻击, 五种补正或 null 表示不改)
    record Values(int[] Attack,double[] Scaling);
    static Dictionary<int,Values> V1(PARAM weapons,Dictionary<int,PARAM.Row> rows,int id,int baseId,int tier,double attackStep,double scalingStep) {
        var result=new Dictionary<int,Values>();
        ulong seed=Mix((ulong)baseId^((ulong)tier<<32)^0xa771f1UL);
        double roll=(Mix(seed^(ulong)baseId)%1001)/1000.0;
        foreach(var original in rows.Values.Where(r=>r.ID>=baseId&&r.ID<baseId+10000)) {
            var attack=Damage.Select(e=>Math.Clamp((int)Math.Round(I(original,"attackBase"+e)*(0.96+roll*0.08+tier*attackStep)),0,ushort.MaxValue)).ToArray();
            var scaling=Scaling.Select(f=>Math.Round(F(original,f)*(0.92+(Mix(seed^(ulong)f.Length)%1001)/1000.0*0.16+tier*scalingStep),2)).ToArray();
            result[id+original.ID-baseId]=new(attack,scaling);
        }
        return result;
    }
    static Dictionary<int,Values> V2(Dictionary<int,PARAM.Row> rows,int id,int baseId,int tier,int profile,double attackStep,double scalingStep) {
        var result=new Dictionary<int,Values>();
        ulong seed=Mix((ulong)baseId^((ulong)tier<<32)^((ulong)profile<<40)^0xaff22026UL);
        double roll=(Mix(seed^43)%10001)/10000.0;var source=rows[baseId];int mask=AdvancedAffixAssets.Mask(source,seed,profile);
        int mode=(int)(Mix(seed^47)%5),stars=roll>=0.995?2:roll>=0.97?1:0;
        double damageRate=0.94+roll*0.12+tier*attackStep+(stars==2?0.12:0);
        double totalWeight=Enumerable.Range(0,5).Where(i=>(mask&(1<<i))!=0).Sum(i=>70d+Mix(seed^(ulong)(200+i))%61);
        foreach(var original in rows.Values.Where(r=>r.ID>=baseId&&r.ID<baseId+10000)) {
            var attack=Damage.Select(e=>Math.Clamp((int)Math.Round(I(original,"attackBase"+e)*damageRate),0,ushort.MaxValue)).ToArray();
            if(original.ID==baseId&&mode!=0) {
                int total=(int)Math.Round(Damage.Sum(e=>I(original,"attackBase"+e))*damageRate*1.1);
                int physical=(int)Math.Round(total*(Mix(seed^53)%100<25?0:0.55));
                attack=new int[5];attack[0]=Math.Clamp(physical,0,ushort.MaxValue);attack[mode]=Math.Clamp(total-physical,0,ushort.MaxValue);
            }
            double budget=Math.Max(60,Scaling.Sum(k=>F(original,k)))*(0.95+roll*0.1+tier*scalingStep);
            var scaling=Enumerable.Range(0,5).Select(i=>(mask&(1<<i))==0?0:Math.Round(budget*(70+Mix(seed^(ulong)(200+i))%61)/totalWeight,2)).ToArray();
            result[id+original.ID-baseId]=new(attack,scaling);
        }
        return result;
    }
    static bool Matches(PARAM.Row row,Values v)=>Enumerable.Range(0,5).All(i=>I(row,"attackBase"+Damage[i])==v.Attack[i]&&Math.Abs(F(row,Scaling[i])-v.Scaling[i])<0.006);
    public static void Patch(string input,string output) {
        if(Path.GetFullPath(input)==Path.GetFullPath(output))throw new Exception("Separate staging output required");
        var reg=SoulsFormats.Cryptography.RegulationDecryptor.DecryptERRegulation(Path.Combine(input,"regulation.bin"));
        var file=reg.Files.Single(f=>Path.GetFileNameWithoutExtension(f.Name.Replace('\\','/'))=="EquipParamWeapon");
        var weapons=PARAM.Read(file.Bytes);weapons.ApplyParamdef(PARAMDEF.XmlDeserialize("work/SoulsRandomizers/diste/Defs/EquipParamWeapon.xml"));
        var original=PARAM.Read(file.Bytes);original.ApplyParamdef(weapons.AppliedParamdef);
        var rows=weapons.Rows.ToDictionary(r=>r.ID);
        var vanilla=rows.Where(p=>p.Key<100000000).ToDictionary(p=>p.Key,p=>p.Value);
        var v1=File.ReadAllLines(Path.Combine(input,"affix-catalog.txt")).Select(l=>l.Split('|')).Where(p=>p[0]=="weapon").ToArray();
        var v2=File.ReadAllLines(Path.Combine(input,"affix-v2-catalog.txt")).Select(l=>l.Split('|')).Where(p=>p[0]=="weapon").ToArray();
        // (变体编号, 旧值, 新值)
        var plan=new List<(int variant,Dictionary<int,Values> old,Dictionary<int,Values> next)>();
        foreach(var p in v1){int id=int.Parse(p[1]),baseId=int.Parse(p[2]),tier=int.Parse(p[3]);
            plan.Add((id,V1(weapons,vanilla,id,baseId,tier,OldAttack,OldScalingV1),V1(weapons,vanilla,id,baseId,tier,AttackStep,ScalingStep)));}
        foreach(var p in v2){int id=int.Parse(p[1]),baseId=int.Parse(p[2]),tier=int.Parse(p[3]),profile=int.Parse(p[5]);
            plan.Add((id,V2(vanilla,id,baseId,tier,profile,OldAttack,OldScalingV2),V2(vanilla,id,baseId,tier,profile,AttackStep,ScalingStep)));}
        // 当前数值必须等于旧公式（首次运行）或新公式（已经运行过）
        var (old,current,other)=(0,0,0);var strays=new List<int>();
        foreach(var (_,before,after) in plan)foreach(var (id,value) in before) {
            if(!rows.TryGetValue(id,out var row))throw new Exception($"Affix weapon row missing: {id}");
            if(Matches(row,after[id]))current++;else if(Matches(row,value))old++;else{other++;if(strays.Count<10)strays.Add(id);}
        }
        Console.WriteLine($"Affix weapon rows: {old} at the old quality steps, {current} already at the new steps, {other} unexplained");
        if(other>0)throw new Exception("Rows differ from both formulas, refusing to rewrite: "+string.Join(",",strays));
        int changed=0;
        foreach(var (_,_,after) in plan)foreach(var (id,value) in after) {
            var row=rows[id];if(Matches(row,value))continue;
            for(int i=0;i<5;i++){row["attackBase"+Damage[i]].Value=Convert.ChangeType(value.Attack[i],row["attackBase"+Damage[i]].Value.GetType());row[Scaling[i]].Value=Convert.ChangeType(value.Scaling[i],row[Scaling[i]].Value.GetType());}
            changed++;
        }
        if(changed>0)file.Bytes=weapons.Write();
        var others=reg.Files.Where(f=>f!=file).ToDictionary(f=>f.Name,f=>f.Bytes.ToArray());
        Directory.CreateDirectory(output);var path=Path.Combine(output,"regulation.bin");
        SoulsFormats.Cryptography.RegulationDecryptor.EncryptERRegulation(path,reg);
        if(new FileInfo(path).Length>4799*1024)throw new Exception("Regulation size limit exceeded");
        var check=SoulsFormats.Cryptography.RegulationDecryptor.DecryptERRegulation(path);
        foreach(var f in check.Files.Where(f=>f.Name!=file.Name))if(!f.Bytes.SequenceEqual(others[f.Name]))throw new Exception($"Unrelated param changed: {f.Name}");
        var actual=PARAM.Read(check.Files.Single(f=>f.Name==file.Name).Bytes);actual.ApplyParamdef(weapons.AppliedParamdef);var byId=actual.Rows.ToDictionary(r=>r.ID);
        var allowed=Damage.Select(e=>"attackBase"+e).Concat(Scaling).ToHashSet();
        foreach(var row in original.Rows)foreach(var cell in row.Cells)
            if(!allowed.Contains(cell.Def.InternalName)&&!System.Collections.StructuralComparisons.StructuralEqualityComparer.Equals(cell.Value,byId[row.ID][cell.Def.InternalName].Value))throw new Exception($"Unrelated weapon field changed: {row.ID} {cell.Def.InternalName}");
        foreach(var (_,_,after) in plan)foreach(var (id,value) in after)if(!Matches(byId[id],value))throw new Exception($"Affix weapon not updated: {id}");
        // 奖励详情里的基础攻击力（+0 标准行）
        var basis=plan.ToDictionary(p=>(uint)p.variant,p=>p.next[p.variant].Attack);
        foreach(var name in new[]{"affix-details.tsv","affix-v2-details.tsv"}) {
            var lines=File.ReadAllLines(Path.Combine(input,name));int updated=0;
            for(int n=0;n<lines.Length;n++) {
                var cells=lines[n].Split('\t');if(!uint.TryParse(cells[0],out var key)||!basis.TryGetValue(key,out var attack))continue;
                var text=Regex.Replace(cells[5],@"物理 \d+　魔力 \d+　(火焰?) \d+　(雷电?) \d+　(神?圣) \d+",m=>$"物理 {attack[0]}　魔力 {attack[1]}　{m.Groups[1].Value} {attack[2]}　{m.Groups[2].Value} {attack[3]}　{m.Groups[3].Value} {attack[4]}");
                if(text!=cells[5]){cells[5]=text;lines[n]=string.Join('\t',cells);updated++;}
            }
            File.WriteAllLines(Path.Combine(output,name),lines);
            Console.WriteLine($"{name}: {updated} detail lines updated");
        }
        // 抽样：同一把武器四档的 +0 物理攻击
        foreach(var group in plan.Take(8).GroupBy(p=>p.variant/40000))Console.WriteLine("Sample "+string.Join(" / ",group.Select(p=>$"{p.variant}:{string.Join(",",p.next[p.variant].Attack)}")));
        Console.WriteLine($"Verified: {changed} affix weapon rows rewritten (attack +{AttackStep:P0}, scaling +{ScalingStep:P0} per quality tier); all other params unchanged");
    }
}
