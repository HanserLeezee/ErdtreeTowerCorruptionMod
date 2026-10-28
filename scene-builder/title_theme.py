"""Original vector title/loading skin. Rebuilds selected GFx scripts, preserving native timelines."""
import argparse
import hashlib
import json
import math
from pathlib import Path
import random
import subprocess
import tempfile

from PIL import Image, ImageDraw, ImageFont
from native_name_glow import verify_tags

TARGETS = [
    ('05_000_title', '_05_000_Title_fla/Timeline_1.as', 'title'),
    ('05_001_title_logo', '_05_001_Title_Logo_fla/MainTimeline.as', 'logo'),
    ('02_903_nowloading2', '_02_903_NowLoading2_fla/MainTimeline.as', 'loading'),
    ('02_904_nowloading3', '_02_904_NowLoading3_fla/MainTimeline.as', 'loading'),
]


def artwork(config, mode):
    p = config['palette']; rng = random.Random(9317); commands = []
    def poly(points, color): commands.append(('poly', points, color))
    def line(points, color, width=2): commands.append(('line', points, color, width))
    def text(x, y, value, size, color): commands.append(('text', x, y, value, size, color))
    poly([(0,0),(1920,0),(1920,1080),(0,1080)],p['background'])
    # Angular, layered silhouettes leave a quiet, high-contrast area for the native menu.
    poly([(600,700),(790,390),(920,530),(1120,210),(1320,480),(1490,260),(1710,490),(1920,340),(1920,1080),(600,1080)],p['mountain'])
    poly([(750,860),(1060,550),(1220,710),(1530,460),(1780,700),(1920,580),(1920,1080),(750,1080)],'1d2e2a')
    # A fractured tower and its crooked stairway.
    poly([(1170,820),(1210,312),(1240,312),(1240,275),(1274,275),(1274,312),(1322,306),(1322,266),(1356,266),(1356,306),(1398,314),(1420,820)],p['stone'])
    poly([(1356,306),(1398,314),(1420,820),(1348,824)],'2c4038')
    for y in range(348,800,55):
        line([(1210,y),(1392,y-7)],'273a33',4)
        for x in range(1220,1390,56): line([(x+(y%3)*6,y-2),(x+(y%3)*6,y+48)],'35483e',2)
    for x,y in [(1235,390),(1326,440),(1250,540),(1330,625)]:
        poly([(x,y),(x+17,y-20),(x+34,y),(x+34,y+43),(x,y+43)],p['background'])
        line([(x+16,y),(x+16,y+29)],p['gold'],2)
    poly([(1253,822),(1253,716),(1292,680),(1330,715),(1330,822)],'0b1212')
    for i in range(18):
        y=826+i*13; x=1270-i*21; w=100+i*4
        poly([(x,y),(x+w,y-5),(x+w+13,y+8),(x-18,y+13)],'5c6650' if i%2 else '3b5044')
    # Hand-shaped golden canopy, branches and leaf flecks.
    poly([(1470,870),(1494,600),(1477,425),(1444,314),(1463,299),(1523,438),(1531,588),(1523,850)],'7b7147')
    line([(1504,640),(1584,430),(1658,325),(1730,280)],p['gold'],13)
    line([(1499,530),(1390,388),(1328,259),(1265,216)],p['gold'],12)
    line([(1510,475),(1520,320),(1490,219),(1532,143)],p['gold'],10)
    for i in range(185):
        angle=rng.uniform(0,math.tau); radius=math.sqrt(rng.random())
        x=1500+math.cos(angle)*radius*310; y=250+math.sin(angle)*radius*145
        w=rng.uniform(8,28); h=rng.uniform(4,12)
        poly([(x-w,y),(x,y-h),(x+w,y+2),(x+3,y+h)],rng.choice([p['gold'],'807848','d8bd71','a39353','4e6043']))
    for i in range(65):
        x=rng.uniform(820,1850); y=rng.uniform(70,980)
        line([(x,y),(x+3,y-7)],'81794c',1)
    poly([(0,1010),(410,987),(720,1005),(920,950),(1120,980),(1550,930),(1920,1000),(1920,1080),(0,1080)],'0c1414')
    line([(80,72),(1840,72),(1840,1008)],'4a5139',2)
    line([(80,72),(80,1008),(1840,1008)],'4a5139',2)
    if mode=='title':
        poly([(104,490),(672,484),(686,970),(118,982)],'172320')
        line([(116,499),(659,495),(674,962),(129,970),(116,499)],'766b45',2)
        text(128,170,config['subtitle'],28,p['gold'])
        text(122,225,config['title'],82,p['paper'])
        line([(130,348),(620,348)],p['gold'],3)
        text(130,375,config['tagline'],24,'9caa91')
        text(130,930,'每一次攀登，都是新的选择',20,'9caa91')
    else:
        poly([(102,748),(1140,740),(1160,968),(117,980)],p['paper'])
        line([(118,762),(1125,754),(1144,954),(132,966),(118,762)],'7b6b46',3)
        text(130,125,config['subtitle'],26,p['gold'])
        text(126,175,config['loading_title'],52,p['paper'])
    return commands


