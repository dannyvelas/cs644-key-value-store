extern crate flexbuffers;

use nix::fcntl;
use nix::fcntl::OFlag;
use nix::sys::stat::Mode;
use nix::unistd::{self};
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::{collections::HashMap, os::fd::OwnedFd};

pub struct DiskMap {
    pub m: HashMap<String, String>,
    file_path: String,
}

impl DiskMap {
    pub fn new(file_path: &str) -> Result<DiskMap, Box<dyn Error>> {
        let fd = fcntl::open(
            file_path,
            OFlag::O_RDONLY | OFlag::O_CREAT,
            Mode::S_IRUSR | Mode::S_IWUSR | Mode::S_IRGRP | Mode::S_IROTH,
        )?;

        let data = DiskMap::read(&fd)?;
        let m = if data.is_empty() {
            HashMap::new()
        } else {
            let reader = flexbuffers::Reader::get_root(&data[..])?;
            HashMap::deserialize(reader)?
        };

        Ok(DiskMap {
            m,
            file_path: file_path.to_string(),
        })
    }

    fn read(fd: &OwnedFd) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let mut buf = [0u8; 1024];
        let mut v: Vec<u8> = Vec::new();

        loop {
            let n = unistd::read(fd, &mut buf)?;
            if n == 0 {
                break;
            }

            v.extend_from_slice(&buf[..n]);
        }
        Ok(v)
    }

    pub fn write(self) -> Result<usize, Box<dyn Error>> {
        // serialize hashmap
        let mut s = flexbuffers::FlexbufferSerializer::new();
        self.m.serialize(&mut s)?;

        // open and truncate file
        let fd = fcntl::open(
            self.file_path.as_str(),
            OFlag::O_WRONLY | OFlag::O_CREAT | OFlag::O_TRUNC,
            Mode::S_IRUSR | Mode::S_IWUSR | Mode::S_IRGRP | Mode::S_IROTH,
        )?;

        // write
        let n = unistd::write(fd, s.view())?;

        Ok(n)
    }

    pub fn set(&mut self, k: &str, v: &str) {
        self.m.insert(k.to_string(), v.to_string());
    }

    pub fn get(&self, k: &str) -> Option<&String> {
        self.m.get(k)
    }
}
