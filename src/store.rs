extern crate flexbuffers;

use nix::fcntl;
use nix::fcntl::OFlag;
use nix::sys::stat::Mode;
use nix::unistd;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, os::fd::OwnedFd};

#[derive(Debug, PartialEq, Serialize, Deserialize)]
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
        if data.is_empty() {
            Ok(DiskMap { m: HashMap::new() })
        } else {
            let reader = flexbuffers::Reader::get_root(&data[..])?;
            let disk_map = DiskMap::deserialize(reader)?;
            Ok(disk_map)
        }
    }

    fn read_data(fd: OwnedFd) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let mut buf = [0u8; 1024];
        let mut v: Vec<u8> = vec![];

        loop {
            let n = unistd::read(&fd, &mut buf)?;
            if n == 0 {
                break;
            }

            v.extend_from_slice(&buf[..n]);
        }
        Ok(v)
    }

    pub fn set(&mut self, k: &str, v: &str) {
        self.m.insert(k.to_string(), v.to_string());
    }

    pub fn get(&self, k: &str) -> Option<&String> {
        self.m.get(k)
    }
}
