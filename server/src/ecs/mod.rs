
use std::collections::{HashMap, HashSet};
use std::collections::hash_set;
use std::collections::hash_map::{Iter, IterMut, Keys};


use crate::ecs::entity::Entity;
use aeonetica_engine::{ClientId, EntityId, Id, log};
use aeonetica_engine::networking::server_packets::{ServerMessage, ServerPacket};
use crate::ecs::events::ConnectionListener;

use crate::ecs::module::{Module, ModuleDyn};
use crate::ecs::scheduling::TaskQueue;
use crate::server_runtime::ServerRuntime;

pub mod module;
pub mod entity;
pub mod events;
pub mod messaging;
pub mod scheduling;

pub struct Engine {
    entites: HashMap<EntityId, Entity>,
    tagged: HashMap<String, EntityId>,
    tasks: TaskQueue,
    pub(crate) clients: HashSet<ClientId>,
    pub(crate) runtime: ServerRuntime,
    pub(crate) tick: usize
}

impl Engine {
    pub fn new(runtime: ServerRuntime) -> Self {
        Self {
            entites: Default::default(),
            tagged: Default::default(),
            clients: Default::default(),
            tasks: TaskQueue::default(),
            runtime,
            tick: 0
        }
    }

    pub fn new_entity(&mut self) -> EntityId {
        let e = Entity::new(self);
        let id = e.entity_id;
        self.entites.insert(id, e);
        id
    }

    /// Retuns true if user got successfully kicked.
    /// Kick fails if the client is not joined.
    /// Kicking will also unregister the client.
    ///
    /// Note: The user might be registered but not logged in.
    /// In that case kicking fails and the client will not be unregistered.
    ///
    /// Since the registration of clients is purely network related it can be
    /// disregarded for most use cases.
    pub fn kick_client(&mut self, id: &ClientId, reason: &str) -> bool {
        if self.clients.contains(id) {
            self.clients.remove(id);
            self.for_each_module_of_type::<ConnectionListener, _>(|engine, eid,  m| {
                (m.on_leave)(eid, engine, id);
            });
            let _ = self.runtime.ns.borrow().send(id, &ServerPacket {
                conv_id: Id::new(),
                message: ServerMessage::Kick(reason.to_string()),
            });
            log!("kicked client {id}");
            true
        } else { false }
    }

    pub fn is_client_logged_in(&self, id: &ClientId) -> bool {
        self.clients.contains(id)
    }

    pub fn clients(&self) -> hash_set::Iter<ClientId> {
        self.clients.iter()
    }

    pub(crate) fn for_each_module<F: Fn(&mut Self, &EntityId, &mut Box<dyn ModuleDyn>)>(&mut self, runner: F) {
        let mut_self_ref_ptr = self as *mut Self;
        for id in self.entites.keys().cloned().collect::<Vec<_>>() {
            if let Some(e) = self.entites.get_mut(&id) {
                for mid in e.modules.keys().cloned().collect::<Vec<_>>() {
                    if let Some(m) = e.modules.get_mut(&mid) {
                        runner(unsafe{ &mut *mut_self_ref_ptr }, &id,  m)
                    }
                }
            };
        }
    }

    pub fn for_each_module_of_type<T: Module + Sized + 'static, F: Fn(&mut Self, &EntityId, &mut T)>(&mut self, runner: F) {
        let mut_self_ref_ptr = self as *mut Self;
        for id in self.entites.keys().cloned().collect::<Vec<_>>() {
            if let Some(e) = self.entites.get_mut(&id) {
                if let Some(m) = e.mut_module::<T>() {
                    runner(unsafe{ &mut *mut_self_ref_ptr }, &id, m)
                }
            }
        }
    }

    pub fn remove_entity(&mut self, id: &EntityId) -> bool {
        let mut_self_ref_ptr = self as *mut Self;
        if let Some(e) = self.entites.get_mut(id) {
            for mid in e.modules.keys().cloned().collect::<Vec<_>>() {
                if let Some(m) = e.modules.get(&mid) {
                    m.remove_dyn(id, unsafe { &mut *mut_self_ref_ptr})
                }
            }
        };
        self.entites.remove(id).is_some()
    }

    /// Returns `true` if tagging is successful.
    /// Tagging fails if `tag_exists(tag)` returns true or `entity_exists(id)` returns false.
    pub fn tag_entity(&mut self, id: EntityId, tag: String) -> bool {
        if !self.tag_exists(&tag) && self.entity_exists(&id) {
            self.tagged.insert(tag, id);
            true
        } else {
            false
        }
    }

    /// Returns `true` if tag exists and `entity_exists(id)` returns true.
    /// Tags whose entity got removed are treated as nonexistent and can be overridden
    pub fn tag_exists(&self, tag: &str) -> bool {
       self.tagged.get(tag).map(|id| self.entity_exists(id)).unwrap_or(false)
    }

    /// Returns `true` if tag existed.
    /// Removal fails if `tag_exists(tag)` returns false.
    pub fn remove_tag(&mut self, tag: &str) -> bool {
        if self.tag_exists(tag) {
            self.tagged.remove(tag);
            true
        } else {
            false
        }
    }

    pub fn get_entity_by_tag(&self, tag: &str) -> Option<&Entity> {
        self.entites.get(self.tagged.get(tag)?)
    }

    pub fn mut_entity_by_tag(&mut self, tag: &str) -> Option<&mut Entity> {
        self.entites.get_mut(self.tagged.get(tag)?)
    }

    pub fn entity_exists(&self, id: &EntityId) -> bool {
        self.entites.contains_key(id)
    }

    pub fn get_entity(&self, id: &EntityId) -> Option<&Entity> {
        self.entites.get(id)
    }

    pub fn mut_entity(&mut self, id: &EntityId) -> Option<&mut Entity> {
        self.entites.get_mut(id)
    }

    pub fn get_module_of<T: Module + Sized + 'static>(&self, id: &EntityId) -> Option<&T> {
        self.entites.get(id)?.get_module()
    }

    pub fn mut_module_of<T: Module + Sized + 'static>(&mut self, id: &EntityId) -> Option<&mut T> {
        self.entites.get_mut(id)?.mut_module()
    }

    pub fn get_module_by_tag<T: Module + Sized + 'static>(&self, tag: &str) -> Option<&T> {
        self.get_entity_by_tag(tag)?.get_module()
    }

    pub fn mut_module_by_tag<T: Module + Sized + 'static>(&mut self, tag: &str) -> Option<&mut T> {
        self.mut_entity_by_tag(tag)?.mut_module()
    }

    pub fn ids(&self) -> Keys<EntityId, Entity>{
        self.entites.keys()
    }

    pub fn iter(&self) -> Iter<EntityId, Entity>{
        self.entites.iter()
    }

    pub fn iter_mut(&mut self) -> IterMut<EntityId, Entity>{
        self.entites.iter_mut()
    }

    #[allow(clippy::type_complexity)]
    pub fn id_find_with<T: Module + Sized + 'static>(&self) -> impl Iterator<Item = &EntityId>{
        self.entites.iter().filter_map(|(id, e)| if e.has_module::<T>() { Some(id)} else { None })
    }

    #[allow(clippy::type_complexity)]
    pub fn find_with<T: Module + Sized + 'static>(&self) -> impl Iterator<Item = (&EntityId, &T)>{
        self.entites.iter().filter_map(|(id, e)| if e.has_module::<T>() { Some((id, e.get_module::<T>()?))} else { None })
    }
}
