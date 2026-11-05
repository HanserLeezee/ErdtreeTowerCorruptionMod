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
        names.Entries.Sort((a,b)=>a.ID.CompareTo(b.ID));file.Bytes=names.Write();
        var tables=ItemTextTables.Read(messages);
        foreach(var (tableId,name) in new[]{(21,"WeaponInfo"),(25,"WeaponCaption")}) {
            var target=messages.Files.Single(f=>f.ID==tableId);var fmg=FMG.Read(target.Bytes);
            var byId=fmg.Entries.ToDictionary(e=>e.ID);
            var source=tables[name].Entries.ToDictionary(e=>e.ID,e=>e.Text);
            foreach(int id in expected.Keys) {
                if(!source.TryGetValue(id/10000*10000,out var text)||string.IsNullOrWhiteSpace(text))throw new Exception($"Missing base {name}: {id}");
                if(byId.TryGetValue(id,out var entry)){if(entry.Text==text)continue;entry.Text=text;}
                else fmg.Entries.Add(new FMG.Entry(id,text));
                changed++;
            }
            fmg.Entries.Sort((a,b)=>a.ID.CompareTo(b.ID));target.Bytes=fmg.Write();
        }
        return changed;
    }
    public static void Verify(string input) {
        var messages=BND4.Read(Path.Combine(input,"msg/zhocn/item_dlc02.msgbnd.dcx"));
        var names=ItemTextTables.Read(messages)["WeaponName"].Entries.ToDictionary(e=>e.ID,e=>e.Text);
        var expected=Expected(input,messages);
        VerifyDescriptions(messages,expected.Keys);
        foreach(var (id,text) in expected) {
            if(names.GetValueOrDefault(id)!=text)throw new Exception($"Affinity name missing or incorrect: {id}; expected {EquipmentStyleAssets.Plain(text)}");
            if(Regex.Matches(text,"<font ").Count!=1||!text.EndsWith("</font>")||text.Contains('\ufffd'))throw new Exception("Invalid affinity name markup");
            for(int level=0;level<=25;level++)if((id+level)/100*100!=id)throw new Exception("Invalid reinforcement name lookup");
        }
        var before=messages.Files.ToDictionary(f=>f.ID,f=>f.Bytes.ToArray());
        if(Apply(input,messages)!=0||messages.Files.Any(f=>!f.Bytes.SequenceEqual(before[f.ID])))throw new Exception("Affinity text patch is not idempotent");
        Console.WriteLine($"Verified {expected.Count} affinity names, rarity colors, reinforcement name IDs and repeatability");
    }
    static void VerifyDescriptions(BND4 messages,IEnumerable<int> ids) {
        var tables=ItemTextTables.Read(messages);
        foreach(var name in new[]{"WeaponInfo","WeaponCaption"}) {
            var entries=tables[name].Entries.ToDictionary(e=>e.ID,e=>e.Text);
            foreach(int id in ids) {
                if(!entries.TryGetValue(id/10000*10000,out var basis)||string.IsNullOrWhiteSpace(basis))throw new Exception($"Missing base {name}: {id}");
                if(entries.GetValueOrDefault(id)!=basis)throw new Exception($"Missing or incorrect affinity {name}: {id}");
            }
        }
    }
    public static void Patch(string input,string output) {
        if(Path.GetFullPath(input)==Path.GetFullPath(output))throw new Exception("Separate staging required");
        const string relative="msg/zhocn/item_dlc02.msgbnd.dcx";
        var messages=BND4.Read(Path.Combine(input,relative));
        var before=messages.Files.ToDictionary(f=>f.ID,f=>f.Bytes.ToArray());
        int changed=Apply(input,messages);
        var path=Path.Combine(output,relative);Directory.CreateDirectory(Path.GetDirectoryName(path)!);messages.Write(path);
        var read=BND4.Read(path);
        foreach(var f in read.Files) {
            if(!f.Bytes.SequenceEqual(messages.Files.Single(x=>x.ID==f.ID).Bytes))throw new Exception("Text roundtrip failed");
            if(f.ID is not (11 or 21 or 25)&&!f.Bytes.SequenceEqual(before[f.ID]))throw new Exception("Unrelated message table changed");
        }
        VerifyDescriptions(read,Expected(input,read).Keys);
        if(Apply(input,read)!=0)throw new Exception("Affinity text patch is not idempotent");
        Console.WriteLine($"Updated {changed} affinity text entries; descriptions, roundtrip and idempotence verified");
    }
    public static void VerifyEffects(string input,string definitions) {
        var reg=SoulsFormats.Cryptography.RegulationDecryptor.DecryptERRegulation(Path.Combine(input,"regulation.bin"));
        var weapons=PARAM.Read(reg.Files.Single(f=>Path.GetFileNameWithoutExtension(f.Name.Replace('\\','/'))=="EquipParamWeapon").Bytes);
        weapons.ApplyParamdef(PARAMDEF.XmlDeserialize(Path.Combine(definitions,"EquipParamWeapon.xml")));
        var rows=weapons.Rows.ToDictionary(r=>r.ID);
        int[] Effects(PARAM.Row row)=>new[]{"residentSpEffectId","residentSpEffectId1","residentSpEffectId2"}.Select(k=>Convert.ToInt32(row[k].Value)).Where(id=>id>=9850000&&id<9850210).Order().ToArray();
        int checkedRows=0;
        foreach(var p in new[]{"affix-catalog.txt","affix-v2-catalog.txt"}.SelectMany(f=>File.ReadAllLines(Path.Combine(input,f))).Select(l=>l.Split('|')).Where(p=>p[0]=="weapon")) {
            int id=int.Parse(p[1]);var effects=Effects(rows[id]);
            for(int affinity=1;affinity<=12;affinity++)if(rows.TryGetValue(id+affinity*100,out var row)) {
                if(!Effects(row).SequenceEqual(effects))throw new Exception($"Affinity affix effects changed: {row.ID}");checkedRows++;
            }
        }
        Console.WriteLine($"Verified native affix effects preserved on {checkedRows} affinity rows");
    }
}
