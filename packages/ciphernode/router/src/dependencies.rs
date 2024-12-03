use std::any::Any;
use std::{collections::HashMap, marker::PhantomData};

pub struct DependencyKey<T> {
    name: &'static str,
    _phantom: PhantomData<T>,
}

impl<T> DependencyKey<T> {
    pub const fn new(name: &'static str) -> Self {
        Self {
            name,
            _phantom: PhantomData,
        }
    }
}

pub struct Dependencies {
    storage: HashMap<&'static str, Box<dyn Any + Send + Sync>>,
}

impl Dependencies {
    pub fn new() -> Self {
        Self {
            storage: HashMap::new(),
        }
    }

    pub fn insert<T: Send + Sync + 'static>(&mut self, key: DependencyKey<T>, dep: T) {
        self.storage.insert(key.name, Box::new(dep));
    }

    pub fn get<T: Send + Sync + 'static>(&self, key: DependencyKey<T>) -> Option<&T> {
        self.storage.get(key.name)?.downcast_ref()
    }

    pub fn contains(&self, name: &'static str) -> bool {
        self.storage.contains_key(name)
    }

    pub fn keys(&self) -> Vec<String> {
        self.storage.keys().map(|&k| k.to_string()).collect()
    }
}
