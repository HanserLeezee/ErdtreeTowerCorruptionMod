using Tower.Launcher;
using Xunit;

namespace Tower.Update.Tests;

public sealed class HistoryAndLogsTests
{
    [Fact]
    public void History_excludes_drafts_preserves_notes_and_merges_by_tag()
    {
        var rows=ReleaseHistory.ParseGitHub("""
        [{"tag_name":"v2","name":"新版","body":"第一行\n<script>test</script>","published_at":"2026-10-30T00:00:00Z","draft":false},
         {"tag_name":"v3","draft":true},{"tag_name":"v1","name":null,"body":null,"published_at":"2026-10-29T00:00:00Z"}]
        """);
        Assert.Equal(2,rows.Count);
        Assert.Equal("第一行\n<script>test</script>",rows[0].Notes);
        Assert.Equal("v1",rows[1].Title);
        var merged=ReleaseHistory.Merge(rows,new[]{new HistoryEntry{Tag="v2",Title="修订",PublishedAt="2026-10-30T00:00:00Z"}});
        Assert.Equal(2,merged.Count);Assert.Equal("修订",merged[0].Title);
    }
    [Fact]
    public void Logs_find_latest_solo_or_multiplayer_file_without_creating_files()
    {
        var root=Path.Combine(Path.GetTempPath(),Guid.NewGuid().ToString("N"));
        Directory.CreateDirectory(root);
        try {
            Assert.Null(ModLogs.Latest(root));
            var solo=Path.Combine(root,"tower-preview.log");File.WriteAllText(solo,"solo");File.SetLastWriteTimeUtc(solo,new DateTime(2026,1,1));
            var party=Path.Combine(root,"seamless-entry","party-runtime","tower-preview.log");Directory.CreateDirectory(Path.GetDirectoryName(party)!);File.WriteAllText(party,"party");
            Assert.Equal(party,ModLogs.Latest(root));File.Delete(party);Assert.Equal(solo,ModLogs.Latest(root));
        } finally {Directory.Delete(root,true);}
    }
}
