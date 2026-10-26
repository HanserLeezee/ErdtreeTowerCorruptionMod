using SoulsFormats;

static class UnrestrictedAshAssets {
    public const int ShopFirst=9500000,ShopCount=37;
    static readonly HashSet<int> Types=[1,3,5,7,9,11,13,14,15,16,17,19,21,23,24,25,28,29,31,35,37,39,41,50,51,53,55,56,57,61,65,67,69,87,88,89,90,91,92,93,94,95];
    static int I(PARAM.Row row,string field)=>Convert.ToInt32(row[field].Value);
    static void Set(PARAM.Row row,string field,int value)=>row[field].Value=Convert.ChangeType(value,row[field].Value.GetType());
    public static void Patch(string input,string output) {
        if(Path.GetFullPath(input)==Path.GetFullPath(output))throw new Exception("Separate staging directory required");
        var reg=SoulsFormats.Cryptography.RegulationDecryptor.DecryptERRegulation(Path.Combine(input,"regulation.bin"));
        var original=reg.Files.ToDictionary(f=>f.Name,f=>f.Bytes.ToArray());
        BinderFile File_(string name)=>reg.Files.Single(f=>Path.GetFileNameWithoutExtension(f.Name.Replace('\\','/'))==name);
        PARAM Read(string name){var p=PARAM.Read(File_(name).Bytes);p.ApplyParamdef(PARAMDEF.XmlDeserialize($"work/SoulsRandomizers/diste/Defs/{name}.xml"));return p;}
        var weapons=Read("EquipParamWeapon");var gems=Read("EquipParamGem");var custom=Read("EquipParamCustomWeapon");
        var sold=File.ReadAllLines(Path.Combine(input,"shop-catalog.txt")).Select(l=>l.Split('|')).Where(p=>p[0]=="4").Select(p=>int.Parse(p[1])).ToHashSet();
        var pool=gems.Rows.Where(r=>sold.Contains(r.ID)).OrderBy(r=>r.ID).ToArray();
        if(pool.Length<100||!pool.Any(r=>r.ID>=400000))throw new Exception("Incomplete base/DLC ash pool");
        var targetWeapons=weapons.Rows.Where(r=>Types.Contains(I(r,"wepType"))).ToArray();
        foreach(var row in targetWeapons){Set(row,"gemMountType",2);Set(row,"restrictSpecialSwordArt",0);}
        foreach(var row in pool)foreach(var cell in row.Cells.Where(c=>c.Def.InternalName.StartsWith("canMountWep_")))Set(row,cell.Def.InternalName,1);
        for(int i=0;i<ShopCount;i++) {
            var row=custom.Rows.SingleOrDefault(r=>r.ID==ShopFirst+i);
            if(row!=null&&row.Name!="Tower random ash shop slot")throw new Exception("Custom weapon row collision");
            if(row==null){row=new PARAM.Row(custom.Rows[0]){ID=ShopFirst+i,Name="Tower random ash shop slot"};custom.Rows.Add(row);}
            Set(row,"baseWepId",1000000);Set(row,"gemId",pool[0].ID);Set(row,"reinforceLv",0);
        }
        custom.Rows.Sort((a,b)=>a.ID.CompareTo(b.ID));
        foreach(var (name,param) in new[]{("EquipParamWeapon",weapons),("EquipParamGem",gems),("EquipParamCustomWeapon",custom)}) {
            var old=PARAM.Read(original[File_(name).Name]);old.ApplyParamdef(param.AppliedParamdef);var rows=param.Rows.ToDictionary(r=>r.ID);
            foreach(var row in old.Rows)foreach(var cell in row.Cells) {
                bool allowed=name=="EquipParamWeapon"&&Types.Contains(I(row,"wepType"))&&(cell.Def.InternalName is "gemMountType" or "restrictSpecialSwordArt")
                    ||name=="EquipParamGem"&&sold.Contains(row.ID)&&cell.Def.InternalName.StartsWith("canMountWep_")
                    ||name=="EquipParamCustomWeapon"&&row.ID>=ShopFirst&&row.ID<ShopFirst+ShopCount;
                if(!allowed&&!System.Collections.StructuralComparisons.StructuralEqualityComparer.Equals(cell.Value,rows[row.ID][cell.Def.InternalName].Value))throw new Exception("Unrelated field changed");
            }
            File_(name).Bytes=param.Write();
        }
        Directory.CreateDirectory(output);var path=Path.Combine(output,"regulation.bin");SoulsFormats.Cryptography.RegulationDecryptor.EncryptERRegulation(path,reg);
        if(new FileInfo(path).Length>4799*1024)throw new Exception("Regulation size limit exceeded");
        var roundtrip=SoulsFormats.Cryptography.RegulationDecryptor.DecryptERRegulation(path);
        foreach(var f in reg.Files)if(!f.Bytes.SequenceEqual(roundtrip.Files.Single(r=>r.Name==f.Name).Bytes))throw new Exception("Regulation roundtrip failed");
        File.WriteAllLines(Path.Combine(output,"unrestricted-ashes.txt"),pool.Select(r=>r.ID.ToString()));
        var arts=Read("SwordArtsParam").Rows.ToDictionary(r=>r.ID);
        var texts=ItemTextTables.Read(BND4.Read(Path.Combine(input,"msg/zhocn/item_dlc02.msgbnd.dcx")));
        string Text(string table,int id)=>EquipmentStyleAssets.Plain(texts[table].Entries.Single(e=>e.ID==id).Text).Replace("\r","").Replace("\n","\\n").Replace("\t"," ");
        File.WriteAllLines(Path.Combine(output,"random-ash-details.tsv"),pool.Select(r=>{
            int textId=I(arts[I(r,"swordArtsParamId")],"textId");return $"{r.ID}\t{Text("ArtsName",textId)}\t{Text("ArtsCaption",textId)}";
        }));
        File.WriteAllLines(Path.Combine(output,"random-ash-weapon-rows.txt"),targetWeapons.Select(r=>r.ID).Order().Select(id=>id.ToString()));
        File.WriteAllText(Path.Combine(output,"unrestricted-ashes-build.json"),System.Text.Json.JsonSerializer.Serialize(new{ashes=pool.Length,weaponRows=targetWeapons.Length,customShopRows=ShopCount,perInstance=true,affinityRestrictionsPreserved=true}));
        Console.WriteLine($"Verified {pool.Length} unrestricted ashes, {targetWeapons.Length} weapon rows and {ShopCount} custom shop slots; all unrelated fields preserved");
    }
}
