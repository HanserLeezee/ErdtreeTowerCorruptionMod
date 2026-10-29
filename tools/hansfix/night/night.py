import sys,struct,json
sys.path.insert(0,'/tmp/claude-0/-home-claude/15080322-d501-5446-9af6-f79f340dd029/scratchpad/gp')
import gpwrite
G='/mnt/user-data/uploads/ELDEN RING/hansfix-tools/gparam-export/'
OUT='/tmp/claude-0/-home-claude/15080322-d501-5446-9af6-f79f340dd029/scratchpad/gp/out/'
import os;os.makedirs(OUT,exist_ok=True)
# ER gparam time keys: key = (hour - 6) mod 24  (0=06:00 morning, 12=18:00 dusk, 15=21:00 night, 21=03:00, 22.5=04:30)
KEYS=[0.0,4.0,8.0,12.0,15.0,21.0,22.5]
NIGHT_KEYS={15.0,21.0,22.5}
DIM=0.6
def at(entries,key):
    e=sorted((t,v) for i,t,v in entries)
    if not e: return None
    for (t0,v0),(t1,v1) in zip(e,e[1:]):
        if t0<=key<=t1:
            f=(key-t0)/(t1-t0) if t1>t0 else 0
            if isinstance(v0[0],bool) or isinstance(v0[0],int) and not isinstance(v0[0],float): return v0
            return tuple(a+(b-a)*f for a,b in zip(v0,v1))
    return e[-1][1] if key>=e[-1][0] else e[0][1]
ref={}
for p in gpwrite.load(open(G+'m60_00_0000.gparam','rb').read()):
    ent=[x for x in p['entries'] if x[0]==0]
    ref[(p['group'],p['name'])]=(p['type'],ent)
report={}
for f in ['m45_00_0000','m45_01_0000','m45_02_0000']:
    b=open(G+f+'.gparam','rb').read();ps=gpwrite.load(b);changed=0
    for p in ps:
        t=p['type'];r=ref.get((p['group'],p['name']))
        ids=[]
        for i,tod,v in p['entries']:
            if i not in ids: ids.append(i)
        new=[]
        for i in ids:
            day=next(v for j,tod,v in p['entries'] if j==i)
            night=day
            if r and r[0]==t and t in (9,12,13,14) and r[1]:
                d=at(r[1],4.0);n=at(r[1],16.0)
                if 'Angle' in p['name']: night=n
                else:
                    comp=[]
                    for m,dd,nn in zip(day,d,n):
                        if abs(dd)>1e-3: comp.append(m*max(0.0,min(4.0,nn/dd)))
                        else: comp.append(m+(nn-dd))
                    night=tuple(comp)
                    # 再整体压暗一些：直射光、环境半球光的强度（alpha 分量）×0.6
                    if p['group'].startswith('LightSet') and t==14 and ('Color' in p['name']):
                        night=night[:3]+(night[3]*DIM,)
            if night!=day: changed+=1
            for k in KEYS: new.append([i,k,night if k in NIGHT_KEYS else day])
        p['entries']=new
    nb=gpwrite.save(b,ps)
    # sanity: reparse
    assert len(gpwrite.load(nb))==len(ps)
    open(OUT+f+'.gparam','wb').write(nb);report[f]=(len(b),len(nb),changed)
print(report)
