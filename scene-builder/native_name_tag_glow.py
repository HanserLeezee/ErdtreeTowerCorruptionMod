"""Player name tags in the HUD (01_000_fe.gfx): give the title in front of a teammate's name its own animated look.

The friendly tags are RemotePCTag0..5 (name, role, HP bar; the same tag moves to the left edge when the teammate is off screen).
The DLL (native/src/name_tags.rs) writes the name as "称号·名字" for teammates wearing a title (src/titles.rs). The name field is
RemotePCTagN.Name.Text_0.Text and its clip class, _01_000_FE_fla.EnemyTag_ColorText_14, is shared with enemy tags and role names, so the
effect only runs for the instance named "Name" directly under a RemotePCTag. Only a known title before the first "·" is styled; the player's
own name keeps the game's colour. Usage:
python scene-builder/native_name_tag_glow.py --ffdec <ffdec.jar> --original <dir with 01_000_fe.gfx> --output <new dir> [--java <java>]
"""
import argparse
import hashlib
import json
import subprocess
import sys
import tempfile
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import native_name_glow as glow  # noqa: E402  (shared GFx tag reader and structural checks)
from native_boss_bar_glow import escape  # noqa: E402

STEM = '01_000_fe'
PACKAGE = '_01_000_FE_fla'
CLASS = 'EnemyTag_ColorText_14'
# 名字特效，与 src/titles.rs 的 Style::ALL 同序：(名称, 主色, 亮色, 动画)
# 动画：0 呼吸，1 扫光，2 波动，3 抖动闪烁，4 彩虹渐变
STYLES = [
    ('灰烬', 0xB8B0A0, 0xEDE6D6, 0), ('黄金流光', 0xE8B84C, 0xFFF4C8, 1), ('余火', 0xE86A2C, 0xFFC070, 2),
    ('鲜血', 0xB01E2E, 0xFF5A6A, 3), ('星辉', 0x8C7CFF, 0x7CF0FF, 4), ('寒霜', 0x6CB8F0, 0xE0F6FF, 0), ('月光', 0x9CA8D8, 0xF0F2FF, 1),
]
# 称号与所用特效（src/titles.rs 的 TITLES：文字、Style 序号）
TITLES = [('褪色者', 0), ('登顶者', 1), ('艰难之王', 2), ('长夜之王', 3), ('危机之王', 4), ('百战之身', 5), ('孤狼', 6)]


