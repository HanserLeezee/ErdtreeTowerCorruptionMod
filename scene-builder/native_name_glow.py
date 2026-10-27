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
      private var towerTierNow:int = -1;
      private var towerFrame:Number = 0;
      private var towerAdded:Array = [];
      private var towerSavedAlpha:Number = 1;
      private var towerEffectState:String = "idle";
      private var towerEffectError:String = "";
      private var towerFilterCount:int = -1;
      private var towerSparkles:flash.display.Sprite;
      private var towerAura:flash.display.Sprite;
      private var towerStyledText:String = "";
      private var towerStyle:String = "@STYLE@";
      private var towerCharText:String = "";
      private var towerCharX:Array = [];
      private var towerFormat:flash.text.TextFormat = new flash.text.TextFormat();
      private var towerBox:flash.geom.Matrix = new flash.geom.Matrix();
      private var towerGold:Array = [0xF5C66B,0xFFD98A,0xFFE49A,0xFFF0BA,0xF8D17A];
      private var towerWarm:Array = [0xFFD478,0xFFE7A6,0xF6C563,0xFFF1C4];
      private var towerViolet:Array = [0xD8B4FF,0xEACFFF,0xCDA5F4,0xF3E4FF];
      private var towerBlue:Array = [0x7FA8F0,0xA8C8FF,0x8CB4F8,0xB8D4FF];

      private function towerHash(n:Number) : Number
      {
         var v:Number = Math.sin(n * 12.9898 + 78.233) * 43758.5453;
         return v - Math.floor(v);
      }

      private function towerMix(a:uint, b:uint, k:Number) : uint
      {
         if(k < 0) k = 0;
         if(k > 1) k = 1;
         var red:int = int(((a >> 16) & 255) + (((b >> 16) & 255) - ((a >> 16) & 255)) * k);
         var green:int = int(((a >> 8) & 255) + (((b >> 8) & 255) - ((a >> 8) & 255)) * k);
         var blue:int = int((a & 255) + ((b & 255) - (a & 255)) * k);
         return uint((red << 16) | (green << 8) | blue);
      }

      private function towerPaletteAt(palette:Array, u:Number) : uint
      {
         u -= Math.floor(u);
         var size:int = palette.length;
         var at:Number = u * size;
         var slot:int = int(at);
         return this.towerMix(uint(palette[slot % size]),uint(palette[(slot + 1) % size]),at - slot);
      }

      private function towerLayers(field:flash.text.TextField) : void
      {
         var parent:flash.display.DisplayObjectContainer = field.parent;
         if(this.towerSparkles == null || this.towerSparkles.parent != parent)
         {
            if(this.towerSparkles != null && this.towerSparkles.parent != null) this.towerSparkles.parent.removeChild(this.towerSparkles);
            if(this.towerAura != null && this.towerAura.parent != null) this.towerAura.parent.removeChild(this.towerAura);
            this.towerSparkles = new flash.display.Sprite();
            this.towerAura = new flash.display.Sprite();
            this.towerSparkles.mouseEnabled = false;
            this.towerSparkles.mouseChildren = false;
            this.towerAura.mouseEnabled = false;
            this.towerAura.mouseChildren = false;
            this.towerSparkles.blendMode = "add";
            this.towerAura.blendMode = "add";
            this.towerSparkles.filters = [new flash.filters.GlowFilter(0xFFB43C,0.8,5,5,2,2,false,false)];
            this.towerAura.filters = [new flash.filters.BlurFilter(6,6,2)];
            parent.addChild(this.towerSparkles);
            parent.addChildAt(this.towerAura,parent.getChildIndex(field));
         }
         this.towerSparkles.visible = field.visible;
         this.towerAura.visible = field.visible;
         this.towerSparkles.transform.matrix = field.transform.matrix;
         this.towerAura.transform.matrix = field.transform.matrix;
         this.towerSparkles.alpha = this.towerSavedAlpha;
         this.towerAura.alpha = this.towerSavedAlpha;
      }

      private function towerMeasure(field:flash.text.TextField, left:Number, width:Number) : void
      {
         var text:String = field.text;
         if(text == this.towerCharText && this.towerCharX.length == text.length) return;
         this.towerCharText = text;
         this.towerCharX = [];
         for(var index:int = 0; index < text.length; index++)
         {
            var x:Number = left + width * (index + 0.5) / Math.max(1,text.length);
            try
            {
               var bounds:flash.geom.Rectangle = field.getCharBoundaries(index);
               if(bounds != null && bounds.width > 0) x = bounds.x + bounds.width * 0.5;
            }
            catch(error:*)
            {
            }
            this.towerCharX.push(x);
         }
      }

      // 星芒：两条细长菱形交叉成十字光
      private function towerFlare(g:flash.display.Graphics, x:Number, y:Number, len:Number, spin:Number, color:uint, alpha:Number) : void
      {
         if(alpha <= 0.01 || len <= 0.3) return;
         var thin:Number = len * 0.13;
         for(var arm:int = 0; arm < 2; arm++)
         {
            var angle:Number = spin + arm * Math.PI / 2;
            var cos:Number = Math.cos(angle);
            var sin:Number = Math.sin(angle);
            g.beginFill(color,alpha);
            g.moveTo(x + cos * len,y + sin * len);
            g.lineTo(x - sin * thin,y + cos * thin);
            g.lineTo(x - cos * len,y - sin * len);
            g.lineTo(x + sin * thin,y - cos * thin);
            g.lineTo(x + cos * len,y + sin * len);
            g.endFill();
         }
      }

      // 闪光：约每 2 秒有六成机会亮一下（平均约 3.3 秒一次），0.5 秒内渐亮渐暗，返回 0..1 的亮度
      private function towerGleam(time:Number) : Number
      {
         var at:Number = time % 2;
         return this.towerHash(Math.floor(time / 2)) < 0.6 && at < 0.5 ? Math.sin(at / 0.5 * Math.PI) : 0;
      }

      // 品质越高特效越多（tier：0 普通、1 稀有、2 史诗、3 传说）：
      // 普通只有偶尔一道淡扫光；稀有蓝色流动 + 扫光；史诗淡紫字身 + 扫光 + 字后紫光 + 稀疏星光；
      // 传说为灾厄“燃金”式：明亮暖金字身 + 扫光 + 偶发渐亮的闪光 + 字后金光 + 密集星光。描边、光晕与泛光由文字滤镜完成（towerFilters）
      private function towerDecorate(field:flash.text.TextField, time:Number, tier:int) : void
      {
         var auric:Boolean = this.towerStyle != "gold";
         var count:int = field.text.length;
         var width:Number = Math.max(8,Math.min(field.width - 4,field.textWidth));
         var height:Number = Math.max(8,Math.min(field.textHeight,40));
         var align:String = field.getTextFormat().align;
         var left:Number = align == "center" ? (field.width - width) / 2 : (align == "right" ? field.width - 2 - width : 2);
         var top:Number = 2;
         var breath:Number = 0.5 + 0.5 * Math.sin(time * 2.3);
         var gleam:Number = tier == 3 ? this.towerGleam(time) : 0;
         // 扫光：亮度 = ((sin(kx - vt) + 1) / 2) ^ n，只改字的颜色。普通、稀有字身本来就亮，用窄峰；
         // 史诗、传说用宽峰（约两个字宽、边缘渐变）并只提亮到四成多，避免单个字突然跳亮
         var speed:Number = [0.7,1.1,1.1,1.2][tier];
         var strength:Number = [0.5,0.85,0.4,0.45][tier];
         var sharpness:Number = tier >= 2 ? 24 : 120;
         var shineColor:uint = [0xFFFFFF,0xF4F8FF,0xEAD4FF,auric ? 0xFEE775 : 0xFFFFF4][tier];
         this.towerMeasure(field,left,width);
         for(var index:int = 0; index < count; index++)
         {
            var shine:Number = Math.pow((Math.sin(0.011 * Number(this.towerCharX[index]) - speed * time) + 1) * 0.5,sharpness) * strength;
            var color:uint = this.towerSavedColor;
            if(tier == 1) color = this.towerPaletteAt(this.towerBlue,time * 0.25 - index * 0.06);
            else if(tier == 2) color = this.towerPaletteAt(this.towerViolet,time * 0.2 - index * 0.07);
            else if(tier == 3) color = auric ? this.towerPaletteAt(this.towerWarm,time * 0.2 - index * 0.07) : this.towerPaletteAt(this.towerGold,time * 0.3 - index * 0.06);
            color = this.towerMix(color,shineColor,shine);
            if(gleam > 0) color = this.towerMix(color,0xFFF6D0,0.25 * gleam);
            this.towerFormat.color = color;
            field.setTextFormat(this.towerFormat,index,index + 1);
         }
         if(tier < 2)
         {
            if(this.towerSparkles != null) {this.towerSparkles.graphics.clear();this.towerSparkles.visible = false;}
            if(this.towerAura != null) {this.towerAura.graphics.clear();this.towerAura.visible = false;}
            return;
         }
         this.towerLayers(field);
         // 字后：随呼吸明暗的椭圆光（传说金、史诗紫）
         var back:flash.display.Graphics = this.towerAura.graphics;
         back.clear();
         this.towerBox.createGradientBox(width + 110,height + 46,0,left - 55,top - 23);
         back.beginGradientFill("radial",[tier == 3 ? 0xFF9A2A : 0x9A5CFF,tier == 3 ? 0xFF9A2A : 0x9A5CFF],[tier == 3 ? 0.3 + 0.15 * breath : 0.16 + 0.1 * breath,0],[0,255],this.towerBox);
         back.drawEllipse(left - 55,top - 23,width + 110,height + 46);
         back.endFill();
         // 星光（灾厄 CosmicPurple 的做法）：位置固定，按时间轮流亮起、上升、旋转；传说约每 20 像素一颗，史诗约每 45 像素一颗
         var g:flash.display.Graphics = this.towerSparkles.graphics;
         g.clear();
         var stars:int = tier == 3 ? Math.max(5,Math.round(width / 20)) : Math.max(3,Math.round(width / 45));
         var outer:uint = tier == 3 ? 0xFFB43C : 0xA870FF;
         var inner:uint = tier == 3 ? 0xFFF8E0 : 0xF2E6FF;
         for(var star:int = 0; star < stars; star++)
         {
            var life:Number = (time * 3 + this.towerHash(star + 90) * Math.PI * 2) % (Math.PI * 2);
            if(life > Math.PI) continue;
            var glint:Number = Math.sin(life);
            var sx:Number = left + this.towerHash(star) * width;
            var sy:Number = top + this.towerHash(star + 50) * height * 0.6 - life * 5;
            var spin:Number = time * (0.8 + 0.7 * this.towerHash(star + 130));
            var reach:Number = height * (0.06 + 0.14 * glint) * (tier == 3 ? 1 : 0.8);
            this.towerFlare(g,sx,sy,reach * 1.6,spin,outer,glint * 0.3);
            this.towerFlare(g,sx,sy,reach,spin,inner,glint * 0.85);
         }
      }

      // 文字滤镜（接在原有滤镜之后，towerAdded 为追加的那几层）：
      // 传说：金色描边、内层热光、两层向上的火焰投影、外层泛光；史诗：紫色描边、两道环绕旋转的紫色投影（灾厄 CosmicPurple 的环绕副本）、紫色泛光；
      // 稀有：一层随呼吸明暗的柔和蓝光；普通：不加
      private function towerFilters(time:Number, tier:int) : void
      {
         var auric:Boolean = this.towerStyle != "gold";
         var breath:Number = 0.5 + 0.5 * Math.sin(time * 2.3);
         var flick:Number = 0.5 + 0.5 * Math.sin(time * 9.1) * Math.sin(time * 5.3 + 1.2);
         if(this.towerGoldFilters == null)
         {
            if(tier == 3) this.towerAdded = [
               auric ? new flash.filters.GlowFilter(0xFFCF5A,1,2.5,2.5,7,2,false,false) : new flash.filters.GlowFilter(0x6A2A06,1,2.2,2.2,5,2,false,false),
               new flash.filters.GlowFilter(auric ? 0xFFF0B0 : 0xFFF0C0,0.6,4,4,1.4,2,false,false),
               new flash.filters.DropShadowFilter(3,270,0xFF7A18,0.5,2,9,1.3,2,false,false,false),
               new flash.filters.DropShadowFilter(5,270,0xFFB030,0.22,5,14,1,2,false,false,false),
               new flash.filters.GlowFilter(0xF28A1A,0.3,16,16,1.1,2,false,false)];
            else if(tier == 2) this.towerAdded = [
               new flash.filters.GlowFilter(0xC58BFF,1,2.5,2.5,6,2,false,false),
               new flash.filters.DropShadowFilter(3,0,0xB070FF,0.45,2,2,1,2,false,false,false),
               new flash.filters.DropShadowFilter(3,180,0x8A50E0,0.35,2,2,1,2,false,false,false),
               new flash.filters.GlowFilter(0x9A5CFF,0.3,14,14,1,2,false,false)];
            else if(tier == 1) this.towerAdded = [new flash.filters.GlowFilter(0x6FA0FF,0.7,6,6,1.5,2,false,false)];
            else this.towerAdded = [];
            var baseFilters:Array = this.towerSavedFilters == null ? [] : this.towerSavedFilters;
            this.towerGoldFilters = baseFilters.concat(this.towerAdded);
         }
         var added:Array = this.towerAdded;
         if(tier == 3)
         {
            var rim:flash.filters.GlowFilter = added[0] as flash.filters.GlowFilter;
            var heat:flash.filters.GlowFilter = added[1] as flash.filters.GlowFilter;
            var flame:flash.filters.DropShadowFilter = added[2] as flash.filters.DropShadowFilter;
            var plume:flash.filters.DropShadowFilter = added[3] as flash.filters.DropShadowFilter;
            var bloom:flash.filters.GlowFilter = added[4] as flash.filters.GlowFilter;
            // 闪光主要提亮描边与热光，字身只略微变亮
            var gleam:Number = this.towerGleam(time);
            if(auric) rim.color = this.towerMix(0xFFCF5A,0xFFF4C8,gleam);
            heat.alpha = Math.min(1,(auric ? 0.5 + 0.3 * breath : 0.55 + 0.25 * breath) + 0.35 * gleam);
            flame.distance = 2 + 2 * flick;
            flame.blurY = 7 + 4 * flick;
            plume.distance = 4 + 2 * breath;
            plume.angle = 270 + 8 * Math.sin(time * 1.7);
            bloom.alpha = 0.22 + 0.2 * breath;
         }
         else if(tier == 2)
         {
            // 两道投影绕着文字转圈，距离随呼吸伸缩
            var orbitA:flash.filters.DropShadowFilter = added[1] as flash.filters.DropShadowFilter;
            var orbitB:flash.filters.DropShadowFilter = added[2] as flash.filters.DropShadowFilter;
            var haze:flash.filters.GlowFilter = added[3] as flash.filters.GlowFilter;
            orbitA.angle = (time * 115) % 360;
            orbitB.angle = (time * 115 + 180) % 360;
            orbitA.distance = orbitB.distance = 2.5 + Math.sin(time * 5);
            haze.alpha = 0.2 + 0.18 * breath;
         }
         else if(tier == 1)
         {
            (added[0] as flash.filters.GlowFilter).alpha = 0.45 + 0.3 * breath;
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

      // 品质前缀：传说 3、史诗 2、稀有 1、普通 0，不是带品质前缀的名字返回 -1（只比较字符编码，不依赖正则）
      private function towerTier(text:String) : int
      {
         var at:int = this.towerPrefixOffset(text,0);
         var first:Number = text.charCodeAt(at);
         var second:Number = text.charCodeAt(at + 1);
         var tier:int = first == 20256 && second == 35828 ? 3 : (first == 21490 && second == 35799 ? 2 : (first == 31232 && second == 26377 ? 1 : (first == 26222 && second == 36890 ? 0 : -1)));
         if(tier < 0) return -1;
         at = this.towerPrefixOffset(text,at + 2);
         return text.charCodeAt(at) == 183 || text.charCodeAt(at) == 12539 ? tier : -1;
      }

      private function towerClear() : void
      {
         if(this.towerSparkles != null) {this.towerSparkles.graphics.clear();this.towerSparkles.visible = false;}
         if(this.towerAura != null) {this.towerAura.graphics.clear();this.towerAura.visible = false;}
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
            this.towerClear();
            this.towerName = field;
            this.towerSavedColor = field.textColor;
            this.towerSavedFilters = field.filters;
            this.towerSavedAlpha = field.alpha;
            this.towerGoldFilters = null;
            this.towerTierNow = -1;
            this.towerStyledText = "";
            this.towerCharText = "";
         }
         var tier:int = this.towerTier(field.text);
         if(tier >= 0)
         {
            if(this.towerTierNow < 0)
            {
               this.towerSavedColor = field.textColor;
               this.towerSavedFilters = field.filters;
               this.towerSavedAlpha = field.alpha;
            }
            // 换到另一种品质时重建滤镜
            if(tier != this.towerTierNow) this.towerGoldFilters = null;
            // GFx reports ReferenceError #1065 for the imported timer; frame time is verified in game.
            var time:Number = (++this.towerFrame) / 30;
            field.alpha = this.towerSavedAlpha;
            this.towerEffectState = "decorate";
            try
            {
               this.towerDecorate(field,time,tier);
               this.towerStyledText = field.text;
               this.towerEffectState = "filters";
               this.towerFilters(time,tier);
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
         else if(this.towerTierNow >= 0)
         {
            this.towerClear();
            if(field.text == this.towerStyledText) field.textColor = this.towerSavedColor;
            field.alpha = this.towerSavedAlpha;
            field.filters = this.towerSavedFilters;
            this.towerGoldFilters = null;
            this.towerEffectState = "restored";
         }
         this.towerTierNow = tier;
      }
'''

def strip_previous(source):
    """Remove an earlier build's tower members so a previously patched menu can be rebuilt in place."""
    import re
    lines = source.split('\n')
    kept = []
    skipping = False
    for line in lines:
        if skipping:
            if line == '      }':
                skipping = False
            continue
        if re.match(r'      private function tower\w+\(', line):
            skipping = True
            continue
        if re.match(r'\s*private var tower\w+[^;]*;\s*$', line):
            continue
        if 'addEventListener(' in line and 'this.towerUpdateName' in line:
            continue
        kept.append(line)
    stripped = re.sub(r'\n(?:[ \t]*\n){2,}', '\n\n', '\n'.join(kept))
    if 'tower' in re.sub(r'_02_05\d_\w+', '', stripped).replace('Tower', ''):
        raise ValueError('Could not strip the previous name effect cleanly')
    return stripped

def patch_source(source, status, style='auric'):
    frame = 'addFrameScript(0,this.frame1,10,this.frame11,18,this.frame19);'
    anchor = '      public function MainTimeline()'
    if source.count(frame) != 1 or source.count(anchor) != 1 or 'towerUpdateName' in source:
        raise ValueError('Unexpected source timeline; refusing to patch')
    if style not in ('auric', 'gold'):
        raise ValueError('Unknown style')
    source = source.replace(anchor, MEMBERS.replace('@STATUS@', status).replace('@STYLE@', style) + '\n' + anchor)
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

def known_originals():
    """Map earlier patched menus to the vanilla hashes recorded in reference/native-name-glow-build.json."""
    record = Path(__file__).resolve().parents[1] / 'reference' / 'native-name-glow-build.json'
    if not record.exists():
        return {}
    files = json.loads(record.read_text(encoding='utf-8')).get('files', [])
    known = {}
    for f in files:
        for key in ('patchedSha256', 'baseSha256'):
            if key in f and 'originalSha256' in f:
                known[f[key]] = f['originalSha256']
    return known

def build(ffdec, original, output, rebase=False, style='auric', java='java'):
    output.mkdir(parents=True, exist_ok=False)
    (output / 'menu').mkdir()
    def run(*args):
        result = subprocess.run([java, '-Djava.awt.headless=true', '-jar', str(ffdec), *map(str, args)], capture_output=True, text=True, encoding='utf-8', errors='replace')
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
            source = script.read_text(encoding='utf-8-sig')
            if rebase:
                source = strip_previous(source)
            changed.write_text(patch_source(source, status, style), encoding='utf-8')
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
        base = hashlib.sha256(src.read_bytes()).hexdigest()
        manifest.append({'file': 'menu/' + src.name, 'rebasedFromPatched': rebase, 'baseSha256': base, 'originalSha256': known_originals().get(base, base) if rebase else base, 'patchedSha256': hashlib.sha256(dst.read_bytes()).hexdigest(), 'tagsVerified': count, 'changedTag': 'DoABC', 'path': f'DetailStatusView.AdjustPosTop.{status}.ItemName'})
    (output / 'manifest.json').write_text(json.dumps({'experimental': True, 'gameVerified': False, 'qualityMatcher': 'UTF-16 prefix 4F20 8BF4 (legendary) / 53F2 8BD7 (epic) / 7A00 6709 (rare) / 666E 901A (common), then 00B7 or 30FB; skip whitespace and leading tags', 'style': style, 'clock': 'ENTER_FRAME / 30; avoids GFx getTimer ReferenceError #1065', 'effect': 'Effects scale with quality (common: faint glint; rare: flowing blue + glint + breathing blue glow; epic: bright lavender body, glint, purple rim, two orbiting purple shadows, haze, sparse sparkles); legendary is Calamity-style burnished gold: bright warm gold per-glyph body with a soft wide ((sin+1)/2)^24 glint on glyphs only (epic too; common/rare keep ^120), ~3.3 s eased 0.5 s gleam mostly on the rim and heat glow, gold rim + heat glow + two upward flame drop shadows + bloom filters, additive aura and CosmicPurple-style rising rotating cross sparkles', 'files': manifest}, ensure_ascii=False, indent=2), encoding='utf-8')
    print('Built and structurally verified two isolated name-glow menus.')

if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--ffdec', type=Path, required=True)
    parser.add_argument('--original', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--rebase', action='store_true', help='--original holds menus from an earlier build; replace its name effect')
    parser.add_argument('--style', choices=['auric', 'gold'], default='auric', help='auric: bright warm gold body with a gold rim (Calamity style); gold: bright gold body')
    parser.add_argument('--java', default='java')
    args = parser.parse_args()
    build(args.ffdec.resolve(), args.original.resolve(), args.output.resolve(), args.rebase, args.style, args.java)
