using SoulsFormats;

// 徘徊赐福：单人死亡后自动消耗一个、返回原房间的道具（开局两个，旅程结束收回）。
// 物品行照强化石的写法：不能丢弃、寄存、掉落，不能在道具栏使用；背包图标借用原版卢恩弧（道具 190）的图标编号（运行时从参数里读，不写死）。
// retry-item-patch：只改 regulation（Linux 上也能运行）；retry-item-text：写名字与说明（需要游戏文字，在 Windows 上运行）；完整构建在 NativeAssets 里调用 Build。
static class RetryItemAssets {
    public const int Item=7500110,Max=9,IconSource=190;
    const string Name="徘徊赐福";
    const string Info="单人旅程中死亡时自动消耗一个：旅程不会结束，复活后自动返回死亡的房间重新挑战，生命与圣杯瓶恢复到进入本层时的状态。\n每次旅程开始时获得两个，旅程结束时收回；联机旅程不能使用。";
    const string Caption="单人旅程中死亡时自动消耗一个：旅程不会结束，复活后自动返回死亡的房间重新挑战，生命与圣杯瓶恢复到进入本层时的状态。\n每次旅程开始时获得两个，旅程结束时收回；联机旅程不能使用。\n\n失去赐福指引的褪色者，曾在黄金树下徘徊不去。\n那份不肯熄灭的余光，让倒下的人再一次站起。";

    static void Set(PARAM.Row row,string name,int value)=>row[name].Value=Convert.ChangeType(value,row[name].Value.GetType());
    /// 在道具表里加徘徊赐福这一行（照强化石：模板为道具 8000）
    public static void Add(PARAM goods) {
        if(goods.Rows.Any(r=>r.ID==Item))throw new Exception("Retry item ID conflict");
        if(Item>=0x800000)throw new Exception("Goods ID overlaps the indexed gaitem handle bit");
        var row=new PARAM.Row(goods.Rows.Single(r=>r.ID==8000)){ID=Item,Name="Tower lingering grace"};
        foreach(var name in new[]{"refId_default","sellValue"})Set(row,name,-1);
        foreach(var name in new[]{"goodsType","isDiscard","isDeposit","isDrop"})Set(row,name,0);
        Set(row,"maxNum",Max);
        Set(row,"iconId",Convert.ToInt32(goods.Rows.Single(r=>r.ID==IconSource)["iconId"].Value));
        foreach(var cell in row.Cells.Where(c=>c.Def.InternalName.StartsWith("enable_")))cell.Value=Convert.ChangeType(0,cell.Value.GetType());
        goods.Rows.Add(row);goods.Rows.Sort((a,b)=>a.ID.CompareTo(b.ID));
    }
    public static void Verify(PARAM goods) {
        var row=goods.Rows.SingleOrDefault(r=>r.ID==Item)??throw new Exception("Retry item missing");
        if(Convert.ToInt32(row["maxNum"].Value)!=Max||Convert.ToInt32(row["isDiscard"].Value)!=0||Convert.ToInt32(row["isDeposit"].Value)!=0||Convert.ToInt32(row["isDrop"].Value)!=0)throw new Exception("Retry item can escape the run");
        if(Convert.ToInt32(row["iconId"].Value)!=Convert.ToInt32(goods.Rows.Single(r=>r.ID==IconSource)["iconId"].Value))throw new Exception("Retry item icon not borrowed from Rune Arc");
        if(row.Cells.Where(c=>c.Def.InternalName.StartsWith("enable_")).Any(c=>Convert.ToInt32(c.Value)!=0))throw new Exception("Retry item must not be usable from the item bar");
    }
    /// 完整构建：加物品行与文字
    public static void Build(Func<string,PARAM> read,Action<string,PARAM> save,BND4 item) {
        var goods=read("EquipParamGoods");Add(goods);Verify(goods);save("EquipParamGoods",goods);
        Text(item);
    }
    static void Text(BND4 item) {
        foreach(var (table,text) in new[]{(10,Name),(20,Info),(24,Caption)}) {
            var file=item.Files.Single(f=>f.ID==table);var fmg=FMG.Read(file.Bytes);
            fmg.Entries.RemoveAll(e=>e.ID==Item);fmg.Entries.Add(new FMG.Entry(Item,text));fmg.Entries.Sort((a,b)=>a.ID.CompareTo(b.ID));file.Bytes=fmg.Write();
        }
    }

