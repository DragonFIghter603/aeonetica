use std::any::Any;
use aeonetica_engine::TypeId;
use aeonetica_engine::util::id_map::IdMap;
use aeonetica_engine::util::nullable::Nullable;
use aeonetica_engine::util::type_to_id;

pub struct DataStore {
    stores: IdMap<Box<dyn Any>>
}

impl DataStore {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            stores: Default::default()
        }
    }

    pub fn add_store<T: Sized + 'static>(&mut self, store: T) -> bool {
        if let std::collections::hash_map::Entry::Vacant(e) = self.stores.entry(type_to_id::<T>()) {
            e.insert(Box::new(store));
            true
        } else {
            false
        }
    }

    pub fn stores(&self) -> Vec<TypeId> {
        self.stores.keys().copied().collect()
    }

    pub fn remove_store<T: Sized + 'static>(&mut self) -> bool{
        self.stores.remove(&type_to_id::<T>()).is_some()
    }

    pub fn get_store<T: Sized + 'static>(&self) -> Nullable<&T> {
        self.stores.get(&type_to_id::<T>()).map(|m| unsafe { &*std::mem::transmute::<&Box<_>, &(*const T, usize)>(m).0 } ).into()
    }

    pub fn mut_store<T: Sized + 'static>(&mut self) -> Nullable<&mut T> {
        self.stores.get_mut(&type_to_id::<T>()).map(|m| unsafe { &mut*std::mem::transmute::<&Box<_>, &(*mut T, usize)>(m).0 }).into()
    }

    pub fn get_or_create<T: Sized + 'static, F: FnOnce() -> T>(&mut self, creator: F) -> &T {
        self.stores.get(&type_to_id::<T>()).map(|m| unsafe { &*std::mem::transmute::<&Box<_>, &(*const T, usize)>(m).0 } ).unwrap_or_else(||{
            self.add_store(creator());
            self.get_store::<T>().unwrap()
        })
    }

    pub fn mut_or_create<T: Sized + 'static, F: FnOnce() -> T>(&mut self, creator: F) -> &T {
        self.stores.get_mut(&type_to_id::<T>()).map(|m| unsafe { &*std::mem::transmute::<&Box<_>, &(*mut T, usize)>(m).0 } ).unwrap_or_else(||{
            self.add_store(creator());
            self.mut_store::<T>().unwrap()
        })
    }

    pub fn has_store<T: Sized + 'static>(&self) -> bool {
        self.stores.contains_key(&type_to_id::<T>())
    }

    pub fn has_store_type(&self, ty: &TypeId) -> bool {
        self.stores.contains_key(ty)
    }
}