use std::any::Any;
use std::{collections::HashMap, marker::PhantomData};

/// A key that is associated to a type within the HetrogenousMap given by the generic parameter T
pub struct TypedKey<T> {
    name: &'static str,
    _phantom: PhantomData<T>,
}

impl<T> TypedKey<T> {
    pub const fn new(name: &'static str) -> Self {
        Self {
            name,
            _phantom: PhantomData,
        }
    }
}

/// A map that accepts hetrogenous data and stores it in a typesafe way using a typed key
pub struct HetrogenousMap {
    storage: HashMap<&'static str, Box<dyn Any + Send + Sync>>,
}

impl HetrogenousMap {
    pub fn new() -> Self {
        Self {
            storage: HashMap::new(),
        }
    }

    /// Insert data of type T
    pub fn insert<T: Send + Sync + 'static>(&mut self, key: TypedKey<T>, dep: T) {
        self.storage.insert(key.name, Box::new(dep));
    }

    /// Get data of type T
    pub fn get<T: Send + Sync + 'static>(&self, key: TypedKey<T>) -> Option<&T> {
        self.storage.get(key.name)?.downcast_ref()
    }

    /// Search for data that holds data under the given key name
    pub fn contains(&self, name: &'static str) -> bool {
        self.storage.contains_key(name)
    }

    /// Get a list of all key names
    pub fn keys(&self) -> Vec<String> {
        self.storage.keys().map(|&k| k.to_string()).collect()
    }
}
