"""标题字体：从思源宋体（Noto Serif CJK SC Bold）裁出界面会用到的字，供带特效的标题文字使用。

用法：python scene-builder/title_font.py <NotoSerifCJK-Bold.ttc> <仓库根目录>
需要 fontTools。保留 GB2312 的全部汉字与符号，再加上仓库里文本资源和代码中出现的所有字符（物品名、词条、界面文字），
写出 scene-assets/fonts/TowerTitleSerif-Bold.otf 与界面字表 ui-glyphs.txt（游戏只加载字表里的字形）。字体按 SIL Open Font License 1.1 授权；裁剪属于修改，按许可改用新的字体名
“Tower Title Serif”，许可全文见 licenses/NotoSerifCJK-OFL.txt。
"""
from pathlib import Path
import sys
from fontTools.ttLib import TTCollection
from fontTools.subset import Options, Subsetter

FAMILY = "Tower Title Serif"
source, root = Path(sys.argv[1]), Path(sys.argv[2])
collection = TTCollection(str(source))
font = next(f for f in collection.fonts if f["name"].getDebugName(4) == "Noto Serif CJK SC Bold")

chars = set(chr(c) for c in range(0x20, 0x7F))
# GB2312：常用汉字与全角符号
for high in range(0xA1, 0xF8):
    for low in range(0xA1, 0xFF):
        try:
            chars.add(bytes([high, low]).decode("gb2312"))
        except UnicodeDecodeError:
            pass
# 仓库里的文本资源与代码中出现的所有字符（包括生僻的物品名用字）
for pattern in ["scene-assets/*.txt", "scene-assets/*.tsv", "src/*.rs", "native/src/*.rs"]:
    for path in root.glob(pattern):
        chars.update(path.read_text(encoding="utf-8", errors="ignore"))
chars = {c for c in chars if c.isprintable() and c not in "\r\n\t"}

options = Options()
options.layout_features = ["*"]
options.name_IDs = ["*"]
options.notdef_outline = True
subsetter = Subsetter(options)
subsetter.populate(unicodes=[ord(c) for c in chars])
subsetter.subset(font)

# 改名：OFL 要求修改后的版本不使用原字体名
names = font["name"]
for record in names.names:
    if record.nameID in (1, 16):
        record.string = FAMILY
    elif record.nameID == 4:
        record.string = f"{FAMILY} Bold"
    elif record.nameID == 6:
        record.string = "TowerTitleSerif-Bold"
    elif record.nameID == 3:
        record.string = f"{FAMILY} Bold; subset of Noto Serif CJK SC Bold"
out = root / "scene-assets" / "fonts" / "TowerTitleSerif-Bold.otf"
out.parent.mkdir(parents=True, exist_ok=True)
font.save(str(out))
# 界面字表：游戏内按这份字表只加载用得到的字形（正文的黑体与标题宋体都用它），避免字体贴图过大
glyphs = sorted(c for c in chars if ord(c) >= 0x80)
(out.parent / "ui-glyphs.txt").write_text("".join(glyphs), encoding="utf-8")
covered = sum(1 for c in chars if ord(c) in font.getBestCmap())
print(f"Wrote {out.name}: {covered} of {len(chars)} requested characters, {out.stat().st_size / 1024 / 1024:.1f} MB")
