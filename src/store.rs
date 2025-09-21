use std::collections::HashMap;

pub struct DiskMap<'a> {
    pub m: HashMap<&'a str, String>,
}

impl<'a> DiskMap<'a> {
    pub fn new() -> DiskMap<'a> {
        DiskMap { m: HashMap::new() }
    }

    pub fn set(&mut self, k: &'a str, v: &'a str) {
        self.m.insert(k, v.to_string());
    }

    pub fn get(&self, k: &'a str) -> Option<&String> {
        self.m.get(k)
    }
}