def render_preview(commands, config, mode, path):
    image=Image.new('RGB',(1920,1080)); d=ImageDraw.Draw(image)
    font='C:/Windows/Fonts/msyh.ttc'
    for c in commands:
        if c[0]=='poly': d.polygon(c[1], fill='#'+c[2])
        elif c[0]=='line': d.line(c[1], fill='#'+c[2], width=c[3])
        else: d.text((c[1],c[2]),c[3],font=ImageFont.truetype(font,c[4]),fill='#'+c[5])
    if mode=='title':
        for i,s in enumerate(['继续旅程','载入游戏','开始新旅程','系统设置','结束游戏']):
            d.text((220,555+i*63),s,font=ImageFont.truetype(font,32),fill='#d1bd87')
        d.polygon([(182,563),(198,577),(182,590)],fill='#c4a45b')
    else:
        d.text((155,785),config['tips'][0][0],font=ImageFont.truetype(font,32),fill='#18231f')
        d.text((155,852),config['tips'][0][1],font=ImageFont.truetype(font,25),fill='#18231f')
    image.save(path)


def methods(config,mode):
    if mode=='logo':
        return 'private function towerThemeTick(e:flash.events.Event):void { this.alpha=0; }'
    commands=artwork(config,mode); code=[]
    for c in commands:
        if c[0] in ('poly','line'):
            code.append('g.lineStyle();')
            if c[0]=='poly': code.append('g.beginFill(0x'+c[2]+');')
            else: code.append('g.lineStyle('+str(c[3])+',0x'+c[2]+');')
            for i,(x,y) in enumerate(c[1]):code.append('g.'+('moveTo' if i==0 else 'lineTo')+'('+f'{x:.2f},{y:.2f}'+');')
            if c[0]=='poly':code.append('g.endFill();')
        else:code.append('towerThemeText('+','.join([str(c[1]),str(c[2]),json.dumps(c[3],ensure_ascii=False),str(c[4]),'0x'+c[5]])+');')
    setup='towerTheme.x=-960; towerTheme.y=-540;' if mode=='title' else ''
    tick='''
        if(this["CommandList"]!=null){this["CommandList"].x=-550;this["CommandList"].y=90;}
        if(this["PressStart"]!=null){this["PressStart"].x=-550;this["PressStart"].y=160;}
    ''' if mode=='title' else '''
        if(this["BackImage"]!=null)this["BackImage"].visible=false;
        if(this["Main"]!=null && this["Main"]["Knowledge"]!=null)this["Main"]["Knowledge"].visible=false;
        var n:int=int(towerThemeFrames/240)%towerThemeTips.length;
        towerThemeTipTitle.text=towerThemeTips[n][0];towerThemeTipBody.text=towerThemeTips[n][1];
    '''
    tips='' if mode=='title' else '''
        towerThemeTipTitle=towerThemeText(155,785,"",32,0x18231f);
        towerThemeTipBody=towerThemeText(155,852,"",25,0x18231f);
    '''
    return '''
      private var towerTheme:flash.display.Sprite;
      private var towerThemeFrames:int=0;
      private var towerThemeTipTitle:flash.text.TextField;
      private var towerThemeTipBody:flash.text.TextField;
      private var towerThemeTips:Array='''+json.dumps(config['tips'],ensure_ascii=False)+''';
      private function towerThemeText(x:Number,y:Number,value:String,size:int,color:uint):flash.text.TextField {
        var t:flash.text.TextField=new flash.text.TextField();t.x=x;t.y=y;t.width=1100;t.height=140;
        t.defaultTextFormat=new flash.text.TextFormat("MenuFont_01",size,color);t.embedFonts=true;
        t.selectable=false;t.mouseEnabled=false;t.multiline=true;t.wordWrap=true;t.text=value;towerTheme.addChild(t);return t;
      }
      private function towerThemeTick(e:flash.events.Event):void {
        towerThemeFrames++;
        if(towerTheme==null){
          towerTheme=new flash.display.Sprite();towerTheme.mouseEnabled=false;towerTheme.mouseChildren=false;
          '''+setup+'''
          var g:flash.display.Graphics=towerTheme.graphics;
          '''+'\n'.join(code)+tips+'''
          addChildAt(towerTheme,'''+('0' if mode=='title' else 'Math.min(1,numChildren)')+''');
        }
        '''+tick+'''
      }
    '''


