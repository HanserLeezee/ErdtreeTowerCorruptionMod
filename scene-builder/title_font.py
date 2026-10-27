"""界面字体：从思源宋体（Noto Serif CJK SC）裁出界面会用到的字。常规体用于正文，粗体用于标题与带特效的文字。

用法：python scene-builder/title_font.py <NotoSerifCJK-Regular.ttc> <NotoSerifCJK-Bold.ttc> <仓库根目录>
需要 fontTools。保留 GB2312 的全部汉字与符号，再加上仓库里文本资源和代码中出现的所有字符（物品名、词条、界面文字），
写出 scene-assets/fonts/TowerSerif-Regular.otf、TowerTitleSerif-Bold.otf 与界面字表 ui-glyphs.txt（游戏只加载字表里的字形）。
字体按 SIL Open Font License 1.1 授权；裁剪属于修改，按许可改用新的字体名“Tower Serif”“Tower Title Serif”，许可全文见 licenses/NotoSerifCJK-OFL.txt。
"""
from pathlib import Path
import sys
from fontTools.ttLib import TTCollection
from fontTools.subset import Options, Subsetter

regular_source, bold_source, root = Path(sys.argv[1]), Path(sys.argv[2]), Path(sys.argv[3])

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
        if path.name == "ui-glyphs.txt":
            continue
        chars.update(path.read_text(encoding="utf-8", errors="ignore"))
# 全角空格（U+3000）在 Python 里不算可打印字符，但界面用它分隔，要单独保留
chars = {c for c in chars if (c.isprintable() or c == "\u3000") and c not in "\r\n\t"}
out_dir = root / "scene-assets" / "fonts"
out_dir.mkdir(parents=True, exist_ok=True)

def subset(source, face, family, style, file_name):
    collection = TTCollection(str(source))
    font = next(f for f in collection.fonts if f["name"].getDebugName(4) == face)
    options = Options()
    options.layout_features = ["*"]
    options.name_IDs = ["*"]
    options.notdef_outline = True
    subsetter = Subsetter(options)
    subsetter.populate(unicodes=[ord(c) for c in chars])
    subsetter.subset(font)
    # 改名：OFL 要求修改后的版本不使用原字体名
    for record in font["name"].names:
        if record.nameID in (1, 16):
            record.string = family
        elif record.nameID == 4:
            record.string = f"{family} {style}"
        elif record.nameID == 6:
            record.string = f"{family.replace(' ', '')}-{style}"
        elif record.nameID == 3:
            record.string = f"{family} {style}; subset of {face}"
    out = out_dir / file_name
    font.save(str(out))
    covered = sum(1 for c in chars if ord(c) in font.getBestCmap())
    print(f"Wrote {out.name}: {covered} of {len(chars)} requested characters, {out.stat().st_size / 1024 / 1024:.1f} MB")

subset(regular_source, "Noto Serif CJK SC", "Tower Serif", "Regular", "TowerSerif-Regular.otf")
subset(bold_source, "Noto Serif CJK SC Bold", "Tower Title Serif", "Bold", "TowerTitleSerif-Bold.otf")
# 界面字表：游戏内按这份字表只加载用得到的字形（正文与标题都用它），避免字体贴图过大
glyphs = sorted(c for c in chars if ord(c) >= 0x80)
(out_dir / "ui-glyphs.txt").write_text("".join(glyphs), encoding="utf-8")
