using SoulsFormats;

static class StormveilRuneAssets {
    const string MapPath="map/mapstudio/m10_00_00_00.msb.dcx";
    const string Prefix="Tower parkour no-rune source:";
    const int First=970000000;
    static string Name(BinderFile f)=>Path.GetFileNameWithoutExtension(f.Name.Replace('\\','/'));
    public static void Patch(string input,string output) {
        if(Path.GetFullPath(input)==Path.GetFullPath(output))throw new Exception("Separate staging required");
        var map=MSBE.Read(Path.Combine(input,MapPath));
        var reg=SoulsFormats.Cryptography.RegulationDecryptor.DecryptERRegulation(Path.Combine(input,"regulation.bin"));
        var file=reg.Files.Single(f=>Name(f)=="NpcParam");
        var original=reg.Files.ToDictionary(f=>f.Name,f=>f.Bytes.ToArray());
        var param=PARAM.Read(file.Bytes);param.ApplyParamdef(PARAMDEF.XmlDeserialize("work/SoulsRandomizers/diste/Defs/NpcParam.xml"));
        var rows=param.Rows.ToDictionary(r=>r.ID);
        int Source(int id)=>rows.TryGetValue(id,out var r)&&r.Name?.StartsWith(Prefix)==true?int.Parse(r.Name[Prefix.Length..]):id;
        var ids=map.Parts.Enemies.Where(e=>e.NPCParamID>=0).Select(e=>Source(e.NPCParamID)).Distinct().Order().ToArray();
        if(ids.Length==0||ids.Length>=10000)throw new Exception("Unexpected Stormveil NPC count");
        var mapped=ids.Select((id,i)=>(id,target:First+i)).ToDictionary(p=>p.id,p=>p.target);
        foreach(var id in ids) {
            if(!rows.TryGetValue(id,out var source))throw new Exception($"Missing NPC {id}");
            int target=mapped[id];string name=Prefix+id;
            if(rows.TryGetValue(target,out var existing)&&existing.Name!=name)throw new Exception($"NPC collision {target}");
            var row=new PARAM.Row(source){ID=target,Name=name};
            row["getSoul"].Value=Convert.ChangeType(0,row["getSoul"].Value.GetType());
            if(existing!=null)param.Rows.Remove(existing);
            param.Rows.Add(row);
        }
        foreach(var enemy in map.Parts.Enemies.Where(e=>e.NPCParamID>=0))enemy.NPCParamID=mapped[Source(enemy.NPCParamID)];
        param.Rows.Sort((a,b)=>a.ID.CompareTo(b.ID));file.Bytes=param.Write();
        Directory.CreateDirectory(Path.Combine(output,"map/mapstudio"));
        var targetReg=Path.Combine(output,"regulation.bin");
        SoulsFormats.Cryptography.RegulationDecryptor.EncryptERRegulation(targetReg,reg);
        if(new FileInfo(targetReg).Length>4799*1024)throw new Exception("Regulation size limit exceeded");
        map.Write(Path.Combine(output,MapPath));
        var actual=SoulsFormats.Cryptography.RegulationDecryptor.DecryptERRegulation(targetReg);
        foreach(var f in actual.Files.Where(f=>Name(f)!="NpcParam"))if(!f.Bytes.SequenceEqual(original[f.Name]))throw new Exception("Unrelated table changed");
        var checkedParam=PARAM.Read(actual.Files.Single(f=>Name(f)=="NpcParam").Bytes);checkedParam.ApplyParamdef(param.AppliedParamdef);
        var checkedRows=checkedParam.Rows.ToDictionary(r=>r.ID);
        foreach(var row in rows.Values.Where(r=>!mapped.Values.Contains(r.ID)))foreach(var cell in row.Cells)
            if(!System.Collections.StructuralComparisons.StructuralEqualityComparer.Equals(cell.Value,checkedRows[row.ID][cell.Def.InternalName].Value))throw new Exception("Original NPC changed");
        foreach(var id in ids)foreach(var cell in rows[id].Cells) {
            var value=checkedRows[mapped[id]][cell.Def.InternalName].Value;
            if(cell.Def.InternalName=="getSoul") {if(Convert.ToInt64(value)!=0)throw new Exception("Parkour NPC still grants runes");}
            else if(!System.Collections.StructuralComparisons.StructuralEqualityComparer.Equals(cell.Value,value))throw new Exception("Unrelated NPC field changed");
        }
        var checkedMap=MSBE.Read(Path.Combine(output,MapPath));
        if(checkedMap.Parts.Enemies.Any(e=>e.NPCParamID>=0&&(!mapped.Values.Contains(e.NPCParamID)||Convert.ToInt64(checkedRows[e.NPCParamID]["getSoul"].Value)!=0)))throw new Exception("Parkour enemy not isolated");
        foreach(var enemy in checkedMap.Parts.Enemies.Where(e=>e.NPCParamID>=0))enemy.NPCParamID=ids[enemy.NPCParamID-First];
        var baseline=MSBE.Read(Path.Combine(input,MapPath));
        foreach(var enemy in baseline.Parts.Enemies.Where(e=>e.NPCParamID>=0))enemy.NPCParamID=Source(enemy.NPCParamID);
        if(!checkedMap.Write().SequenceEqual(baseline.Write()))throw new Exception("Unrelated map data changed");
        Console.WriteLine($"Verified {ids.Length} isolated zero-rune NPC rows for {map.Parts.Enemies.Count} Stormveil actors; original NPCs, placements, drops and event rewards unchanged");
    }
}