def build(args):
    config=json.loads(args.config.read_text(encoding='utf-8'))
    args.output.mkdir(parents=True,exist_ok=False);(args.output/'menu').mkdir()
    manifest=[]
    def run(*cmd):
        r=subprocess.run([args.java,'-Djava.awt.headless=true','-jar',str(args.ffdec),*map(str,cmd)],capture_output=True,text=True,encoding='utf-8',errors='replace')
        with (args.output/'build.log').open('a',encoding='utf-8') as log:log.write(r.stdout+r.stderr)
        if r.returncode:raise RuntimeError('FFDec failed; see build.log')
    for stem,relative,mode in TARGETS:
        src=args.original/(stem+'.gfx');dst=args.output/'menu'/src.name
        with tempfile.TemporaryDirectory(prefix='tower-theme-') as tmp:
            tmp=Path(tmp);run('-export','script',tmp/'before',src)
            before=tmp/'before/scripts'/relative;source=before.read_text(encoding='utf-8-sig')
            marker='      public function '+Path(relative).stem+'('
            assert marker in source and 'towerTheme' not in source
            source=source.replace(marker,methods(config,mode)+'\n'+marker,1)
            source=source.replace('super();','super();\n         addEventListener(flash.events.Event.ENTER_FRAME,towerThemeTick);',1)
            changed=tmp/'changed'/relative;changed.parent.mkdir(parents=True);changed.write_text(source,encoding='utf-8')
            run('-importScript',src,dst,tmp/'changed');count=verify_tags(src.read_bytes(),dst.read_bytes())
            run('-export','script',tmp/'after',dst)
            for original in (tmp/'before/scripts').rglob('*.as'):
                rel=original.relative_to(tmp/'before/scripts');after=tmp/'after/scripts'/rel
                if str(rel).replace('\\','/')!=relative and original.read_bytes()!=after.read_bytes():raise ValueError('Unrelated script changed: '+str(rel))
            actual=(tmp/'after/scripts'/relative).read_text(encoding='utf-8-sig')
            assert 'towerThemeTick' in actual
        manifest.append(dict(file='menu/'+src.name,sha256=hashlib.sha256(dst.read_bytes()).hexdigest(),tagsVerified=count))
    for mode in ['title','loading']:render_preview(artwork(config,mode),config,mode,args.output/(mode+'-design-preview.png'))
    (args.output/'manifest.json').write_text(json.dumps(dict(gameVerified=False,preview='Design preview; native menu labels are illustrative',files=manifest),indent=2),encoding='utf-8')
    print('Built four GFx resources; native timelines and unrelated scripts preserved')


if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--config',type=Path,default=Path(__file__).with_name('title_theme.json'))
    parser.add_argument('--original',type=Path,required=True);parser.add_argument('--output',type=Path,required=True)
    parser.add_argument('--ffdec',type=Path,required=True);parser.add_argument('--java',default='java')
    build(parser.parse_args())
