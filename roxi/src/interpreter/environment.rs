use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex},
};

use crate::interpreter::value::Value;

#[derive(Debug)]
pub struct Environment {
    parent: Option<Arc<Self>>,
    values: Mutex<HashMap<String, Value>>,
}

impl Environment {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            parent: None,
            values: Mutex::new(HashMap::new()),
        })
    }

    pub fn with_parent(parent: Arc<Environment>) -> Arc<Self> {
        Arc::new(Self {
            parent: Some(parent),
            values: Mutex::new(HashMap::new()),
        })
    }

    pub fn parent(&self) -> Option<&Arc<Self>> {
        self.parent.as_ref()
    }

    fn get_parent(&self, depth: usize) -> &Self {
        let mut this = self;
        for _ in 0..depth {
            this = this.parent.as_deref().unwrap();
        }
        this
    }

    pub fn define(&self, name: String, value: Value) {
        self.values.lock().unwrap().insert(name, value);
    }

    pub fn get(&self, name: &str, depth: usize) -> Option<Value> {
        self.get_parent(depth)
            .values
            .lock()
            .unwrap()
            .get(name)
            .cloned()
    }

    pub fn set(&self, name: &str, depth: usize, value: Value) {
        *self
            .get_parent(depth)
            .values
            .lock()
            .unwrap()
            .get_mut(name)
            .unwrap() = value;
    }

    pub fn names(&self) -> HashSet<String> {
        self.values.lock().unwrap().keys().cloned().collect()
    }
}
