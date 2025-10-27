extern crate flexbuffers;

use flexbuffers::FlexbufferSerializer;
use nix::fcntl::OFlag;
use nix::fcntl::{self, FlockArg};
use nix::sys::stat::Mode;
use nix::unistd::{self, close, dup2_stdout};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::error;
use std::ops::Deref;
use std::os;
use std::os::fd::AsFd;

pub struct ReadResult {
    fd: os::fd::OwnedFd,
    m: HashMap<String, String>,
}

pub struct DiskMap {
    file_path: String,
}

impl DiskMap {
    pub fn new(file_path: &str) -> Result<DiskMap, Box<dyn error::Error>> {
        Ok(DiskMap {
            file_path: String::from(file_path),
        })
    }

    fn read(&self) -> Result<ReadResult, Box<dyn error::Error>> {
        let fd = fcntl::open(
            self.file_path.deref(),
            OFlag::O_RDWR | OFlag::O_CREAT,
            Mode::S_IRUSR | Mode::S_IWUSR | Mode::S_IRGRP | Mode::S_IROTH,
        )?;

        let (new_fd, data) = DiskMap::read_lock(fd)?;

        let m = if data.is_empty() {
            HashMap::new()
        } else {
            let reader = flexbuffers::Reader::get_root(&data[..])?;
            HashMap::deserialize(reader)?
        };

        Ok(ReadResult { fd: new_fd, m })
    }

    fn read_lock(fd: os::fd::OwnedFd) -> Result<(os::fd::OwnedFd, Vec<u8>), Box<dyn error::Error>> {
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

    pub fn set(&self, k: &str, v: &str) -> Result<usize, Box<dyn error::Error>> {
        let mut read_result = self.read()?;

        // set new values
        read_result.m.insert(k.to_string(), v.to_string());

        // serialize hashmap
        let mut s = flexbuffers::FlexbufferSerializer::new();
        read_result.m.serialize(&mut s)?;

        // consume and replace fd
        let n = DiskMap::write_lock(read_result.fd, s)?;

        Ok(n)
    }

    pub fn get(&self, k: &str) -> Result<String, Box<dyn error::Error>> {
        let read_result = self.read()?;
        Ok(read_result.m.get(k).ok_or("not found")?.to_owned())
    }

    pub fn dump(&self) -> Result<HashMap<String, String>, Box<dyn error::Error>> {
        let read_result = self.read()?;
        Ok(read_result.m)
    }

    pub fn size(&self) -> Result<String, Box<dyn error::Error>> {
        let (r, w) = unistd::pipe()?;
        match unsafe { unistd::fork() } {
            Ok(unistd::ForkResult::Parent { child, .. }) => {
                // we don't need to write as a parent, just read
                close(w)?;
                // wait for our child to terminate
                nix::sys::wait::waitpid(child, None)?;
                // read from our child
                let mut buf = [0u8; 1024];
                let mut v: Vec<u8> = Vec::new();

                loop {
                    let n = unistd::read(&r, &mut buf)?;
                    if n == 0 {
                        break;
                    }

                    v.extend_from_slice(&buf[..n]);
                }

                Ok(String::from_utf8(v)?)
            }
            Ok(unistd::ForkResult::Child) => {
                // we don't need to read as a child, just write
                close(r)?;
                // make a copy our writing fd. it should be given the same fd as stdout. so when
                // someone writes to stdout, it will write to the same destination as `w`
                dup2_stdout(w)?;
                // execv
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
