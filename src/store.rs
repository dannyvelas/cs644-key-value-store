/*
 *  [08#9#asdfasdfasdfasdfg][08#9#asdfasdfasdfasdfg][08#9#asdfasdfasdfasdfg][08#9#asdfasdfasdfasdfg]
 */
extern crate flexbuffers;

use nix::fcntl::OFlag;
use nix::fcntl::{self, FlockArg};
use nix::sys::stat::Mode;
use nix::unistd::{self, close, dup2_stdout};
use std::collections::HashMap;
use std::ops::Deref;
use std::os::fd::AsFd;
use std::{error, ffi, mem, os, process};

struct ReadResult {
    offset: usize,
    fd: os::fd::OwnedFd,
    data: Vec<u8>,
}

impl Iterator for ReadResult {
    type Item = Entry;

    fn next(&mut self) -> Option<Self::Item> {
        // increment offset until this is no longer deleted
        while self.offset < self.data.len() {
            let start = self.offset;

            // get is_deleted byte, one byte long
            let is_deleted = self.data[self.offset] == 0;
            self.offset += 1;

            // get key size field, it is 4 bytes long and stored in big-endian
            // if number is 0xCAFEBABE, it is stored as CA FE BA BE
            let key_size_bytes = &self.data[self.offset..(self.offset + 4)];
            let key_size = ((key_size_bytes[0] << 24)
                | (key_size_bytes[1] << 16)
                | (key_size_bytes[2] << 8)
                | (key_size_bytes[3])) as usize;
            self.offset += 4;

            // get value size field, also 4 bytes long and stored in big-endian
            let value_size_bytes = &self.data[self.offset..(self.offset + 4)];
            let value_size = ((value_size_bytes[0] << 24)
                | (value_size_bytes[1] << 16)
                | (value_size_bytes[2] << 8)
                | (value_size_bytes[3])) as usize;
            self.offset += 4;

            let key = str::from_utf8(&self.data[self.offset..(self.offset + key_size)]).ok()?;
            self.offset += key_size;
            let value = str::from_utf8(&self.data[self.offset..(self.offset + value_size)]).ok()?;
            self.offset += value_size;

            if !is_deleted {
                return Some(Entry {
                    offset: start,
                    key: key.to_owned(),
                    value: value.to_owned(),
                });
            }
        }
        return None;
    }
}

struct Entry {
    offset: usize,
    key: String,
    value: String,
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

    fn find_key(&self, data: &[u8]) -> Option<Entry> {
        Some(Entry {
            offset: 1,
            key: String::from("hi"),
            value: String::from("bye"),
        })
    }

    fn read_lock(&self) -> Result<ReadResult, Box<dyn error::Error>> {
        let fd = fcntl::open(
            self.file_path.deref(),
            OFlag::O_RDWR | OFlag::O_CREAT,
            Mode::S_IRUSR | Mode::S_IWUSR | Mode::S_IRGRP | Mode::S_IROTH,
        )?;
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

        Ok(ReadResult {
            offset: 0,
            fd: new_fd,
            data: v,
        })
    }

    fn append_key(fd: os::fd::OwnedFd, k: &str, v: &str) -> Result<usize, Box<dyn error::Error>> {
        //// acquire exclusive lock
        //let lock = fcntl::Flock::lock(fd, FlockArg::LockExclusive).map_err(|(_, e)| e)?;

        //thread::sleep(time::Duration::from_secs(11));

        //// self.truncate file
        //unistd::ftruncate(lock.as_fd(), 0)?;

        //// write
        //let n = unistd::write(lock.as_fd(), s.view())?;

        //// release lock
        //let _ = lock.unlock().map_err(|(_, e)| e)?;

        Ok(123)
    }
    fn delete_entry(
        fd: os::fd::OwnedFd,
        entry: Entry,
    ) -> Result<os::fd::OwnedFd, Box<dyn error::Error>> {
        //// acquire exclusive lock
        //let lock = fcntl::Flock::lock(fd, FlockArg::LockExclusive).map_err(|(_, e)| e)?;

        //thread::sleep(time::Duration::from_secs(11));

        //// self.truncate file
        //unistd::ftruncate(lock.as_fd(), 0)?;

        //// write
        //let n = unistd::write(lock.as_fd(), s.view())?;

        //// release lock
        //let _ = lock.unlock().map_err(|(_, e)| e)?;

        let fd: os::fd::OwnedFd = unsafe { mem::zeroed() };

        Ok(fd)
    }

    pub fn set(&self, k: &str, v: &str) -> Result<usize, Box<dyn error::Error>> {
        let read_result = self.read_lock()?;

        let fd = if let Some(entry) = self.find_key(&read_result.data) {
            DiskMap::delete_entry(read_result.fd, entry)?
        } else {
            read_result.fd
        };

        DiskMap::append_key(fd, k, v)
    }

    pub fn get(&self, k: &str) -> Result<String, Box<dyn error::Error>> {
        let read_result = self.read_lock()?;
        Ok(self
            .find_key(&read_result.data)
            .ok_or(format!("{k} not found"))?
            .value)
    }

    pub fn dump(&self) -> Result<HashMap<String, String>, Box<dyn error::Error>> {
        let read_result = self.read_lock()?;
        let mut m = HashMap::<String, String>::new();
        for entry in read_result {
            m.insert(entry.key, entry.value);
        }
        Ok(m)
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
                Ok(String::from_utf8(v)?.trim().to_string())
            }
            Ok(unistd::ForkResult::Child) => {
                // we don't need to read as a child, just write
                close(r)?;
                // make a copy our writing fd. it should be given the same fd as stdout. so when
                // someone writes to stdout, it will write to the same destination as `w`
                dup2_stdout(w)?;
                // execv
                let path = ffi::CString::new("/usr/bin/wc")?;
                let arg1 = ffi::CString::new("-c")?;
                let arg2 = ffi::CString::new(self.file_path.clone())?;
                let argv = [&path, &arg1, &arg2];
                let Err(err) = nix::unistd::execv(&path, &argv);
                // if we're here, it means execv failed. if it succeeded then the code from this process would
                // have been replaced by now
                eprintln!("execv failed: {}", err);
                process::exit(1);
            }
            Err(err_no) => Err(err_no.into()),
        }
    }
}
