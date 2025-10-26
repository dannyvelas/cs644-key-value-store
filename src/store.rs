extern crate flexbuffers;

use flexbuffers::FlexbufferSerializer;
use nix::fcntl::OFlag;
use nix::fcntl::{self, FlockArg};
use nix::sys::stat::Mode;
use nix::unistd;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::error;
use std::os;
use std::os::fd::AsFd;

pub struct DiskMap {
    pub m: HashMap<String, String>,
    fd: os::fd::OwnedFd,
    file_path: String,
}

impl DiskMap {
    pub fn new(file_path: &str) -> Result<DiskMap, Box<dyn error::Error>> {
        let fd = fcntl::open(
            file_path,
            OFlag::O_RDWR | OFlag::O_CREAT,
            Mode::S_IRUSR | Mode::S_IWUSR | Mode::S_IRGRP | Mode::S_IROTH,
        )?;

        let (new_fd, data) = DiskMap::read(fd)?;
        let m = if data.is_empty() {
            HashMap::new()
        } else {
            let reader = flexbuffers::Reader::get_root(&data[..])?;
            HashMap::deserialize(reader)?
        };

        Ok(DiskMap {
            m,
            fd: new_fd,
            file_path: String::from(file_path),
        })
    }

    fn read(fd: os::fd::OwnedFd) -> Result<(os::fd::OwnedFd, Vec<u8>), Box<dyn error::Error>> {
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

    pub fn write(self) -> Result<usize, Box<dyn error::Error>> {
        // serialize hashmap
        let mut s = flexbuffers::FlexbufferSerializer::new();
        self.m.serialize(&mut s)?;

        // consume and replace fd
        let n = DiskMap::write_lock(self.fd, s)?;

        Ok(n)
    }

    fn write_lock(
        fd: os::fd::OwnedFd,
        s: FlexbufferSerializer,
    ) -> Result<usize, Box<dyn error::Error>> {
        // acquire exclusive lock
        let lock = fcntl::Flock::lock(fd, FlockArg::LockExclusive).map_err(|(_, e)| e)?;

        // self.truncate file
        unistd::ftruncate(lock.as_fd(), 0)?;

        // write
        let n = unistd::write(lock.as_fd(), s.view())?;

        // release lock
        let _ = lock.unlock().map_err(|(_, e)| e)?;

        Ok(n)
    }

    pub fn set(&mut self, k: &str, v: &str) {
        self.m.insert(k.to_string(), v.to_string());
    }

    pub fn get(&self, k: &str) -> Option<&String> {
        self.m.get(k)
    }

    pub fn size(&self) -> Result<(), Box<dyn error::Error>> {
        match unsafe { unistd::fork() } {
            Ok(unistd::ForkResult::Parent { child, .. }) => {
                match nix::sys::wait::waitpid(child, None) {
                    Ok(_) => Ok(()),
                    Err(err) => Err(format!("error waiting: {}", err).into()),
                }
            }
            Ok(unistd::ForkResult::Child) => {
                let path = std::ffi::CString::new("/usr/bin/wc")?;
                let arg1 = std::ffi::CString::new("-c")?;
                let arg2 = std::ffi::CString::new(self.file_path.clone())?;
                let argv = [&path, &arg1, &arg2];
                let Err(err) = nix::unistd::execv(&path, &argv);
                // if we're here, it means execv failed. if it succeeded then the code from this process would
                // have been replaced by now
                eprintln!("execv failed: {}", err);
                std::process::exit(1);
            }
            Err(err_no) => Err(err_no.into()),
        }
    }
}
