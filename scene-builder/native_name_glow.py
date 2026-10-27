"""Build an isolated GFx experiment from locally extracted game menus using FFDec."""
import argparse
import hashlib
import json
import struct
import subprocess
import tempfile
from pathlib import Path

TARGETS = {
    '02_055_detailstatus_weapon1': ('_02_055_DetailStatus_Weapon1_fla', 'Status_Weapon1'),
    '02_052_detailstatus_armor': ('_02_052_DetailStatus_Armor_fla', 'Status_Armor'),
}

MEMBERS = '''
      private var towerName:flash.text.TextField;
      private var towerSavedColor:uint;
      private var towerSavedFilters:Array;
      private var towerGoldFilters:Array;
      private var towerLegendary:Boolean = false;
      private var towerSavedAlpha:Number = 1;
      private var towerPulseFrame:uint = 0;
      private var towerEffectState:String = "idle";
      private var towerEffectError:String = "";
      private var towerFilterCount:int = -1;
      private var towerSparkles:flash.display.Sprite;
      private var towerStyledText:String = "";

      private function towerDecorate(field:flash.text.TextField, phase:Number) : void
      {
         var count:int = field.text.length;
         var sweep:Number = phase * (count + 8) - 4;
         var format:flash.text.TextFormat = new flash.text.TextFormat();
         for(var index:int = 0; index < count; index++)
         {
            var distance:Number = index - sweep;
            if(distance < 0) distance = -distance;
            var shine:Number = distance < 4 ? 1 - distance / 4 : 0;
            format.color = (uint(218 + 37 * shine) << 16) | (uint(157 + 98 * shine) << 8) | uint(40 + 215 * shine);
            field.setTextFormat(format,index,index + 1);
         }
         if(this.towerSparkles == null || this.towerSparkles.parent != field.parent)
         {
            if(this.towerSparkles != null && this.towerSparkles.parent != null) this.towerSparkles.parent.removeChild(this.towerSparkles);
            this.towerSparkles = new flash.display.Sprite();
            this.towerSparkles.mouseEnabled = false;
            this.towerSparkles.mouseChildren = false;
            field.parent.addChild(this.towerSparkles);
         }
         this.towerSparkles.visible = field.visible;
         this.towerSparkles.transform.matrix = field.transform.matrix;
         this.towerSparkles.alpha = this.towerSavedAlpha;
         this.towerSparkles.graphics.clear();
         var width:Number = Math.min(field.width - 4,field.textWidth + 4);
         for(var star:int = 0; star < 7; star++)
         {
            var life:Number = (phase * 2 + star * 0.17) % 1;
            var opacity:Number = life < 0.5 ? life * 2 : (1 - life) * 2;
            opacity *= opacity;
            var x:Number = 3 + width * ((star * 0.17 + 0.07) % 1) + life * 9;
            var y:Number = -1 + (star % 2) * (Math.min(field.textHeight,32) + 7) - life * 5;
            var radius:Number = 2 + opacity * 5;
            this.towerSparkles.graphics.lineStyle(3,0xF4B844,opacity * 0.25);
            this.towerSparkles.graphics.moveTo(x - radius * 1.3,y);
            this.towerSparkles.graphics.lineTo(x + radius * 1.3,y);
            this.towerSparkles.graphics.moveTo(x,y - radius * 1.7);
            this.towerSparkles.graphics.lineTo(x,y + radius * 1.7);
            this.towerSparkles.graphics.lineStyle(1.3,0xFFF9DC,opacity);
            this.towerSparkles.graphics.moveTo(x - radius,y);
            this.towerSparkles.graphics.lineTo(x + radius,y);
            this.towerSparkles.graphics.moveTo(x,y - radius * 1.4);
            this.towerSparkles.graphics.lineTo(x,y + radius * 1.4);
         }
         var head:Number = phase * (width + 80) - 40;
         var baseline:Number = Math.min(field.textHeight,32) + 5;
         for(var tail:int = 0; tail < 10; tail++)
         {
            var left:Number = head - tail * 5;
            if(left < 0 || left + 5 > width) continue;
            this.towerSparkles.graphics.lineStyle(2,0xFFD36B,(1 - tail / 10) * 0.85);
            this.towerSparkles.graphics.moveTo(left,baseline);
            this.towerSparkles.graphics.lineTo(left + 5,baseline);
         }
         if(head >= 0 && head <= width)
         {
            this.towerSparkles.graphics.lineStyle(1.5,0xFFFFEF,1);
            this.towerSparkles.graphics.moveTo(head - 6,baseline);
            this.towerSparkles.graphics.lineTo(head + 6,baseline);
            this.towerSparkles.graphics.moveTo(head,baseline - 5);
            this.towerSparkles.graphics.lineTo(head,baseline + 5);
         }
      }

      private function towerPrefixOffset(text:String, at:int) : int
      {
         while(at < text.length)
         {
            var code:Number = text.charCodeAt(at);
            if(code == 32 || code == 9 || code == 10 || code == 13 || code == 160 || code == 12288 || code == 65279)
            {
               at++;
               continue;
            }
            if(code == 60)
            {
               var end:int = text.indexOf(">",at);
               if(end < 0) return text.length;
               at = end + 1;
               continue;
            }
            break;
         }
         return at;
      }

      private function towerIsLegendary(text:String) : Boolean
      {
         var at:int = this.towerPrefixOffset(text,0);
         if(text.charCodeAt(at) != 20256 || text.charCodeAt(at + 1) != 35828) return false;
         at = this.towerPrefixOffset(text,at + 2);
         return text.charCodeAt(at) == 183 || text.charCodeAt(at) == 12539;
      }

      private function towerUpdateName(event:flash.events.Event) : void
      {
         var view:flash.display.DisplayObjectContainer = this.DetailStatusView;
         if(view == null) return;
         var top:flash.display.DisplayObjectContainer = view.getChildByName("AdjustPosTop") as flash.display.DisplayObjectContainer;
         if(top == null) return;
         var status:flash.display.DisplayObjectContainer = top.getChildByName("@STATUS@") as flash.display.DisplayObjectContainer;
         if(status == null) return;
         var field:flash.text.TextField = status.getChildByName("ItemName") as flash.text.TextField;
         if(field == null) return;
         if(this.towerName != field)
         {
            if(this.towerSparkles != null) this.towerSparkles.graphics.clear();
            this.towerName = field;
            this.towerSavedColor = field.textColor;
            this.towerSavedFilters = field.filters;
            this.towerSavedAlpha = field.alpha;
            this.towerGoldFilters = null;
            this.towerLegendary = false;
            this.towerStyledText = "";
         }
         var legendary:Boolean = this.towerIsLegendary(field.text);
         if(legendary)
         {
            if(!this.towerLegendary)
            {
               this.towerSavedColor = field.textColor;
               this.towerSavedFilters = field.filters;
               this.towerSavedAlpha = field.alpha;
            }
            this.towerPulseFrame = (this.towerPulseFrame + 1) % 120;
            var phase:Number = this.towerPulseFrame / 120;
            var pulse:Number = this.towerPulseFrame < 60 ? this.towerPulseFrame / 60 : (120 - this.towerPulseFrame) / 60;
            field.alpha = this.towerSavedAlpha;
            field.textColor = 0xF2C65C;
            this.towerDecorate(field,phase);
            this.towerStyledText = field.text;
            this.towerEffectState = "alpha";
            try
            {
               if(this.towerGoldFilters == null)
               {
                  this.towerEffectState = "construct";
                  var baseFilters:Array = this.towerSavedFilters == null ? [] : this.towerSavedFilters;
                  this.towerGoldFilters = baseFilters.concat([new flash.filters.GlowFilter(0xE8B84A,1,14,14,2,1,false,false)]);
               }
               this.towerEffectState = "strength";
               var glow:flash.filters.GlowFilter = this.towerGoldFilters[this.towerGoldFilters.length - 1] as flash.filters.GlowFilter;
               glow.strength = 1.1 + 1.4 * pulse;
               this.towerEffectState = "assign";
               field.filters = this.towerGoldFilters;
               this.towerFilterCount = field.filters == null ? -1 : field.filters.length;
               this.towerEffectState = "done";
               this.towerEffectError = "";
            }
            catch(error:*)
            {
               this.towerEffectError = String(error);
            }
         }
         else if(this.towerLegendary)
         {
            if(this.towerSparkles != null) {this.towerSparkles.graphics.clear();this.towerSparkles.visible = false;}
            if(field.text == this.towerStyledText) field.textColor = this.towerSavedColor;
            field.alpha = this.towerSavedAlpha;
            field.filters = this.towerSavedFilters;
            this.towerGoldFilters = null;
            this.towerEffectState = "restored";
         }
         this.towerLegendary = legendary;
      }
'''

