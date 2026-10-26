using SoulsFormats;
using System.Text.RegularExpressions;

static class AffinityNameAssets {
    static Dictionary<int,string> Expected(string input,BND4 messages) {
        var names=ItemTextTables.Read(messages)["WeaponName"].Entries.ToDictionary(e=>e.ID,e=>EquipmentStyleAssets.Plain(e.Text));
        var colors=File.ReadAllLines(Path.Combine(input,"equipment-colors.txt")).Select(l=>l.Split('|')).ToDictionary(p=>int.Parse(p[0]),p=>p[1]);
        var reg=SoulsFormats.Cryptography.RegulationDecryptor.DecryptERRegulation(Path.Combine(input,"regulation.bin"));
        var rows=PARAM.Read(reg.Files.Single(f=>Path.GetFileNameWithoutExtension(f.Name.Replace('\\','/'))=="EquipParamWeapon").Bytes).Rows.Select(r=>r.ID).ToHashSet();
        var result=new Dictionary<int,string>();
        foreach(var p in new[]{"affix-catalog.txt","affix-v2-catalog.txt"}.SelectMany(f=>File.ReadAllLines(Path.Combine(input,f))).Select(l=>l.Split('|')).Where(p=>p[0]=="weapon")) {
            int id=int.Parse(p[1]),baseId=int.Parse(p[2]),tier=int.Parse(p[3]);
            string basic=names[baseId],canonical=p[4];int start=canonical.LastIndexOf(basic,StringComparison.Ordinal);
            if(start<0||canonical[(start+basic.Length)..].Trim(' ','★')!="")throw new Exception($"Cannot identify weapon name in variant {id}");
            for(int affinity=1;affinity<=12;affinity++) {
                int offset=affinity*100;if(!rows.Contains(id+offset))continue;
                if(!names.TryGetValue(baseId+offset,out var original)||string.IsNullOrWhiteSpace(original))throw new Exception($"Missing source affinity name: {baseId+offset}");
                string name=canonical[..start]+original+canonical[(start+basic.Length)..];
                result.Add(id+offset,$"<font color=\"#{colors[tier]}\">{name}</font>");
            }
        }
        return result;
    }
    public static int Apply(string input,BND4 messages) {
        var expected=Expected(input,messages);
        var file=messages.Files.Single(f=>f.ID==11);var names=FMG.Read(file.Bytes);
        var entries=names.Entries.ToDictionary(e=>e.ID);int changed=0;
        foreach(var (id,text) in expected) {
            if(entries.TryGetValue(id,out var entry)) {if(entry.Text==text)continue;entry.Text=text;}
            else names.Entries.Add(new FMG.Entry(id,text));
            changed++;
        }
        names.Entries.Sort((a,b)=>a.ID.CompareTo(b.ID));file.Bytes=names.Write();return changed;
    }
    public static void Verify(string input) {
        var messages=BND4.Read(Path.Combine(input,"msg/zhocn/item_dlc02.msgbnd.dcx"));
        var names=ItemTextTables.Read(messages)["WeaponName"].Entries.ToDictionary(e=>e.ID,e=>e.Text);
        var expected=Expected(input,messages);
        foreach(var (id,text) in expected) {
            if(names.GetValueOrDefault(id)!=text)throw new Exception($"Affinity name missing or incorrect: {id}; expected {EquipmentStyleAssets.Plain(text)}");
            if(Regex.Matches(text,"<font ").Count!=1||!text.EndsWith("</font>")||text.Contains('\ufffd'))throw new Exception("Invalid affinity name markup");
            for(int level=0;level<=25;level++)if((id+level)/100*100!=id)throw new Exception("Invalid reinforcement name lookup");
        }
        var before=messages.Files.ToDictionary(f=>f.ID,f=>f.Bytes.ToArray());
        if(Apply(input,messages)!=0||messages.Files.Any(f=>!f.Bytes.SequenceEqual(before[f.ID])))throw new Exception("Affinity text patch is not idempotent");
        Console.WriteLine($"Verified {expected.Count} affinity names, rarity colors, reinforcement name IDs and repeatability");
    }
}