def members():
    titles = ','.join(escape(text) for text, _ in TITLES)
    kinds = ','.join(str(style) for _, style in TITLES)
    colors = ','.join(f'0x{c:06X}' for _, c, _, _ in STYLES)
    accents = ','.join(f'0x{a:06X}' for _, _, a, _ in STYLES)
    motions = ','.join(str(m) for _, _, _, m in STYLES)
    return f'''
      private var towerTagField:flash.text.TextField;
      private var towerTagText:String = "";
      private var towerTagStyle:int = -1;
      private var towerTagLength:int = 0;
      private var towerTagColor:uint = 0xFFFFFF;
      private var towerTagFilters:Array;
      private var towerTagGlow:flash.filters.GlowFilter;
      private var towerTagFormat:flash.text.TextFormat = new flash.text.TextFormat();
      private var towerTagFrame:Number = 0;
      private var towerTagError:String = "";
      private var towerTitleNames:Array = [{titles}];
      private var towerTitleStyles:Array = [{kinds}];
      private var towerStyleColors:Array = [{colors}];
      private var towerStyleAccents:Array = [{accents}];
      private var towerStyleMotions:Array = [{motions}];

      private function towerTagMix(a:uint, b:uint, k:Number) : uint
      {{
         if(k < 0) k = 0;
         if(k > 1) k = 1;
         var red:int = int(((a >> 16) & 255) + (((b >> 16) & 255) - ((a >> 16) & 255)) * k);
         var green:int = int(((a >> 8) & 255) + (((b >> 8) & 255) - ((a >> 8) & 255)) * k);
         var blue:int = int((a & 255) + ((b & 255) - (a & 255)) * k);
         return uint((red << 16) | (green << 8) | blue);
      }}

      private function towerTagHue(h:Number) : uint
      {{
         h = h - Math.floor(h);
         var r:Number = Math.abs(h * 6 - 3) - 1;
         var g:Number = 2 - Math.abs(h * 6 - 2);
         var b:Number = 2 - Math.abs(h * 6 - 4);
         var red:int = int(Math.max(0,Math.min(1,r)) * 155 + 100);
         var green:int = int(Math.max(0,Math.min(1,g)) * 155 + 100);
         var blue:int = int(Math.max(0,Math.min(1,b)) * 155 + 100);
         return uint((red << 16) | (green << 8) | blue);
      }}

      // 只处理队友名牌的名字（RemotePCTagN 下名为 Name 的实例）；“称号·名字”里的称号按所属特效逐字着色，名字保持原色
      private function towerTagUpdate(event:flash.events.Event) : void
      {{
         try
         {{
            if(this.name != "Name" || this.parent == null || this.parent.name.indexOf("RemotePCTag") != 0) return;
            var holder:flash.display.DisplayObjectContainer = this.getChildByName("Text_0") as flash.display.DisplayObjectContainer;
            if(holder == null) return;
            var field:flash.text.TextField = holder.getChildByName("Text") as flash.text.TextField;
            if(field == null) return;
            if(this.towerTagField != field)
            {{
               this.towerTagField = field;
               this.towerTagColor = field.textColor;
               this.towerTagFilters = field.filters;
               this.towerTagText = "";
            }}
            var text:String = field.text;
            if(text != this.towerTagText)
            {{
               this.towerTagText = text;
               var dot:int = text.indexOf("\\u00B7");
               var index:int = dot > 0 ? this.towerTitleNames.indexOf(text.substr(0,dot)) : -1;
               this.towerTagStyle = index < 0 ? -1 : int(this.towerTitleStyles[index]);
               this.towerTagLength = index < 0 ? 0 : dot;
               this.towerTagFormat.color = this.towerTagColor;
               if(text.length > 0) field.setTextFormat(this.towerTagFormat,0,text.length);
               if(this.towerTagStyle < 0) field.filters = this.towerTagFilters == null ? [] : this.towerTagFilters;
            }}
            if(this.towerTagStyle < 0 || !this.visible) return;
            var style:int = this.towerTagStyle;
            var main:uint = uint(this.towerStyleColors[style]);
            var accent:uint = uint(this.towerStyleAccents[style]);
            var motion:int = int(this.towerStyleMotions[style]);
            var time:Number = (++this.towerTagFrame) / 30;
            var breath:Number = 0.5 + 0.5 * Math.sin(time * 2.2);
            for(var i:int = 0; i < this.towerTagLength; i++)
            {{
               var k:Number = breath;
               if(motion == 1) k = Math.pow((Math.sin(i * 0.9 - time * 2.6) + 1) * 0.5,6);
               else if(motion == 2) k = 0.5 + 0.5 * Math.sin(time * 3.1 - i * 0.8);
               else if(motion == 3) k = 0.35 + 0.65 * Math.abs(Math.sin(time * 7.3 + i * 1.7) * Math.sin(time * 3.1));
               var color:uint = motion == 4 ? this.towerTagMix(this.towerTagHue(time * 0.15 + i * 0.12),accent,0.25) : this.towerTagMix(main,accent,k);
               this.towerTagFormat.color = color;
               field.setTextFormat(this.towerTagFormat,i,i + 1);
            }}
            this.towerTagFormat.color = 0x968C7E;
            field.setTextFormat(this.towerTagFormat,this.towerTagLength,this.towerTagLength + 1);
            if(this.towerTagGlow == null) this.towerTagGlow = new flash.filters.GlowFilter(main,0.5,8,8,1.4,2,false,false);
            this.towerTagGlow.color = motion == 4 ? this.towerTagHue(time * 0.15) : main;
            this.towerTagGlow.alpha = 0.3 + 0.35 * breath;
            this.towerTagGlow.blurX = this.towerTagGlow.blurY = 6 + 4 * breath;
            field.filters = (this.towerTagFilters == null ? [] : this.towerTagFilters).concat([this.towerTagGlow]);
            this.towerTagError = "";
         }}
         catch(error:*)
         {{
            this.towerTagError = String(error);
         }}
      }}
'''


