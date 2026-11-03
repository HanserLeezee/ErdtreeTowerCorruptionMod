const CENTER:[f32;2]=[-322.794,-307.975];
const RADIUS:f32=10.4;
const ENTRY:[f32;3]=[-320.291,-21.96,-315.732];

fn correction(p:[f32;3])->Option<[f32;3]> {
    if !p.iter().all(|v|v.is_finite()){return None;}
    let dx=p[0]-CENTER[0];let dz=p[2]-CENTER[1];
    let distance=dx.hypot(dz);
    if distance<=RADIUS&&p[1]>=-23.0{return None;}
    if distance>RADIUS+2.0||p[1]< -23.0{return Some(ENTRY);}
    let scale=(RADIUS-0.05)/distance;
    Some([CENTER[0]+dx*scale,p[1],CENTER[1]+dz*scale])
}

#[derive(Default)]
pub struct Boundary {armed:bool}
impl Boundary {
    pub fn tick(&mut self,world:&mut eldenring::cs::WorldChrMan,warping:bool) {
        let Some(player)=world.main_player.as_mut().filter(|p|!warping&&i32::from(p.current_block_id)==super::hub::BLOCK
            &&p.chr_ins.modules.data.hp>0&&!p.chr_ins.chr_flags1c5.death_flag()) else {self.armed=false;return;};
        if !self.armed {
            if !player.chr_ins.modules.physics.standing_on_solid_ground{return;}
            self.armed=true;
        }
        let p=&player.block_position;let current=[p.x,p.y,p.z];
        if let Some(at)=correction(current) {
            let physics=&mut player.chr_ins.modules.physics;
            physics.position.0+=at[0]-current[0];physics.position.1+=at[1]-current[1];physics.position.2+=at[2]-current[2];
            physics.last_update_position=physics.position;physics.chr_proxy_pos_update_requested=true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]fn grace_spawn_and_both_services_remain_accessible(){
        for p in [ENTRY,[-322.811,-21.985,-308.012],[-329.5,-21.9,-309.0],[-329.5,-21.9,-302.0],[-330.936,-21.985,-305.686]] {
            assert_eq!(correction(p),None,"{p:?}");
        }
    }
    #[test]fn walking_rolling_and_jumping_cannot_leave_any_side(){
        for angle in 0..360 {for y in [-22.0,-20.0,-17.0] {
            let a=(angle as f32).to_radians();
            let at=correction([CENTER[0]+a.cos()*11.0,y,CENTER[1]+a.sin()*11.0]).unwrap();
            assert!((at[0]-CENTER[0]).hypot(at[2]-CENTER[1])<RADIUS);
            assert_eq!(at[1],y);
        }}
    }
    #[test]fn old_side_room_saves_and_balcony_falls_return_to_entry(){
        for p in [[-305.236,-19.889,-297.818],[-340.0,-22.0,-315.0],[CENTER[0],-30.0,CENTER[1]]] {
            assert_eq!(correction(p),Some(ENTRY));
        }
    }
    #[test]fn nonfinite_coordinates_are_ignored(){
        for v in [f32::NAN,f32::INFINITY,f32::NEG_INFINITY] {assert_eq!(correction([v,-22.0,0.0]),None);}
    }
}