def patch_source(source, status):
    frame = 'addFrameScript(0,this.frame1,10,this.frame11,18,this.frame19);'
    anchor = '      public function MainTimeline()'
    if source.count(frame) != 1 or source.count(anchor) != 1 or 'towerUpdateName' in source:
        raise ValueError('Unexpected source timeline; refusing to patch')
    source = source.replace(anchor, MEMBERS.replace('@STATUS@', status) + '\n' + anchor)
    return source.replace(frame, frame + '\n         addEventListener(flash.events.Event.ENTER_FRAME,this.towerUpdateName);')

def tags(data):
    if data[:3] != b'GFX':
        raise ValueError('Expected uncompressed GFX')
    end = struct.unpack_from('<I', data, 4)[0]
    pos = 8 + ((5 + (data[8] >> 3) * 4 + 7) // 8) + 4
    result = []
    while pos < end:
        header = struct.unpack_from('<H', data, pos)[0]
        pos += 2
        size = header & 63
        if size == 63:
            size = struct.unpack_from('<I', data, pos)[0]
            pos += 4
        if pos + size > end:
            raise ValueError('Truncated GFX tag')
        result.append((header >> 6, data[pos:pos + size]))
        pos += size
    if pos != end:
        raise ValueError('Invalid GFX length')
    return result

def verify_tags(original, patched):
    header_end = 8 + ((5 + (original[8] >> 3) * 4 + 7) // 8) + 4
    if original[:4] != patched[:4] or original[8:header_end] != patched[8:header_end]:
        raise ValueError('GFX version or movie dimensions/timing changed')
    before, after = tags(original), tags(patched)
    if [t for t, _ in before] != [t for t, _ in after]:
        raise ValueError('Tag structure changed')
    changed = [t for (t, a), (_, b) in zip(before, after) if a != b]
    if changed != [82]:
        raise ValueError(f'Expected only one DoABC change, got {changed}')
    return len(before)

def build(ffdec, original, output):
    output.mkdir(parents=True, exist_ok=False)
    (output / 'menu').mkdir()
    def run(*args):
        result = subprocess.run(['java', '-Djava.awt.headless=true', '-jar', str(ffdec), *map(str, args)], capture_output=True, text=True, encoding='utf-8', errors='replace')
        with (output / 'build.log').open('a', encoding='utf-8') as log:
            log.write(result.stdout + result.stderr)
        if result.returncode:
            raise RuntimeError('FFDec failed; see build.log')
    manifest = []
    for stem, (package, status) in TARGETS.items():
        src = original / (stem + '.gfx')
        dst = output / 'menu' / src.name
        with tempfile.TemporaryDirectory(prefix='tower-gfx-') as tmp:
            tmp = Path(tmp)
            run('-export', 'script', tmp / 'before', src)
            relative = Path(package) / 'MainTimeline.as'
            script = tmp / 'before' / 'scripts' / relative
            changed = tmp / 'changed' / relative
            changed.parent.mkdir(parents=True)
            changed.write_text(patch_source(script.read_text(encoding='utf-8-sig'), status), encoding='utf-8')
            run('-importScript', src, dst, tmp / 'changed')
            count = verify_tags(src.read_bytes(), dst.read_bytes())
            run('-export', 'script', tmp / 'after', dst)
            for before in (tmp / 'before' / 'scripts').rglob('*.as'):
                rel = before.relative_to(tmp / 'before' / 'scripts')
                after = tmp / 'after' / 'scripts' / rel
                if rel != relative and before.read_bytes() != after.read_bytes():
                    raise ValueError(f'Unrelated script changed: {rel}')
            actual = (tmp / 'after' / 'scripts' / relative).read_text(encoding='utf-8-sig')
            if 'towerUpdateName' not in actual or 'GlowFilter' not in actual or 'towerSavedFilters' not in actual:
                raise ValueError('Compiled name effect missing')
            (output / (stem + '.as')).write_text(actual, encoding='utf-8')
        manifest.append({'file': 'menu/' + src.name, 'originalSha256': hashlib.sha256(src.read_bytes()).hexdigest(), 'patchedSha256': hashlib.sha256(dst.read_bytes()).hexdigest(), 'tagsVerified': count, 'changedTag': 'DoABC', 'path': f'DetailStatusView.AdjustPosTop.{status}.ItemName'})
    (output / 'manifest.json').write_text(json.dumps({'experimental': True, 'gameVerified': False, 'legendaryMatcher': 'UTF-16 codepoints 4F20 8BF4 then 00B7 or 30FB; skip whitespace and leading tags', 'effect': 'wide platinum sweep, seven large sparkles and moving golden underline trail', 'files': manifest}, ensure_ascii=False, indent=2), encoding='utf-8')
    print('Built and structurally verified two isolated name-glow menus.')

if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--ffdec', type=Path, required=True)
    parser.add_argument('--original', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    build(args.ffdec.resolve(), args.original.resolve(), args.output.resolve())
