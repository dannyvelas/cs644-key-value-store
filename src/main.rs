use std::{error, io, mem, ptr};

use nix::{libc, unistd};
mod disk;
mod handler;
mod net;

static mut SELF_PIPE_WRITE: i32 = -1;

extern "C" fn handle_signal(signal_no: libc::c_int) {
    if unsafe { SELF_PIPE_WRITE } == -1 {
        return;
    }

    let buf: [u8; 1] = [signal_no as u8];
    unsafe { libc::write(SELF_PIPE_WRITE, buf.as_ptr().cast(), buf.len()) };
}

fn main() -> Result<(), Box<dyn error::Error>> {
    // init signal pipe
    let pipe_fd = init_signal_pipe()?;

    // process id
    let pid = unistd::getpid();

    // define deps
    let disk_map = disk::map::DiskMap::new("/tmp/map")?;

    // define handlers
    let handler = Box::new(handler::DiskHandler::new(disk_map));

    // start server
    let tcp_server = net::server::TCPServer::new(pid, handler);
    let result = tcp_server.start(pipe_fd[0], "8080");

    // close self-write fds
    unsafe { libc::close(pipe_fd[0]) };
    unsafe { libc::close(pipe_fd[1]) };

    result
}

fn init_signal_pipe() -> Result<[i32; 2], io::Error> {
    let mut pipe_fd = [0i32; 2];
    unsafe {
        libc::pipe2(pipe_fd.as_mut_ptr(), libc::O_NONBLOCK);
        SELF_PIPE_WRITE = pipe_fd[1];

        // register handler
        let mut action = libc::sigaction {
            sa_sigaction: handle_signal as usize,
            sa_mask: mem::zeroed(),
            sa_flags: 0,
            sa_restorer: mem::zeroed(),
        };
        libc::sigemptyset(&mut action.sa_mask);
        if libc::sigaction(libc::SIGUSR1, &action, ptr::null_mut()) == -1 {
            return Err(io::Error::last_os_error());
        }

        if libc::sigaction(libc::SIGINT, &action, ptr::null_mut()) == -1 {
            return Err(io::Error::last_os_error());
        }
    }
    Ok(pipe_fd)
}
