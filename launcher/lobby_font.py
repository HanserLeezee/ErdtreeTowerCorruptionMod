"""联机大厅（法环风）用的宋体子集：从 scene-assets/fonts 的思源宋体子集里，按 launcher/ui/js/app.js 大厅部分出现的字
再裁一次，写到 launcher/ui/assets/fonts（几十 KB）。房间名等动态文字缺字时退回系统宋体。
用法：PYTHONPATH=<fontTools> python3 launcher/lobby_font.py"""
import pathlib, re
from fontTools import subset
root = pathlib.Path(__file__).resolve().parent.parent
src = (root / "launcher/ui/js/app.js").read_text(encoding="utf-8")
start = src.index("/* ================= 联机大厅"); end = src.index("document.addEventListener('keydown'", start)
text = src[start:end] + src[src.index("var steamReminder"):src.index("\n", src.index("var steamReminder"))]
chars = sorted(set(c for c in text if ord(c) > 0x2e80)) + [chr(c) for c in range(0x20, 0x7f)] + list("·，。：；（）「」…—、？！《》“”0123456789")
out = root / "launcher/ui/assets/fonts"; out.mkdir(parents=True, exist_ok=True)
for name, dest in [("TowerSerif-Regular.otf", "LobbySerif-Regular.otf"), ("TowerTitleSerif-Bold.otf", "LobbySerif-Bold.otf")]:
    options = subset.Options(); options.layout_features = ["*"]; options.name_IDs = ["*"]; options.notdef_outline = True
    font = subset.load_font(str(root / "scene-assets/fonts" / name), options)
    s = subset.Subsetter(options); s.populate(text="".join(set(chars))); s.subset(font)
    subset.save_font(font, str(out / dest), options)
    print(dest, (out / dest).stat().st_size, "bytes,", len(set(chars)), "chars")
