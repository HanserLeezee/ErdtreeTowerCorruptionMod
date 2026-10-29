import base64,struct,sys,json,os
S='/tmp/claude-0/-home-claude/15080322-d501-5446-9af6-f79f340dd029/scratchpad'
U='/mnt/user-data/uploads/ELDEN RING/Game/'
def der_int(b,i):
    assert b[i]==2;i+=1;l=b[i];i+=1
    if l&0x80:n=l&0x7f;l=int.from_bytes(b[i:i+n],'big');i+=n
    return int.from_bytes(b[i:i+l],'big'),i+l
def key(name):
    t=open(f'{S}/bhd/{name}.pem').read()
    body=''.join(l for l in t.splitlines() if not l.startswith('-----'))
    b=base64.b64decode(body);i=0
    assert b[0]==0x30;i=1;l=b[i];i+=1
    if l&0x80:i+=l&0x7f
    n,i=der_int(b,i);e,i=der_int(b,i);return n,e
def decrypt(name):
    n,e=key(name);k=(n.bit_length()+7)//8
    raw=open(U+name+'.bhd','rb').read();out=bytearray()
    for o in range(0,len(raw),k):
        m=pow(int.from_bytes(raw[o:o+k],'big'),e,n);out+=m.to_bytes(k-1,'big')
    return bytes(out)
def h(p):
    p=p.lower().replace('\\','/');p=p if p.startswith('/') else '/'+p;v=0
    for c in p:v=(v*0x85+ord(c))&0xffffffffffffffff
    return v
def entries(b):
    assert b[:4]==b'BHD5',b[:8]
    count,off=struct.unpack_from('<ii',b,0x10)
    for i in range(count):
        n,fo=struct.unpack_from('<ii',b,off+i*8)
        for j in range(n):
            hh,pad,unpad,fofs,sha,aes=struct.unpack_from('<QiiqqQ',b,fo+j*40)
            yield hh,pad,unpad,fofs,sha,aes
if __name__=='__main__':
    want=sys.argv[1:]
    hs={h(p):p for p in want}
    found={}
    for name in ['Data0','Data1','Data2','Data3','DLC']:
        b=decrypt(name)
        for hh,pad,unpad,fofs,sha,aes in entries(b):
            if hh in hs:
                rec={'archive':name,'offset':fofs,'padded':pad,'size':unpad}
                if aes:
                    k=b[aes:aes+16];cnt=struct.unpack_from('<i',b,aes+16)[0]
                    rec['aes']=k.hex();rec['ranges']=[struct.unpack_from('<qq',b,aes+20+16*r) for r in range(cnt)]
                found[hs[hh]]=rec
    print(json.dumps(found,indent=1))
