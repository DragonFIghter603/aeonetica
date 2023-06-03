use aeonetica_engine::{math::vector::Vector2, EntityId, networking::SendMode};
use aeonetica_server::{ServerMod, ecs::{module::Module, Engine, messaging::Messenger}};
use player_mod::server::{PLAYER_HANDLER, PlayerHandler, Player};
use world_mod::{server::world::{WORLD, World}, common::WorldView};
use crate::client::WormHandle;


pub struct WormsModServer {

}

impl WormsModServer {
    pub(crate) fn new() -> Self {
        Self {}
    }
}

impl ServerMod for WormsModServer {
    fn start(&mut self, engine: &mut Engine) {
        Worm::create(engine);
    }
}

const SEG_LEN: f32 = 0.8;
pub(crate) const WORM_SPEED: f32 = 0.25;

pub(crate) struct Worm {
    ppos: Vector2<f32>,
    looking_dir: Vector2<f32>,
    segments: Vec<Vector2<f32>>
}

impl Worm {
    fn create(engine: &mut Engine) -> EntityId {
        let eid = engine.new_entity();
        
        let mut entity = engine.mut_entity(&eid);
        entity.add_module(Worm::new(Vector2::new(-15.0, 0.0), Vector2::new(1.0,  0.0), 10));
        entity.add_module(Messenger::new::<WormHandle>());
        eid
    }

    fn new(pos: Vector2<f32>, dir: Vector2<f32>, segs: usize) -> Self {
        Self {
            ppos: Default::default(),
            looking_dir: Vector2::new(1.0, 0.0),
            segments: {
                let dir = dir.normalized();
                let mut segments = vec![];
                for i in 0..segs {
                    segments.push(pos + dir * SEG_LEN * i as f32);
                }
                segments
            }
        }
    }
}

impl Module for Worm {
    fn tick(id: &EntityId, engine: &mut Engine) {
        let prcrc = engine.mut_module_by_tag::<PlayerHandler>(PLAYER_HANDLER).players.clone();
        let players = prcrc.borrow_mut();
        let worm = engine.get_module_of::<Worm>(id);
        let mut self_pos = worm.segments[0];
        let ppos = worm.ppos;
        let mut target = None;
        let mut pdsq = f32::MAX;
        for (pid, epid) in players.iter() {
            let pos = engine.get_module_of::<Player>(epid).position;
            let dsq = (pos - self_pos).mag_sq();
            if dsq > 24.0*24.0 {
                engine.mut_module_of::<Messenger>(id).remove_client(pid);

            } else {
                if dsq < pdsq {
                    target = Some(pos);
                    pdsq = dsq;
                }
                if engine.mut_module_of::<Messenger>(id).add_client(*pid) {
                    println!("ADDDD");
                    let (mut messenger, worm) = engine.two_mut_modules_of::<Messenger, Worm>(id);
                    messenger.call_client_fn(WormHandle::receive_position, (worm.segments.clone(), worm.looking_dir, true), SendMode::Safe);
                }
            }
        }
        if let Some(pos) = target {
            let wid = **engine.get_entity_id_by_tag(WORLD);
            let (mut worm, mut world) = engine.two_mut_modules_of_entities::<Worm, World>(id, &wid);
            let dir = (pos - self_pos).normalized();
            let dir = (worm.looking_dir * 3.0 + dir).normalized();
            worm.looking_dir = dir;
            world.calc_move(&mut worm.segments[0], (1.0, 1.0).into(), dir * WORM_SPEED);
            self_pos = worm.segments[0];
            let mut last_segment = self_pos;
            for segment in worm.segments.iter_mut().skip(1) {
                let d = (last_segment - *segment).normalized();
                *segment = last_segment - d * SEG_LEN;
                last_segment = *segment;
            }
        }

        if (ppos - self_pos).mag_sq() > 0.05 {
            let (mut messenger, mut worm) = engine.two_mut_modules_of::<Messenger, Worm>(id);
            worm.ppos = self_pos;
            messenger.call_client_fn(WormHandle::receive_position, (worm.segments.clone(), worm.looking_dir, false), SendMode::Safe);
        }
    }
}