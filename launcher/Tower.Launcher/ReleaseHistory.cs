using System;
using System.Collections.Generic;
using System.Linq;
using System.Text.Json;

namespace Tower.Launcher
{
    sealed class HistoryEntry
    {
        public string Tag { get; set; } = "";
        public string Title { get; set; } = "";
        public string Notes { get; set; } = "";
        public string PublishedAt { get; set; } = "";
    }

    static class ReleaseHistory
    {
        static string Text(JsonElement row,string key) => row.TryGetProperty(key,out var v)&&v.ValueKind==JsonValueKind.String?v.GetString()??"":"";
        public static List<HistoryEntry> ParseGitHub(string json)
        {
            using var document=JsonDocument.Parse(json);
            var rows=new List<HistoryEntry>();
            foreach(var row in document.RootElement.EnumerateArray())
            {
                if(row.ValueKind!=JsonValueKind.Object||(row.TryGetProperty("draft",out var draft)&&draft.ValueKind==JsonValueKind.True))continue;
                var tag=Text(row,"tag_name");if(tag.Length==0)continue;
                var title=Text(row,"name");
                rows.Add(new HistoryEntry{Tag=tag,Title=title.Length==0?tag:title,Notes=Text(row,"body"),PublishedAt=Text(row,"published_at")});
            }
            return rows;
        }
        public static List<HistoryEntry> Merge(IEnumerable<HistoryEntry> saved,IEnumerable<HistoryEntry> fresh) =>
            saved.Concat(fresh).Where(r=>r.Tag.Length>0).GroupBy(r=>r.Tag).Select(g=>g.Last()).OrderByDescending(r=>r.PublishedAt,StringComparer.Ordinal).ToList();
    }
}
