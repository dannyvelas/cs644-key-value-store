extern crate flexbuffers;

use nix::fcntl::OFlag;
use nix::fcntl::{self, FlockArg};
use nix::sys::stat::Mode;
use nix::unistd::{self};
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::os::fd::AsFd;
use std::{collections::HashMap, os::fd::OwnedFd};

pub struct DiskMap {
    pub m: HashMap<String, String>,
    fd: OwnedFd,
}

impl DiskMap {
    pub fn new(file_path: &str) -> Result<DiskMap, Box<dyn Error>> {
        let fd = fcntl::open(
            file_path,
            OFlag::O_RDONLY | OFlag::O_CREAT,
            Mode::S_IRUSR | Mode::S_IWUSR | Mode::S_IRGRP | Mode::S_IROTH,
        )?;

        let (new_fd, data) = DiskMap::read(fd)?;
        let m = if data.is_empty() {
            HashMap::new()
        } else {
            let reader = flexbuffers::Reader::get_root(&data[..])?;
            HashMap::deserialize(reader)?
        };

        Ok(DiskMap { m, fd: new_fd })
    }

    fn read(fd: OwnedFd) -> Result<(OwnedFd, Vec<u8>), Box<dyn std::error::Error>> {
        let mut buf = [0u8; 1024];
        let mut v: Vec<u8> = Vec::new();

        let lock = fcntl::Flock::lock(fd, FlockArg::LockShared).map_err(|(_, e)| e)?;
        loop {
            let n = unistd::read(lock.as_fd(), &mut buf)?;
            if n == 0 {
                break;
            }

            v.extend_from_slice(&buf[..n]);
        }
        let new_fd = lock.unlock().map_err(|(_, e)| e)?;

        Ok((new_fd, v))
    }

    fn write(mut self) -> Result<usize, Box<dyn Error>> {
        // serialize hashmap
        let mut s = flexbuffers::FlexbufferSerializer::new();
        self.m.serialize(&mut s)?;

        // acquire exclusive lock
        let lock = fcntl::Flock::lock(self.fd, FlockArg::LockExclusive).map_err(|(_, e)| e)?;

        // self.truncate file
        unistd::ftruncate(lock.as_fd(), 0)?;

        // write
        let n = unistd::write(lock.as_fd(), s.view())?;

        // release lock
        let new_fd = lock.unlock().map_err(|(_, e)| e)?;
        self.fd = new_fd;

        Ok(n)
    }

    pub fn set(mut self, k: &str, v: &str) -> Result<usize, Box<dyn Error>> {
        self.m.insert(k.to_string(), v.to_string());
        self.write()
    }

    pub fn get(&self, k: &str) -> Option<&String> {
        self.m.get(k)
    }
}
