use std::{error, io, mem, os, ptr};

use nix::libc;

mod handler;
mod net;
mod store;

fn get_sigfd(set: &libc::sigset_t) -> Result<os::raw::c_int, io::Error> {
    #[cfg(target_os = "linux")]
    {
        let sigfd = unsafe { libc::signalfd(-1, &set, 0) };
        if sigfd == -1 {
            return Err(io::Error::last_os_error());
        }
        Ok(sigfd)
    }

    #[cfg(not(target_os = "linux"))]
    Ok(34) // stub value for non-Linux OSes
}

fn main() -> Result<(), Box<dyn error::Error>> {
    // signal stuff
    unsafe {
        let mut set: libc::sigset_t = mem::zeroed();
        // define set of SIGUSR1 and mask it
        libc::sigemptyset(&mut set);
        libc::sigaddset(&mut set, libc::SIGUSR1);
        if libc::sigprocmask(libc::SIG_BLOCK, &set, ptr::null_mut()) == -1 {
            return Err(io::Error::last_os_error().into());
        }
        let sigfd = get_sigfd(&set)?;
        // define set of descriptors that we can listen to
        let mut readfs: libc::fd_set = mem::zeroed();
        libc::FD_ZERO(&mut readfs);
        libc::FD_SET(sigfd, &mut readfs);
    }

    // define deps
    let disk_map = store::DiskMap::new("/tmp/map")?;

    // define handlers
    let handler: Box<dyn net::types::Handler> = Box::new(handler::DiskHandler::new(disk_map));

    // start server
    let tcp_server = net::server::TCPServer::new(handler);
    tcp_server.start("8080")
}
