
use std::cell::RefCell;
use std::collections::{HashSet};
use std::collections::hash_set::Iter;
use std::fmt::Debug;
use std::rc::Rc;
use aeonetica_engine::{Id, log};
use aeonetica_engine::nanoserde::{DeBin, SerBin};
use aeonetica_engine::networking::messaging::ClientHandle;
use aeonetica_engine::networking::server_packets::{ServerMessage, ServerPacket};
use aeonetica_engine::util::type_to_id;
use crate::ecs::{Module, Engine};
use crate::networking::NetworkServer;

pub(crate) struct MessagingSystem {
    pub(crate) ns: Rc<RefCell<NetworkServer>>,
    pub(crate) messengers: HashSet<Id>
}

impl MessagingSystem {
    pub(crate) fn new(ns: Rc<RefCell<NetworkServer>>) -> Self {
        Self {
            ns,
            messengers: Default::default()
        }
    }
}

pub trait Message: SerBin + DeBin + Debug {}

pub struct Messenger {
    ms: Option<Rc<RefCell<MessagingSystem>>>,
    handle_type: Id,
    entity_id: Id,
    pub(crate) receivers: HashSet<Id>,
    pub(crate) on_send: fn(id: &Id, engine: &mut Engine, sending: &mut Vec<u8>),
    pub(crate) on_receive: fn(id: &Id, engine: &mut Engine, user: &Id, receiving: &Vec<u8>),
}

impl Module for Messenger {
    fn start(id: &Id, engine: &mut Engine) where Self: Sized {
        engine.mut_module_of::<Self>(id).unwrap().ms = Some(engine.ms.clone());
        engine.mut_module_of::<Self>(id).unwrap().entity_id = *id;
        engine.ms.borrow_mut().messengers.insert(*id);
    }

    fn remove(id: &Id, engine: &mut Engine) where Self: Sized {
        engine.ms.borrow_mut().messengers.remove(id);
    }
}

impl Messenger {
    pub fn new<H: ClientHandle + Sized + 'static>(on_send: fn(id: &Id, engine: &mut Engine, sending: &mut Vec<u8>), on_receive: fn(id: &Id, engine: &mut Engine, user: &Id, receiving: &Vec<u8>)) -> Self {
        Self {
            ms: None,
            receivers: Default::default(),
            handle_type: type_to_id::<H>(),
            entity_id: Id::new(),
            on_send,
            on_receive
        }
    }

    pub fn clients(&self) -> Iter<Id> {
        self.receivers.iter()
    }

    pub fn has_client(&self, id: &Id) -> bool {
        self.receivers.contains(id)
    }

    pub fn add_client(&mut self, id: Id) -> bool {
        if !self.receivers.contains(&id) && self.ms.as_ref().unwrap().borrow().ns.borrow().clients.contains_key(&id) {
            self.receivers.insert(id);
            let _ = self.ms.as_ref().unwrap().borrow().ns.borrow().send(&id, &ServerPacket {
                conv_id: Id::new(),
                message: ServerMessage::AddClientHandle(self.entity_id, unsafe { self.handle_type }),
            });
            true
        } else { false }
    }

    pub fn remove_client(&mut self, id: &Id) -> bool {
        if self.receivers.contains(id) {
            self.receivers.remove(id);
            let _ = self.ms.as_ref().unwrap().borrow().ns.borrow().send(id, &ServerPacket {
                conv_id: Id::new(),
                message: ServerMessage::RemoveClientHandle(self.entity_id),
            });
            true
        } else { false }
    }
}