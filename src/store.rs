use nix::fcntl;
use nix::fcntl::OFlag;
use nix::sys::stat::Mode;
use nix::unistd;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, os::fd::OwnedFd};

#[derive(Serialize, Deserialize, Debug)]
pub struct DiskMap {
    pub m: HashMap<String, String>,
}

impl DiskMap {
    pub fn new() -> Result<DiskMap, Box<dyn std::error::Error>> {
        let fd = fcntl::open(
            "/tmp/map",
            OFlag::O_RDWR | OFlag::O_CREAT,
            Mode::S_IRUSR | Mode::S_IWUSR | Mode::S_IRGRP | Mode::S_IROTH,
        )?;

        let data = DiskMap::read_data(fd)?;
        println!("{}.", data);

        Ok(DiskMap { m: HashMap::new() })
    }

    fn read_data(fd: OwnedFd) -> Result<String, Box<dyn std::error::Error>> {
        let mut buf = [0u8; 1024];
        let mut s = String::new();
        let mut n = unistd::read(&fd, &mut buf)?;

        while n != 0 {
            let chunk = str::from_utf8(&buf[..n]).expect("Valid UTF-8");
            s.push_str(chunk);
            n = unistd::read(&fd, &mut buf)?;
        }
        Ok(s)
    }

    pub fn set(&mut self, k: &str, v: &str) {
        self.m.insert(k.to_string(), v.to_string());
    }

    pub fn get(&self, k: &str) -> Option<&String> {
        self.m.get(k)
    }
}
