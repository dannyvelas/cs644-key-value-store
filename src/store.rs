extern crate flexbuffers;

use nix::{fcntl, fcntl::OFlag, libc, sys, sys::stat::Mode, unistd};
use std::collections::HashMap;
use std::ops::Deref;
use std::os::fd::{AsFd, AsRawFd};
use std::{error, ffi, io, os, process, thread, time};

struct ReadResult {
    offset: usize,
    data: Vec<u8>,
}

impl Iterator for ReadResult {
    type Item = Entry;

    fn next(&mut self) -> Option<Self::Item> {
        println!("top. offset={}. len={}", self.offset, self.data.len());
        // increment offset until this is no longer deleted
        while self.offset < self.data.len() {
            println!("offset={}. len={}", self.offset, self.data.len());
            let start = self.offset;

            // get is_deleted byte, one byte long
            let is_deleted = self.data[self.offset] == 0;
            self.offset += 1;
            println!(
                "offset={}. byte={}",
                self.offset,
                self.data[self.offset - 1]
            );

            // get key size field, it is 4 bytes long and stored in big-endian
            // if number is 0xCAFEBABE, it is stored as CA FE BA BE
            let key_size_bytes = &self.data[self.offset..(self.offset + 4)];
            let key_size = (((key_size_bytes[0] as i32) << 24)
                | ((key_size_bytes[1] as i32) << 16)
                | ((key_size_bytes[2] as i32) << 8)
                | (key_size_bytes[3] as i32)) as usize;
            println!("key_size={}", key_size);
            self.offset += 4;

            // get value size field, also 4 bytes long and stored in big-endian
            let value_size_bytes = &self.data[self.offset..(self.offset + 4)];
            let value_size = (((value_size_bytes[0] as i32) << 24)
                | ((value_size_bytes[1] as i32) << 16)
                | ((value_size_bytes[2] as i32) << 8)
                | (value_size_bytes[3] as i32)) as usize;
            println!("value_size={}", value_size);
            self.offset += 4;

            let key = str::from_utf8(&self.data[self.offset..(self.offset + key_size)]).ok()?;
            println!("key={}", key);
            self.offset += key_size;
            let value = str::from_utf8(&self.data[self.offset..(self.offset + value_size)]).ok()?;
            println!("value={}", value);
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

    fn slurp(fd: os::fd::BorrowedFd) -> Result<ReadResult, Box<dyn error::Error>> {
        let mut buf = [0u8; 1024];
        let mut v: Vec<u8> = Vec::new();
        loop {
            let n = unistd::read(fd, &mut buf)?;
            if n == 0 {
                break;
            }

            v.extend_from_slice(&buf[..n]);
        }
        Ok(ReadResult { offset: 0, data: v })
    }

    fn read(&self) -> Result<ReadResult, Box<dyn error::Error>> {
        let fd = fcntl::open(
            self.file_path.deref(),
            OFlag::O_RDWR | OFlag::O_CREAT,
            Mode::S_IRUSR | Mode::S_IWUSR | Mode::S_IRGRP | Mode::S_IROTH,
        )?;

        let lock = fcntl::Flock::lock(fd, fcntl::FlockArg::LockShared).map_err(|(_, e)| e)?;
        let read_result = DiskMap::slurp(lock.as_fd())?;
        let _ = lock.unlock().map_err(|(_, e)| e)?;

        Ok(read_result)
    }

    fn append_key(
        fd: os::fd::BorrowedFd,
        k: &str,
        v: &str,
    ) -> Result<isize, Box<dyn error::Error>> {
        // seek to end
        if unsafe { libc::lseek(fd.as_raw_fd(), 0, libc::SEEK_END) } == -1 {
            return Err(io::Error::last_os_error().into());
        }

        // create Entry  [0u8] + [k.len() as u32]  + [v.len() as u32] + [k] + [v]
        let klen: u32 = k.len().try_into()?;
        let key_size_bytes: [u8; 4] = [
            ((klen >> 24) as u8),
            ((klen >> 16) as u8),
            ((klen >> 8) as u8),
            (klen as u8),
        ];
        let vlen: u32 = v.len().try_into()?;
        let value_size_bytes: [u8; 4] = [
            ((vlen >> 24) as u8),
            ((vlen >> 16) as u8),
            ((vlen >> 8) as u8),
            (vlen as u8),
        ];
        let mut buf = Vec::<u8>::with_capacity(
            1 + key_size_bytes.len() + value_size_bytes.len() + k.len() + v.len(),
        );
        buf.extend_from_slice(&[1u8; 1]);
        buf.extend_from_slice(&key_size_bytes);
        buf.extend_from_slice(&value_size_bytes);
        buf.extend_from_slice(k.as_bytes());
        buf.extend_from_slice(v.as_bytes());

        println!("buf={:?}", buf);

        let n = unsafe { libc::write(fd.as_raw_fd(), buf.as_ptr().cast(), buf.len()) };
        if n == -1 {
            return Err(io::Error::last_os_error().into());
        }

        Ok(n)
    }

    fn delete_entry(fd: os::fd::BorrowedFd, entry: Entry) -> Result<(), Box<dyn error::Error>> {
        // seek to offset
        if unsafe { libc::lseek(fd.as_raw_fd(), entry.offset as i64, libc::SEEK_SET) } == -1 {
            return Err(io::Error::last_os_error().into());
        }

        // overwrite byte to be 0 instead of 1
        let del = &[0u8; 1];
        let len = del.len() as libc::size_t;
        if unsafe { libc::write(fd.as_raw_fd(), del.as_ptr().cast(), len) } == -1 {
            return Err(io::Error::last_os_error().into());
        }

        Ok(())
    }

    pub fn set(&self, k: &str, v: &str) -> Result<isize, Box<dyn error::Error>> {
        // open file
        let fd = fcntl::open(
            self.file_path.deref(),
            OFlag::O_RDWR | OFlag::O_CREAT,
            Mode::S_IRUSR | Mode::S_IWUSR | Mode::S_IRGRP | Mode::S_IROTH,
        )?;

        // acquire exclusive lock
        let lock = fcntl::Flock::lock(fd, fcntl::FlockArg::LockExclusive).map_err(|(_, e)| e)?;

        thread::sleep(time::Duration::from_secs(10));

        // read into variable
        let mut read_result = DiskMap::slurp(lock.as_fd())?;

        // if key exists, delete it
        if let Some(entry) = read_result.find(|x| x.key == k) {
            DiskMap::delete_entry(lock.as_fd(), entry)?;
        }

        // append key
        let size = DiskMap::append_key(lock.as_fd(), k, v)?;

        // release lock
        let _ = lock.unlock().map_err(|(_, e)| e)?;

        Ok(size)
    }

    pub fn get(&self, k: &str) -> Result<String, Box<dyn error::Error>> {
        Ok(self
            .read()?
            .find(|x| x.key == k)
            .ok_or(format!("{k} not found"))?
            .value)
    }

    pub fn dump(&self) -> Result<HashMap<String, String>, Box<dyn error::Error>> {
        let read_result = self.read()?;

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
                unistd::close(w)?;
                // wait for our child to terminate
                sys::wait::waitpid(child, None)?;
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
                unistd::close(r)?;
                // make a copy our writing fd. it should be given the same fd as stdout. so when
                // someone writes to stdout, it will write to the same destination as `w`
                unistd::dup2_stdout(w)?;
                // execv
                let path = ffi::CString::new("/usr/bin/wc")?;
                let arg1 = ffi::CString::new("-c")?;
                let arg2 = ffi::CString::new(self.file_path.clone())?;
                let argv = [&path, &arg1, &arg2];
                let Err(err) = unistd::execv(&path, &argv);
                // if we're here, it means execv failed. if it succeeded then the code from this process would
                // have been replaced by now
                eprintln!("execv failed: {}", err);
                process::exit(1);
            }
            Err(err_no) => Err(err_no.into()),
        }
    }
}
