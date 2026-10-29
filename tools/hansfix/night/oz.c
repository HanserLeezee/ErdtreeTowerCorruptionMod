#include <windows.h>
#include <stdio.h>
#include <stdlib.h>
typedef long long (*Comp)(int,const void*,long long,void*,int,void*,void*,void*,void*,long long);
typedef long long (*Dec)(const void*,long long,void*,long long,int,int,int,void*,long long,void*,void*,void*,long long,int);
int main(int argc,char**argv){
  HMODULE h=LoadLibraryA("oo2core_6_win64.dll"); if(!h){printf("no dll\n");return 1;}
  FILE*f=fopen(argv[2],"rb"); fseek(f,0,2); long n=ftell(f); fseek(f,0,0); unsigned char*in=malloc(n); fread(in,1,n,f); fclose(f);
  if(argv[1][0]=='c'){
    Comp c=(Comp)GetProcAddress(h,"OodleLZ_Compress");
    unsigned char*out=malloc(n+65536); long long m=c(8,in,n,out,atoi(argv[4]),0,0,0,0,0);
    f=fopen(argv[3],"wb"); fwrite(out,1,m,f); fclose(f); printf("%ld -> %lld\n",n,m);
  } else {
    Dec d=(Dec)GetProcAddress(h,"OodleLZ_Decompress"); long long us=atoll(argv[4]);
    unsigned char*out=malloc(us+64); long long m=d(in,n,out,us,1,0,0,0,0,0,0,0,0,3);
    f=fopen(argv[3],"wb"); fwrite(out,1,m>0?m:0,f); fclose(f); printf("dec %lld\n",m);
  }
  return 0;
}
