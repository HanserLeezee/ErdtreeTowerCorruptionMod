const CENTER:[f32;2]=[-110.05,14.96];
const TANGENT:[f32;2]=[-0.819152,0.573576];
const NORMAL:[f32;2]=[0.573576,0.819152];
const HALF_WIDTH:f32=5.2;
const CLEARANCE:f32=0.8;
fn coordinates(p:[f32;3])->(f32,f32) {
    let delta=[p[0]-CENTER[0],p[2]-CENTER[1]];
    (delta[0]*TANGENT[0]+delta[1]*TANGENT[1],delta[0]*NORMAL[0]+delta[1]*NORMAL[1])
}
fn correction(previous:Option<[f32;3]>,current:[f32;3])->Option<[f32;3]> {
    if !current.iter().all(|v|v.is_finite()){return None;}
    let (along,distance)=coordinates(current);
    let mut hit=None;
    if let Some(before)=previous.filter(|p|p.iter().all(|v|v.is_finite())) {
        let (_,old_distance)=coordinates(before);
        if old_distance>=CLEARANCE&&distance<CLEARANCE {
            let t=(old_distance-CLEARANCE)/(old_distance-distance);
            let at=std::array::from_fn(|i|before[i]+(current[i]-before[i])*t);
            if coordinates(at).0.abs()<=HALF_WIDTH&&(20.0..=32.0).contains(&at[1]) {hit=Some(at);}
        }
    }
    if hit.is_none()&&along.abs()<=HALF_WIDTH&&(-2.0..CLEARANCE).contains(&distance)&&(20.0..=32.0).contains(&current[1]) {
        hit=Some([current[0]+NORMAL[0]*(CLEARANCE-distance),current[1],current[2]+NORMAL[1]*(CLEARANCE-distance)]);
    }
    hit
}
#[derive(Default)]
pub struct Barrier {previous:Option<[f32;3]>}
impl Barrier {
    pub fn reset(&mut self){self.previous=None;}
    pub fn apply(&mut self,player:&mut eldenring::cs::PlayerIns) {
        let p=&player.block_position;let current=[p.x,p.y,p.z];
        let corrected=correction(self.previous,current);
        self.previous=Some(corrected.unwrap_or(current));
        if let Some(at)=corrected {
            let physics=&mut player.chr_ins.modules.physics;
            physics.position.0+=at[0]-current[0];physics.position.1+=at[1]-current[1];physics.position.2+=at[2]-current[2];
            physics.last_update_position=physics.position;physics.chr_proxy_pos_update_requested=true;
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn point(along:f32,distance:f32,y:f32)->[f32;3]{[CENTER[0]+TANGENT[0]*along+NORMAL[0]*distance,y,CENTER[1]+TANGENT[1]*along+NORMAL[1]*distance]}
    #[test]fn blocks_walk_roll_and_jump_through_side_door(){
        for y in [21.5,23.0,25.0,30.0] {for distance in [0.0,-1.0,-8.0] {
            let at=correction(Some(point(0.0,4.0,y)),point(0.0,distance,y)).unwrap();
            assert!((coordinates(at).1-CLEARANCE).abs()<0.001);
        }}
    }
    #[test]fn main_gate_spawn_and_unrelated_elevations_remain_open(){
        for p in [[-110.85,21.54,23.06],[-93.97,21.54,19.71],point(8.0,0.0,21.5),point(0.0,0.0,40.0)] {assert!(correction(None,p).is_none());}
    }
    #[test]fn leaving_the_side_path_is_not_trapped_and_invalid_data_is_ignored(){
        assert!(correction(Some(point(0.0,-3.0,21.5)),point(0.0,3.0,21.5)).is_none());
        assert!(correction(None,[f32::NAN,21.5,15.0]).is_none());
        assert!(correction(None,point(0.0,0.0,21.5)).is_some());
    }
}
