//! [HansFix] 堕落装备的游戏内文字：在内存里把这件装备的名字、简介、说明换成改写后的版本。
//! MsgRepositoryImp 布局（Erd-Tools 记录）：+0x08 → 版本数组，[0] → 文本表数组，[表编号] → FMG。
//! 内存里的 FMG 与文件相同，只有 +0x18 的“字符串偏移表位置”被换成了绝对地址；
//! 偏移表每项是相对 FMG 开头的偏移（若遇到绝对地址也能识别），改写就是让这一项指向我们自己的 UTF-16 字符串。
//! 所有读写都经 Read/WriteProcessMemory，任何一步对不上就放弃，不碰游戏。
use std::{ffi::c_void,sync::Mutex};

#[link(name="kernel32")]
unsafe extern "system" {
    fn ReadProcessMemory(process:*mut c_void,address:*const c_void,buffer:*mut c_void,size:usize,read:*mut usize)->i32;
    fn WriteProcessMemory(process:*mut c_void,address:*mut c_void,buffer:*const c_void,size:usize,written:*mut usize)->i32;
}
const SELF:*mut c_void=-1isize as *mut c_void;
fn read_bytes(address:usize,buffer:&mut [u8])->bool {
    if address<0x10000 {return false;}
    let mut n=0;unsafe {ReadProcessMemory(SELF,address as *const c_void,buffer.as_mut_ptr().cast(),buffer.len(),&mut n)!=0&&n==buffer.len()}
}
fn read_u64(address:usize)->Option<u64> {let mut b=[0;8];read_bytes(address,&mut b).then(||u64::from_le_bytes(b))}
fn read_u32(address:usize)->Option<u32> {let mut b=[0;4];read_bytes(address,&mut b).then(||u32::from_le_bytes(b))}
fn write_u64(address:usize,value:u64)->bool {
    let mut n=0;unsafe {WriteProcessMemory(SELF,address as *mut c_void,(&raw const value).cast(),8,&mut n)!=0&&n==8}
}

/// 名字、简介、说明三类表（本体、DLC1、DLC2）
#[derive(Clone,Copy,PartialEq,Eq,Debug)]
pub enum Part {Name,Info,Caption}
const WEAPON:[(u32,Part);9]=[(11,Part::Name),(21,Part::Info),(25,Part::Caption),(0x136,Part::Name),(0x137,Part::Info),(0x138,Part::Caption),(0x19a,Part::Name),(0x19b,Part::Info),(0x19c,Part::Caption)];
const PROTECTOR:[(u32,Part);9]=[(12,Part::Name),(22,Part::Info),(26,Part::Caption),(0x139,Part::Name),(0x13a,Part::Info),(0x13b,Part::Caption),(0x19d,Part::Name),(0x19e,Part::Info),(0x19f,Part::Caption)];

struct Fmg {base:usize,size:usize,table:usize,groups:Vec<(i32,i32,i32)>}
fn repository()->Option<usize> {
    use fromsoftware_shared::FromStatic;
    unsafe {eldenring::cs::MsgRepositoryImp::instance()}.ok().map(|r|std::ptr::from_ref(r) as usize)
}
fn fmg(repo:usize,index:u32)->Option<Fmg> {
    let count=read_u32(repo+0x14)?;
    if index>=count||count>0x1000 {return None;}
    let versions=read_u64(repo+8)? as usize;
    let tables=read_u64(versions)? as usize;
    let base=read_u64(tables+index as usize*8)? as usize;
    if base<0x10000 {return None;}
    let mut head=[0u8;0x28];
    if !read_bytes(base,&mut head)||head[0]!=0||head[1]!=0||head[2]!=2 {return None;}
    let size=u32::from_le_bytes(head[4..8].try_into().ok()?) as usize;
    let groups=i32::from_le_bytes(head[0x0c..0x10].try_into().ok()?);
    let strings=i32::from_le_bytes(head[0x10..0x14].try_into().ok()?);
    let table=u64::from_le_bytes(head[0x18..0x20].try_into().ok()?) as usize;
    if !(0x28..0x1000_0000).contains(&size)||!(0..0x10_0000).contains(&groups)||!(0..0x100_0000).contains(&strings) {return None;}
    // 偏移表应该在 FMG 内；文件里的相对偏移也接受
    let table=if (base..base+size).contains(&table) {table} else if table<size {base+table} else {return None};
    let mut raw=vec![0u8;groups as usize*16];
    if !read_bytes(base+0x28,&mut raw) {return None;}
    let groups=raw.chunks_exact(16).map(|c|(i32::from_le_bytes(c[0..4].try_into().unwrap()),i32::from_le_bytes(c[4..8].try_into().unwrap()),i32::from_le_bytes(c[8..12].try_into().unwrap()))).collect::<Vec<_>>();
    if groups.iter().any(|&(index,first,last)|index<0||last<first||index+(last-first)>=strings) {return None;}
    Some(Fmg{base,size,table,groups})
}
impl Fmg {
    fn slots(&self,range:std::ops::Range<u32>)->Vec<(u32,usize)> {
        // FMG 的分组按编号升序：二分跳到第一个可能相交的分组（词条装备的名字表有上万个分组）
        let sorted=self.groups.windows(2).all(|w|w[0].1<=w[1].1);
        let start=if sorted {self.groups.partition_point(|&(_,_,last)|(last as i64)<range.start as i64)} else {0};
        self.groups[start..].iter().take_while(|&&(_,first,_)|!sorted||(first as i64)<range.end as i64).flat_map(|&(index,first,last)|{
            let (lo,hi)=((first as i64).max(range.start as i64),(last as i64).min(range.end as i64-1));
            (lo..=hi).map(move|id|(id as u32,self.table+(index as i64+id-first as i64) as usize*8))
        }).collect()
    }
    fn resolve(&self,value:u64)->Option<(usize,bool)> {
        let v=value as usize;
        if (self.base..self.base+self.size).contains(&v) {Some((v,true))}
        else if value>0&&(value as usize)<self.size {Some((self.base+v,false))}
        else {None}
    }
    fn text(&self,slot:usize)->Option<(String,bool,u64)> {
        let value=read_u64(slot)?;let (address,absolute)=self.resolve(value)?;
        // r8：按块读（原来每个字符一次系统调用，几十件堕落装备时一次刷新要卡 1–2 秒）
        let mut units=Vec::new();let mut done=false;
        while !done&&units.len()<8192 {
            let at=address+units.len()*2;
            // 不跨页读，避免块尾落在未映射的页上
            let room=(0x1000-(at&0xfff)).min(512);
            let mut b=vec![0u8;room&!1];
            if b.is_empty()||!read_bytes(at,&mut b) {
                let mut one=[0;2];if !read_bytes(at,&mut one){return None;}b=one.to_vec();
            }
            for c in b.chunks_exact(2) {let u=u16::from_le_bytes([c[0],c[1]]);if u==0 {done=true;break;}units.push(u);}
        }
        Some((String::from_utf16_lossy(&units),absolute,value))
    }
}

