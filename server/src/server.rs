use std::time::{Duration, Instant};
use aeonetica_engine::{log, log_err};
use crate::ecs::Engine;
use crate::server_runtime::ServerRuntime;

const TPS: usize = 20;

pub fn run(ip: &str) {
    let runtime = ServerRuntime::create(ip).map_err(|e| {
        e.log_exit();
    }).unwrap();

    log!("running start for all mods");
    let mut engine = Engine::new(runtime);

    let mut_engine_ref = unsafe { &mut *(&mut engine as *mut Engine) };
    engine.runtime.loaded_mods.iter_mut().for_each(|m| {
        m.start(mut_engine_ref);
    });

    loop {
        let t = Instant::now();

        let _ = engine.handle_queued().map_err(|e| {
            log_err!("{e}")
        });

        engine.timeout_inactive();

        engine.for_each_module(|engine, id, m| m.tick_dyn(id, engine));
        engine.run_tasks();
        engine.tick += 1;

        let to_wait = 1_000_000_000 / TPS - t.elapsed().as_nanos() as usize;
        std::thread::sleep(Duration::from_nanos(to_wait as u64));
    }
}