    /// 已打过补丁的 regulation 上只加这一行，其他参数一个字节都不改（Linux 上也能运行）
    public static void Patch(string input,string output) {
        if(Path.GetFullPath(input)==Path.GetFullPath(output))throw new Exception("Separate staging output required");
        var reg=SoulsFormats.Cryptography.RegulationDecryptor.DecryptERRegulation(Path.Combine(input,"regulation.bin"));
        var file=reg.Files.Single(f=>Path.GetFileNameWithoutExtension(f.Name.Replace('\\','/'))=="EquipParamGoods");
        var def=PARAMDEF.XmlDeserialize("work/SoulsRandomizers/diste/Defs/EquipParamGoods.xml");
        var goods=PARAM.Read(file.Bytes);goods.ApplyParamdef(def);var original=PARAM.Read(file.Bytes);original.ApplyParamdef(def);
        if(goods.Rows.Any(r=>r.ID==Item)){Console.WriteLine("Retry item already present; regulation unchanged");return;}
        Add(goods);file.Bytes=goods.Write();
        var others=reg.Files.Where(f=>f!=file).ToDictionary(f=>f.Name,f=>f.Bytes.ToArray());
        Directory.CreateDirectory(output);var path=Path.Combine(output,"regulation.bin");
        SoulsFormats.Cryptography.RegulationDecryptor.EncryptERRegulation(path,reg);
        var check=SoulsFormats.Cryptography.RegulationDecryptor.DecryptERRegulation(path);
        foreach(var f in check.Files.Where(f=>f.Name!=file.Name))if(!f.Bytes.SequenceEqual(others[f.Name]))throw new Exception($"Unrelated param changed: {f.Name}");
        var actual=PARAM.Read(check.Files.Single(f=>f.Name==file.Name).Bytes);actual.ApplyParamdef(def);var rows=actual.Rows.ToDictionary(r=>r.ID);
        foreach(var old in original.Rows)foreach(var cell in old.Cells)
            if(!System.Collections.StructuralComparisons.StructuralEqualityComparer.Equals(cell.Value,rows[old.ID][cell.Def.InternalName].Value))throw new Exception($"Unexpected goods change: {old.ID} {cell.Def.InternalName}");
        if(actual.Rows.Count!=original.Rows.Count+1)throw new Exception("Expected exactly one new goods row");
        Verify(actual);
        Console.WriteLine($"Lingering grace item {Item}: icon {rows[Item]["iconId"].Value} (from Rune Arc), maxNum {Max}; other params unchanged");
    }
    /// 名字与说明（需要游戏文字，在 Windows 上运行）：item_dlc02 的 10 / 20 / 24 表各加一条，其他文字不变
    public static void PatchText(string input,string output) {
        const string name="msg/zhocn/item_dlc02.msgbnd.dcx";
        var item=BND4.Read(Path.Combine(input,name));var original=BND4.Read(Path.Combine(input,name));
        Text(item);
        var path=Path.Combine(output,name);Directory.CreateDirectory(Path.GetDirectoryName(path)!);item.Write(path);
        var actual=BND4.Read(path);
        foreach(var file in original.Files) {
            var before=FMG.Read(file.Bytes).Entries;var after=FMG.Read(actual.Files.Single(f=>f.ID==file.ID).Bytes).Entries.ToDictionary(e=>e.ID,e=>e.Text);
            foreach(var entry in before.Where(e=>e.ID!=Item))if(!after.TryGetValue(entry.ID,out var text)||text!=entry.Text)throw new Exception($"Unrelated item text changed: {file.ID}/{entry.ID}");
        }
        foreach(var (table,text) in new[]{(10,Name),(20,Info),(24,Caption)})
            if(FMG.Read(actual.Files.Single(f=>f.ID==table).Bytes).Entries.Single(e=>e.ID==Item).Text!=text)throw new Exception($"Retry item text {table} missing");
        Console.WriteLine("Lingering grace name and description written; other item texts unchanged");
    }
}