#[derive(Default)]
struct State {buffers:Vec<Box<[u16]>>,warned:bool}
static STATE:Mutex<Option<State>>=Mutex::new(None);

/// 把一组改过的文字项写回原值（按写入的逆序）
pub fn restore_slots(slots:&[(usize,u64)]) {
    for &(slot,value) in slots.iter().rev() {write_u64(slot,value);}
    // 旧字符串留着：菜单可能还拿着指针，量很小
}
/// 一批改写：每张文本表只解析一次
pub struct Batch {repo:Option<usize>,cache:std::collections::HashMap<u32,Option<Fmg>>}
impl Batch {
    pub fn new()->Self {Self{repo:repository(),cache:std::collections::HashMap::new()}}
    /// 给一件堕落装备换文字；返回改过的 (偏移表项地址, 原值)，用于之后还原
    pub fn apply(&mut self,cursed:&erdtree_tower::descent::Cursed)->Vec<(usize,u64)> {
        let Some(repo)=self.repo else{return vec![]};
        let Ok(mut guard)=STATE.lock() else{return vec![]};
        let state=guard.get_or_insert_with(State::default);
        apply_one(state,repo,&mut self.cache,cursed)
    }
}
fn apply_one(state:&mut State,repo:usize,cache:&mut std::collections::HashMap<u32,Option<Fmg>>,cursed:&erdtree_tower::descent::Cursed)->Vec<(usize,u64)> {
    let weapon=cursed.item>>28==0;let first=cursed.item&0x0fff_ffff;
    // 武器的质变名（first+100…）也有名字项
    let range=if weapon {first..first+10000} else {first..first+1};
    let mut changed=Vec::new();let mut seen=0;
    for &(index,part) in if weapon {&WEAPON} else {&PROTECTOR} {
        let Some(table)=cache.entry(index).or_insert_with(||fmg(repo,index)).as_ref() else{continue};seen+=1;
        for (id,slot) in table.slots(range.clone()) {
            if part!=Part::Name&&id!=first {continue;}
            let Some((text,absolute,value))=table.text(slot) else{continue};
            if text.trim().is_empty() {continue;}
            let replaced=match part {
                Part::Name=>erdtree_tower::descent::game_name(&text),
                Part::Info=>erdtree_tower::descent::game_info(&text,cursed),
                Part::Caption=>erdtree_tower::descent::game_caption(&text,cursed),
            };
            if replaced==text {continue;}
            let buffer:Box<[u16]>=replaced.encode_utf16().chain(Some(0)).collect();
            let address=buffer.as_ptr() as usize;
            let new=if absolute {address as u64} else {(address as u64).wrapping_sub(table.base as u64)};
            if write_u64(slot,new) {changed.push((slot,value));state.buffers.push(buffer);}
        }
    }
    if seen==0&&!state.warned {crate::log("HANSFIX_TEXT message tables not found; in-game text unchanged");state.warned=true;}
    changed
}
#[test]
fn strings_are_read_in_chunks_up_to_the_terminator() {
    let text:String="堕落的史诗·大龙爪".repeat(40);
    let mut blob:Vec<u8>=vec![0;16];
    let at=blob.len();blob.extend(text.encode_utf16().chain(Some(0)).flat_map(u16::to_le_bytes));blob.extend([0x41,0,0x42,0]);
    let base=blob.as_ptr() as usize;
    let slot:Box<u64>=Box::new(at as u64);
    let fmg=Fmg{base,size:blob.len(),table:0,groups:vec![]};
    let (read,absolute,_)=fmg.text(std::ptr::from_ref(&*slot) as usize).unwrap();
    assert_eq!(read,text);assert!(!absolute);
}
