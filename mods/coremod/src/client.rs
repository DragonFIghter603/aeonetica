use std::collections::HashMap;
use aeonetica_client::ClientMod;
use aeonetica_engine::{Id};
use aeonetica_engine::networking::messaging::ClientHandle;

pub struct CoreModClient {

}

impl ClientMod for CoreModClient {
    fn init(&mut self, _flags: &Vec<String>) {
    }

    fn register_handlers(&self, handlers: &mut HashMap<Id, fn() -> Box<dyn ClientHandle>>) {

    }
}