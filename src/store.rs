use std::collections::HashMap;

pub struct DiskMap {
    pub m: HashMap<String, String>,
}

impl DiskMap {
    pub fn new() -> DiskMap {
        DiskMap { m: HashMap::new() }
    }

    pub fn set(&mut self, k: &str, v: &str) {
        self.m.insert(k.to_string(), v.to_string());
    }

    pub fn get(&self, k: &str) -> Option<&String> {
        self.m.get(k)
    }
}