def patch_source(source):
    anchor = f'      public function {CLASS}()'
    frame = 'addFrameScript(0,this.frame1);'
    if source.count(anchor) != 1 or source.count(frame) != 1 or 'towerTagUpdate' in source:
        raise ValueError('Unexpected name tag class; refusing to patch')
    source = source.replace(anchor, members() + '\n' + anchor)
    return source.replace(frame, frame + '\n         addEventListener(flash.events.Event.ENTER_FRAME,this.towerTagUpdate);')


def build(ffdec, original, output, java='java'):
    output.mkdir(parents=True, exist_ok=False)
    (output / 'menu').mkdir()

    def run(*args):
        result = subprocess.run([java, '-Djava.awt.headless=true', '-jar', str(ffdec), *map(str, args)], capture_output=True, text=True, encoding='utf-8', errors='replace')
        with (output / 'build.log').open('a', encoding='utf-8') as log:
            log.write(result.stdout + result.stderr)
        if result.returncode:
            raise RuntimeError('FFDec failed; see build.log')

    src = original / (STEM + '.gfx')
    dst = output / 'menu' / src.name
    with tempfile.TemporaryDirectory(prefix='tower-tag-') as tmp:
        tmp = Path(tmp)
        run('-export', 'script', tmp / 'before', src)
        relative = Path(PACKAGE) / (CLASS + '.as')
        script = tmp / 'before' / 'scripts' / relative
        changed = tmp / 'changed' / relative
        changed.parent.mkdir(parents=True)
        changed.write_text(patch_source(script.read_text(encoding='utf-8-sig')), encoding='utf-8')
        run('-importScript', src, dst, tmp / 'changed')
        count = glow.verify_tags(src.read_bytes(), dst.read_bytes())
        run('-export', 'script', tmp / 'after', dst)
        for before in (tmp / 'before' / 'scripts').rglob('*.as'):
            rel = before.relative_to(tmp / 'before' / 'scripts')
            after = tmp / 'after' / 'scripts' / rel
            if rel != relative and before.read_bytes() != after.read_bytes():
                raise ValueError(f'Unrelated script changed: {rel}')
        actual = (tmp / 'after' / 'scripts' / relative).read_text(encoding='utf-8-sig')
        if 'towerTagUpdate' not in actual or 'towerTitleNames' not in actual or 'GlowFilter' not in actual:
            raise ValueError('Compiled name tag effect missing')
        (output / (STEM + '-name-tag.as')).write_text(actual, encoding='utf-8')
    manifest = {'experimental': True, 'gameVerified': False, 'class': f'{PACKAGE}.{CLASS}', 'field': 'RemotePCTagN.Name.Text_0.Text',
                'effect': 'title before the first "·" coloured per title style (breathing, glint, wave, flicker or rainbow) with a tinted glow; the name keeps its colour',
                'files': [{'file': 'menu/' + src.name, 'baseSha256': hashlib.sha256(src.read_bytes()).hexdigest(),
                           'patchedSha256': hashlib.sha256(dst.read_bytes()).hexdigest(), 'tagsVerified': count, 'changedTag': 'DoABC'}]}
    (output / 'manifest.json').write_text(json.dumps(manifest, ensure_ascii=False, indent=2), encoding='utf-8')
    print('Built and structurally verified the player name tag effect.')


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--ffdec', type=Path, required=True)
    parser.add_argument('--original', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--java', default='java')
    args = parser.parse_args()
    build(args.ffdec.resolve(), args.original.resolve(), args.output.resolve(), args.java)
