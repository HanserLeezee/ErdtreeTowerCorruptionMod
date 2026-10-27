"""Boss health bar names in the HUD (01_000_fe.gfx): colour affix words and add a soft glint, built from a locally extracted menu with FFDec.

The bar name field is BossList.Item_N_0.Name.Text; the HUD has no document class, so the effect runs from the class bound to each bar's
red fill (Current, _01_000_FE_fla.BossFE_Bar_Ani_40) and reaches the name two levels up. Bar names written by text-update look like
"狂暴的·黄金律法的 “恶兆妖鬼”玛尔基特": affix words joined by "·", one space, then the boss name. Only a prefix whose every word is a known affix
(src/boss_affixes.rs Affix::name) is coloured; colours follow native/src/fancy_text.rs affix_style. Usage:
python scene-builder/native_boss_bar_glow.py --ffdec <ffdec.jar> --original <dir with 01_000_fe.gfx> --output <new dir> [--java <java>]
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

STEM = '01_000_fe'
PACKAGE = '_01_000_FE_fla'
CLASS = 'BossFE_Bar_Ani_40'
# (affix name, colour, accent) in Affix::ALL order; accents are the lighter tone the colour flows towards
AFFIXES = [
    ('坚毅的', 0xC4B696, 0xF4E2B2), ('铁壁的', 0x8CACCC, 0xE4F0FF), ('狂暴的', 0xE84834, 0xFFAA5A),
    ('再生的', 0x70CC7C, 0xCCFFC4), ('腐败的', 0xCE523E, 0xF49260), ('嫉妒的', 0xA060D6, 0xD2AAF0),
    ('傲慢的', 0xF4EEE0, 0xFFD68C), ('黄金律法的', 0xF2C85C, 0xFFF8D2), ('涅槃的', 0xF4963E, 0xFFE080),
    ('吸血的', 0xCC2036, 0xFF6074), ('荆棘的', 0xAC2A2A, 0xE86054), ('迅猛的', 0x5CDECC, 0xCCFFF6),
]


def escape(text):
    """AS3 string literal with every non-ASCII character escaped, so the compiler never sees raw CJK."""
    return '"' + ''.join(c if c.isascii() and c not in '"\\' else f'\\u{ord(c):04X}' for c in text) + '"'


def members():
    names = ','.join(escape(name) for name, _, _ in AFFIXES)
    colors = ','.join(f'0x{color:06X}' for _, color, _ in AFFIXES)
    accents = ','.join(f'0x{accent:06X}' for _, _, accent in AFFIXES)
    return f'''
      private var towerItemField:flash.text.TextField;
      private var towerItemText:String = "";
      private var towerItemKinds:Array = [];
      private var towerItemX:Array = [];
      private var towerItemColor:uint = 0xFFFFFF;
      private var towerItemFilters:Array;
      private var towerItemGlow:flash.filters.GlowFilter;
      private var towerItemRim:flash.filters.GlowFilter;
      private var towerItemPlume:flash.filters.DropShadowFilter;
      private var towerItemTint:uint = 0xE0B050;
      private var towerItemFormat:flash.text.TextFormat = new flash.text.TextFormat();
      private var towerItemError:String = "";
      private var towerAffixNames:Array = [{names}];
      private var towerAffixColors:Array = [{colors}];
      private var towerAffixAccents:Array = [{accents}];

      private function towerBarMix(a:uint, b:uint, k:Number) : uint
      {{
         if(k < 0) k = 0;
         if(k > 1) k = 1;
         var red:int = int(((a >> 16) & 255) + (((b >> 16) & 255) - ((a >> 16) & 255)) * k);
         var green:int = int(((a >> 8) & 255) + (((b >> 8) & 255) - ((a >> 8) & 255)) * k);
         var blue:int = int((a & 255) + ((b & 255) - (a & 255)) * k);
         return uint((red << 16) | (green << 8) | blue);
      }}

      // 拆出词条前缀：空格前的每一段都是已知词条才上色；每个字记下类别（词条序号、-1 Boss 名、-2 分隔符）与中心位置
      private function towerBarParse(field:flash.text.TextField) : void
      {{
         var text:String = field.text;
         this.towerItemText = text;
         this.towerItemKinds = [];
         this.towerItemX = [];
         var space:int = text.indexOf(" ");
         var tokens:Array = space > 0 ? text.substr(0,space).split("\\u00B7") : [];
         var valid:Boolean = tokens.length > 0;
         for(var check:int = 0; check < tokens.length; check++)
         {{
            if(this.towerAffixNames.indexOf(tokens[check]) < 0) valid = false;
         }}
         var token:int = 0;
         for(var index:int = 0; index < text.length; index++)
         {{
            var kind:int = -1;
            if(valid && index <= space)
            {{
               var code:Number = text.charCodeAt(index);
               if(code == 183 || code == 32)
               {{
                  kind = -2;
                  if(code == 183) token++;
               }}
               else kind = this.towerAffixNames.indexOf(tokens[token]);
            }}
            this.towerItemKinds.push(kind);
            var x:Number = index * 24;
            try
            {{
               var bounds:flash.geom.Rectangle = field.getCharBoundaries(index);
               if(bounds != null && bounds.width > 0) x = bounds.x + bounds.width * 0.5;
            }}
            catch(boundsError:*)
            {{
            }}
            this.towerItemX.push(x);
         }}
         this.towerItemTint = valid ? uint(this.towerAffixColors[this.towerAffixNames.indexOf(tokens[0])]) : 0xE0B050;
      }}

      // 每帧：词条按各自颜色在主色与亮色之间流动；Boss 名在原色与金色之间沿字流动，并有一道白金扫光；
      // 整行加细金边、随呼吸明暗的外圈柔光与向上飘的光焰（柔光与光焰用第一个词条的颜色，没有词条时为暖金）
      private function towerBarUpdate(event:flash.events.Event) : void
      {{
         try
         {{
            var item:flash.display.DisplayObjectContainer = this.parent == null ? null : this.parent.parent as flash.display.DisplayObjectContainer;
            if(item == null) return;
            var holder:flash.display.DisplayObjectContainer = item.getChildByName("Name") as flash.display.DisplayObjectContainer;
            if(holder == null) return;
            var field:flash.text.TextField = holder.getChildByName("Text") as flash.text.TextField;
            if(field == null) return;
            if(this.towerItemField != field)
            {{
               this.towerItemField = field;
               this.towerItemColor = field.textColor;
               this.towerItemFilters = field.filters;
               this.towerItemText = "";
               this.towerItemKinds = [];
            }}
            var text:String = field.text;
            if(text.length == 0 || !item.visible) return;
            if(text != this.towerItemText || this.towerItemKinds.length != text.length) this.towerBarParse(field);
            var time:Number = flash.utils.getTimer() / 1000;
            var breath:Number = 0.5 + 0.5 * Math.sin(time * 2.2);
            for(var index:int = 0; index < text.length; index++)
            {{
               var kind:int = int(this.towerItemKinds[index]);
               var color:uint = this.towerItemColor;
               if(kind == -2) color = 0x968C7E;
               else if(kind >= 0) color = this.towerBarMix(uint(this.towerAffixColors[kind]),uint(this.towerAffixAccents[kind]),0.5 + 0.5 * Math.sin(time * 2.2 - index * 0.5));
               else
               {{
                  color = this.towerBarMix(this.towerItemColor,0xF2D27E,0.3 + 0.3 * Math.sin(time * 1.6 - index * 0.35));
                  var shine:Number = Math.pow((Math.sin(0.011 * Number(this.towerItemX[index]) - 1.4 * time) + 1) * 0.5,24) * 0.9;
                  color = this.towerBarMix(color,0xFFFAE6,shine);
               }}
               this.towerItemFormat.color = color;
               field.setTextFormat(this.towerItemFormat,index,index + 1);
            }}
            var flick:Number = 0.5 + 0.5 * Math.sin(time * 8.3) * Math.sin(time * 4.7 + 1.1);
            if(this.towerItemRim == null) this.towerItemRim = new flash.filters.GlowFilter(0xD8A848,0.85,2.5,2.5,3,2,false,false);
            if(this.towerItemPlume == null) this.towerItemPlume = new flash.filters.DropShadowFilter(3,270,this.towerItemTint,0.35,2,8,1.2,2,false,false,false);
            if(this.towerItemGlow == null) this.towerItemGlow = new flash.filters.GlowFilter(this.towerItemTint,0.4,10,10,1.3,2,false,false);
            this.towerItemPlume.color = this.towerItemTint;
            this.towerItemPlume.distance = 2 + 2.5 * flick;
            this.towerItemPlume.blurY = 6 + 4 * flick;
            this.towerItemGlow.color = this.towerItemTint;
            this.towerItemGlow.alpha = 0.3 + 0.3 * breath;
            this.towerItemGlow.blurX = this.towerItemGlow.blurY = 8 + 4 * breath;
            field.filters = (this.towerItemFilters == null ? [] : this.towerItemFilters).concat([this.towerItemRim,this.towerItemPlume,this.towerItemGlow]);
            this.towerItemError = "";
         }}
         catch(error:*)
         {{
            this.towerItemError = String(error);
         }}
      }}
'''


def patch_source(source):
    anchor = f'      public function {CLASS}()'
    frame = 'addFrameScript(0,this.frame1);'
    if source.count(anchor) != 1 or source.count(frame) != 1 or 'towerBarUpdate' in source:
        raise ValueError('Unexpected boss bar class; refusing to patch')
    source = source.replace(anchor, members() + '\n' + anchor)
    return source.replace(frame, frame + '\n         addEventListener(flash.events.Event.ENTER_FRAME,this.towerBarUpdate);')


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
    with tempfile.TemporaryDirectory(prefix='tower-hud-') as tmp:
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
        if 'towerBarUpdate' not in actual or 'GlowFilter' not in actual or 'towerAffixNames' not in actual:
            raise ValueError('Compiled boss bar effect missing')
        (output / (STEM + '.as')).write_text(actual, encoding='utf-8')
    manifest = {'experimental': True, 'gameVerified': False, 'class': f'{PACKAGE}.{CLASS}', 'field': 'BossList.Item_N_0.Name.Text',
                'effect': 'affix words coloured per affix and flowing towards a lighter tone; boss name flows between its colour and gold with a bright white-gold glint; thin gold rim, upward flickering plume and breathing outer glow tinted by the first affix',
                'files': [{'file': 'menu/' + src.name, 'originalSha256': hashlib.sha256(src.read_bytes()).hexdigest(),
                           'patchedSha256': hashlib.sha256(dst.read_bytes()).hexdigest(), 'tagsVerified': count, 'changedTag': 'DoABC'}]}
    (output / 'manifest.json').write_text(json.dumps(manifest, ensure_ascii=False, indent=2), encoding='utf-8')
    print('Built and structurally verified the boss bar name effect.')


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--ffdec', type=Path, required=True)
    parser.add_argument('--original', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--java', default='java')
    args = parser.parse_args()
    build(args.ffdec.resolve(), args.original.resolve(), args.output.resolve(), args.java)
