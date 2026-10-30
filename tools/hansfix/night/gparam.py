import struct,sys
def u16(b,o):
    e=o
    while b[e:e+2]!=b'\0\0': e+=2
    return b[o:e].decode('utf-16le'),e+2
FMT={1:('B',1),2:('h',2),3:('i',4),5:('?',1),7:('i',4),9:('f',4),11:('?',1),12:('2f',16),13:('3f',16),14:('4f',16),15:('4B',4)}
def parse(b):
    assert b[:8]=='filt'.encode('utf-16le')
    game,=struct.unpack_from('<I',b,8)
    gcount,unk14,hdr=struct.unpack_from('<iii',b,0x10)
    gh,pho,ph,vals,vids,unk2,u3c,u3,u3v=struct.unpack_from('<9i',b,0x1c)
    groups=[]
    for gi in range(gcount):
        go,=struct.unpack_from('<i',b,hdr+gi*4)
        o=gh+go;pc,phoo=struct.unpack_from('<ii',b,o)
        n1,e=u16(b,o+8);n2,e=u16(b,e)
        params=[]
        for pi in range(pc):
            po,=struct.unpack_from('<i',b,pho+phoo+pi*4)
            p=ph+po;vo,io=struct.unpack_from('<ii',b,p);t=b[p+8];cnt=b[p+9]
            pn1,e=u16(b,p+12);pn2,e=u16(b,e)
            f,sz=FMT[t];values=[]
            for k in range(cnt):
                v=struct.unpack_from('<'+f,b,vals+vo+k*sz);values.append(v if len(v)>1 else v[0])
            ids=[struct.unpack_from('<if',b,vids+io+k*8) for k in range(cnt)]
            params.append((pn1,pn2,t,values,ids))
        groups.append((n1,n2,params))
    return groups
if __name__=='__main__':
    g=parse(open(sys.argv[1],'rb').read())
    want=sys.argv[2] if len(sys.argv)>2 else None
    for n1,n2,ps in g:
        if want and want not in n1: continue
        print('##',n1,'|',n2,len(ps))
        for pn1,pn2,t,v,ids in ps:
            if len(sys.argv)>3 and sys.argv[3] not in pn1: continue
            print('  ',pn1,'|',pn2,'t',t,[(i,round(tod,2),vv if not isinstance(vv,tuple) else tuple(round(x,3) for x in vv)) for (i,tod),vv in zip(ids,v)][:12])
