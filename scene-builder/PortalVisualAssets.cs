using SoulsFormats;

static class PortalVisualAssets {
    public const string Model="AEG990_446",FileName="asset/aeg/aeg990/aeg990_446.geombnd.dcx";
    const int Param=990446,Source=220446,Carrier=99991;
    const string SourceModel="AEG220_446",RowName="Tower decorative portal arch";
    static PARAM Read(BND4 reg,PARAMDEF def) {
        var p=PARAM.Read(reg.Files.Single(f=>Path.GetFileNameWithoutExtension(f.Name)=="AssetEnvironmentGeometryParam").Bytes);
        p.ApplyParamdef(def);return p;
    }
    static PARAM.Row Row(PARAM p) {
        var row=new PARAM.Row(p.Rows.Single(r=>r.ID==Source)){ID=Param,Name=RowName};
        var carrier=p.Rows.Single(r=>r.ID==Carrier);
        foreach(string field in new[]{"behaviorType","hitCreateType","collisionType"})row[field].Value=carrier[field].Value;
        return row;
    }
    public static void Build(string game,string input,string output) {
        const string original="/asset/aeg/aeg220/aeg220_446.geombnd.dcx";
        var cache=Path.Combine(output,"source-cache","aeg220_446.geombnd.dcx");
        if(!File.Exists(cache)) {
            ulong hash=0;foreach(char ch in original)hash=unchecked(hash*0x85+ch);
            foreach(var archive in new[]{"DLC","Data0","Data1","Data2","Data3"}) {
                using var header=UXM.CryptographyUtility.DecryptRsa(Path.Combine(game,archive+".bhd"),UXM.ArchiveKeys.EldenRingKeys[archive]);
                var entry=BHD5.Read(header,BHD5.Game.EldenRing).Buckets.SelectMany(b=>b).FirstOrDefault(f=>f.FileNameHash==hash);
                if(entry==null)continue;
                using var data=File.OpenRead(Path.Combine(game,archive+".bdt"));
                Directory.CreateDirectory(Path.GetDirectoryName(cache)!);File.WriteAllBytes(cache,entry.ReadFile(data));break;
            }
        }
        var model=BND4.Read(cache);var originalFiles=model.Files.ToArray();
        if(originalFiles.Length!=1||!originalFiles[0].Name.EndsWith(".flver"))throw new Exception("Expected render-only arch binder");
        var originalBytes=originalFiles[0].Bytes.ToArray();
        model.Files[0].Name=model.Files[0].Name.Replace(SourceModel,Model);
        var path=Path.Combine(output,FileName);Directory.CreateDirectory(Path.GetDirectoryName(path)!);model.Write(path);
        var checkModel=BND4.Read(path);
        if(checkModel.Files.Count!=1||!checkModel.Files[0].Name.Contains(Model)||!checkModel.Files[0].Bytes.SequenceEqual(originalBytes))throw new Exception("Decorative arch render data changed");
        // Only the render binder is copied; neither high nor low collision archives exist for this model.
        foreach(var root in new[]{input,output})foreach(var suffix in new[]{"_h.geomhkxbnd.dcx","_l.geomhkxbnd.dcx"})
            if(File.Exists(Path.Combine(root,"asset/aeg/aeg990/aeg990_446"+suffix)))throw new Exception("Decorative arch must not have collision archives");
        var reg=SoulsFormats.Cryptography.RegulationDecryptor.DecryptERRegulation(Path.Combine(input,"regulation.bin"));
        var defPath="work/SoulsRandomizers/diste/Defs/AssetGeometryParam.xml";if(!File.Exists(defPath))defPath="../../"+defPath;
        var def=PARAMDEF.XmlDeserialize(defPath);var p=Read(reg,def);var file=reg.Files.Single(f=>Path.GetFileNameWithoutExtension(f.Name)=="AssetEnvironmentGeometryParam");
        if(p.Rows.Any(r=>r.ID==Param&&r.Name!=RowName))throw new Exception("Decorative arch param ID already used");
        var before=Read(reg,def);var others=reg.Files.Where(f=>f!=file).ToDictionary(f=>f.Name,f=>f.Bytes.ToArray());
        p.Rows.RemoveAll(r=>r.ID==Param);p.Rows.Add(Row(p));p.Rows.Sort((a,b)=>a.ID.CompareTo(b.ID));file.Bytes=p.Write();
        var target=Path.Combine(output,"regulation.bin");SoulsFormats.Cryptography.RegulationDecryptor.EncryptERRegulation(target,reg);
        var check=SoulsFormats.Cryptography.RegulationDecryptor.DecryptERRegulation(target);var actual=Read(check,def);var rows=actual.Rows.ToDictionary(r=>r.ID);
        foreach(var f in check.Files.Where(f=>f.Name!=file.Name))if(!f.Bytes.SequenceEqual(others[f.Name]))throw new Exception("Unrelated param changed: "+f.Name);
        foreach(var old in before.Rows.Where(r=>r.ID!=Param).Append(Row(before)))foreach(var c in old.Cells)
            if(!System.Collections.StructuralComparisons.StructuralEqualityComparer.Equals(c.Value,rows[old.ID][c.Def.InternalName].Value))throw new Exception($"Unexpected geometry param change: {old.ID}/{c.Def.InternalName}");
        if(actual.Rows.Count!=before.Rows.Count(r=>r.ID!=Param)+1)throw new Exception("Unexpected geometry row count");
        Console.WriteLine("Decorative arch: intact render mesh, dynamic visual-only carrier settings, no collision archives; original geometry params preserved");
    }
    public static void Manifest(string directory) {
        var path=Path.Combine(directory,"arenas-resources.txt");var rows=File.ReadAllLines(path).Where(l=>l.Split('|')[0]!=FileName).ToList();
        var bytes=File.ReadAllBytes(Path.Combine(directory,FileName));ulong hash=14695981039346656037;
        foreach(byte value in bytes)hash=unchecked((hash^value)*1099511628211);
        rows.Add($"{FileName}|{bytes.Length}|{hash:x16}");File.WriteAllLines(path,rows);
    }
}
