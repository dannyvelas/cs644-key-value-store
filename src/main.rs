use std::{error, ffi, io, mem, ptr};

use nix::libc;
mod handler;
mod net;
mod store;

static mut SELF_PIPE_WRITE: i32 = -1;

extern "C" fn handle_signal(signal_no: libc::c_int) {
    if unsafe { SELF_PIPE_WRITE } == -1 {
        return;
    }

    let buf: [char; 1] = [signal_no as u8 as char];
    let len = buf.len() as libc::size_t;
    unsafe { libc::write(SELF_PIPE_WRITE, buf.as_ptr() as *const ffi::c_void, len) };
}

fn main() -> Result<(), Box<dyn error::Error>> {
    // signal stuff
    let mut pipefd: [i32; 2] = [0; 2];
    unsafe {
        libc::pipe2(pipefd.as_mut_ptr(), libc::O_NONBLOCK);
        SELF_PIPE_WRITE = pipefd[1];

        // register handler
        let mut action = libc::sigaction {
            sa_sigaction: handle_signal as usize,
            sa_mask: mem::zeroed(),
            sa_flags: 0,
            sa_restorer: mem::zeroed(),
        };
        libc::sigemptyset(&mut action.sa_mask);
        if libc::sigaction(libc::SIGUSR1, &action, ptr::null_mut()) == -1 {
            return Err(io::Error::last_os_error().into());
        }
    }

    // define deps
    let disk_map = store::DiskMap::new("/tmp/map")?;

    // define handlers
    let handler = Box::new(handler::DiskHandler::new(disk_map));

    // start server
    let tcp_server = net::server::TCPServer::new(handler);
    tcp_server.start(pipefd[0], "8080")
}
