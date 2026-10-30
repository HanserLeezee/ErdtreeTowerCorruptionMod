import struct,sys
sys.path.insert(0,'/tmp/claude-0/-home-claude/15080322-d501-5446-9af6-f79f340dd029/scratchpad/gp')
from gparam import u16
PACK={1:('<I',None),2:('<h',2),3:('<i',4),5:('<?',1),7:('<i',4),9:('<f',4),11:('<?',1),12:('<2f8x',16),13:('<3f4x',16),14:('<4f',16),15:('<4B',4)}
def load(b):
    """Return header info + list of params in file order: (header_abs_offset, type, [(id,tod,value)])"""
    H=list(struct.unpack_from('<9i',b,0x1c));gh,pho,ph,vals,vids,unk2,u3c,u3,u3v=H
    gcount,=struct.unpack_from('<i',b,0x10);hdr,=struct.unpack_from('<i',b,0x18)
    params=[]
    for gi in range(gcount):
        go,=struct.unpack_from('<i',b,hdr+gi*4);o=gh+go
        pc,phoo=struct.unpack_from('<ii',b,o);gname,e=u16(b,o+8)
        for pi in range(pc):
            po,=struct.unpack_from('<i',b,pho+phoo+pi*4);p=ph+po
            vo,io=struct.unpack_from('<ii',b,p);t=b[p+8];cnt=b[p+9];name,_=u16(b,p+12)
            fmt=PACK[t][0];sz={1:4,2:2,3:4,5:1,7:4,9:4,11:1,12:16,13:16,14:16,15:4}[t]
            if t==1: fmt='<B3x'
            entries=[]
            for k in range(cnt):
                v=struct.unpack_from({1:'<B',2:'<h',3:'<i',5:'<?',7:'<i',9:'<f',11:'<?',12:'<2f',13:'<3f',14:'<4f',15:'<4B'}[t],b,vals+vo+k*sz)
                i,tod=struct.unpack_from('<if',b,vids+io+k*8)
                entries.append([i,tod,v])
            params.append(dict(group=gname,name=name,hdr=p,type=t,entries=entries,vo=vo,io=io))
    return params
def save(b,params):
    b=bytearray(b)
    gh,pho,ph,vals,vids,unk2,u3c,u3,u3v=struct.unpack_from('<9i',b,0x1c)
    # values and ids are laid out in param header order; rebuild in the same order the original used (sorted by vo)
    order=sorted(params,key=lambda p:p['vo'])
    vbuf=bytearray()
    for p in order:
        p['nvo']=len(vbuf);t=p['type']
        for i,tod,v in p['entries']:
            if t==1: vbuf+=struct.pack('<I',v[0])
            elif t in (12,): vbuf+=struct.pack('<2f',*v)+b'\0'*8
            elif t==13: vbuf+=struct.pack('<3f',*v)+b'\0'*4
            else: vbuf+=struct.pack({2:'<h',3:'<i',5:'<?',7:'<i',9:'<f',11:'<?',14:'<4f',15:'<4B'}[t],*v)
        while len(vbuf)%4: vbuf+=b'\0'
    ibuf=bytearray()
    for p in sorted(params,key=lambda p:p['io']):
        p['nio']=len(ibuf)
        for i,tod,v in p['entries']: ibuf+=struct.pack('<if',i,tod)
    for p in params:
        assert len(p['entries'])<256
        struct.pack_into('<ii',b,p['hdr'],p['nvo'],p['nio']);b[p['hdr']+9]=len(p['entries'])
    tail=bytes(b[unk2:])
    out=bytes(b[:vals])+bytes(vbuf)
    nvids=len(out);out+=bytes(ibuf);nunk2=len(out);delta=nunk2-unk2
    out+=tail;out=bytearray(out)
    struct.pack_into('<5i',out,0x1c+12,vals,nvids,nunk2,u3c,u3+delta)
    struct.pack_into('<i',out,0x1c+32,u3v+delta)
    c=struct.unpack_from('<3i',out,0x44);struct.pack_into('<3i',out,0x44,*(x+delta for x in c))
    return bytes(out)
if __name__=='__main__':
    G='/mnt/user-data/uploads/ELDEN RING/hansfix-tools/gparam-export/'
    for f in ['m45_00_0000','m45_01_0000','m45_02_0000','m60_00_0000','default']:
        b=open(G+f+'.gparam','rb').read();ps=load(b)
        r=save(b,ps);print(f,r==b,len(r),len(b))
