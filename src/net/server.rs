use nix::libc;
use std::{error, ffi, io, mem, ptr};

use crate::net::types::Handler;

enum Error {
    RetryableErr(i32),
    UnexpectedErr(String),
}

enum ReadError {
    RetryableErr(i32),
    UnexpectedErr(String),
    Closed,
}

pub struct ConnectionCtx<'a> {
    server: &'a TCPServer,
    conn: i32,
}

pub struct TCPServer {
    handler: Box<dyn Handler>,
}

impl TCPServer {
    pub fn new(handler: Box<dyn Handler>) -> TCPServer {
        TCPServer { handler }
    }

    pub fn start(&self, signal_fd: i32, port: &str) -> Result<(), Box<dyn error::Error>> {
        let sock_fd = TCPServer::local_sockfd(port)?;

        if unsafe { libc::listen(sock_fd, 128) } != 0 {
            return Err("error calling listen".into());
        }

        // set up epoll
        let epoll_fd = unsafe { libc::epoll_create(1) };
        TCPServer::setup_epoll(epoll_fd, signal_fd, sock_fd)?;

        const MAX_EVENTS: i32 = 256;
        let mut events: [libc::epoll_event; MAX_EVENTS as usize] = unsafe { mem::zeroed() };
        loop {
            match self.handle_events(epoll_fd, &mut events, MAX_EVENTS, signal_fd, sock_fd) {
                Ok(()) | Err(Error::RetryableErr(_)) => continue,
                Err(Error::UnexpectedErr(err)) => {
                    eprintln!("{err}");
                    break;
                }
            }
        }
        // TODO: should i close epoll_fd as well?
        // TODO: should i close signal_fd as well?
        eprintln!("closing socket");
        unsafe { libc::close(sock_fd) };
        Ok(())
    }

    fn handle_events(
        &self,
        epoll_fd: i32,
        events: &mut [libc::epoll_event],
        max_events: i32,
        signal_fd: i32,
        sock_fd: i32,
    ) -> Result<(), Error> {
        let count = unsafe { libc::epoll_wait(epoll_fd, events.as_mut_ptr(), max_events, -1) };
        if count == -1 {
            let last_err = io::Error::last_os_error();
            if last_err.raw_os_error() == Some(libc::EINTR) {
                return Err(Error::RetryableErr(libc::EINTR));
            }
            return Err(Error::UnexpectedErr(
                format!("got -1 from epoll_wait: {}", last_err).into(),
            ));
        }

        for i in 0..count as usize {
            if let Err(err) = self.handle_event(events[i], signal_fd, sock_fd) {
                return Err(Error::UnexpectedErr(err.to_string()));
            }
        }
        Ok(())
    }

    fn handle_event(
        &self,
        event: libc::epoll_event,
        signal_fd: i32,
        sock_fd: i32,
    ) -> Result<(), Box<dyn error::Error>> {
        match event.u64 as i32 {
            fd if fd == signal_fd => self.accept_signal(signal_fd),
            fd if fd == sock_fd => self.accept_conn(sock_fd),
            fd => Err(format!("received unexpected event of fd: {}", fd).into()),
        }
    }

    fn accept_signal(&self, signal_fd: i32) -> Result<(), Box<dyn error::Error>> {
        let mut buf = [0u8; 1];
        let len = buf.len() as libc::size_t;
        let read = unsafe { libc::read(signal_fd, buf.as_mut_ptr().cast(), len) };
        if read == -1 {
            return Err(io::Error::last_os_error().into());
        } else if read == 0 {
            return Err("read 0 bytes from signal pipe...somehow".into());
        };

        let signal = buf[0] as i32;
        if signal == libc::SIGINT {
            return Err("received SIGINT".into());
        } else if signal == libc::SIGUSR1 {
            self.handler.handle("compact");
        }

        return Ok(());
    }

    fn accept_conn(&self, sock_fd: i32) -> Result<(), Box<dyn error::Error>> {
        let conn = match TCPServer::safe_accept(sock_fd) {
            Err(Error::UnexpectedErr(err)) => return Err(err.into()),
            Err(Error::RetryableErr(_)) => return Ok(()),
            Ok(conn) => conn,
        };

        // spawn thread
        match unsafe {
            let mut native: libc::pthread_t = mem::zeroed();
            let boxed_args = Box::new(ConnectionCtx { server: self, conn });
            let arg_ptr = Box::into_raw(boxed_args) as *mut ffi::c_void;
            libc::pthread_create(&mut native, ptr::null(), TCPServer::handle_c, arg_ptr)
        } {
            0 => Ok(()),
            libc::EAGAIN => Err("insufficient resources".into()),
            libc::EINVAL => Err("invalid settings in attr".into()),
            libc::EPERM => Err("insufficient permissions".into()),
            ret => Err(format!("unexpected return value: {}", ret).into()),
        }
    }

    extern "C" fn handle_c(arg: *mut ffi::c_void) -> *mut ffi::c_void {
        let arg = unsafe { Box::from_raw(arg as *mut ConnectionCtx) };
        if let Err(err) = arg.server.handle_connection(arg.conn) {
            eprintln!("internal server error. connection closed: {}", err)
        }
        ptr::null_mut()
    }

    fn handle_connection(&self, conn: i32) -> Result<(), Box<dyn error::Error>> {
        if let ReadError::Closed = self.repl(conn) {
            unsafe {
                let msg = "client closed connection. closing on server side.\n";
                libc::write(conn, msg.as_ptr().cast(), msg.len() as libc::size_t);
            }
        }
        unsafe { libc::close(conn) };
        Ok(())
    }

    fn repl(&self, conn: i32) -> ReadError {
        loop {
            // show prompt
            match TCPServer::safe_write(conn, "~> ") {
                Err(Error::RetryableErr(_)) => continue,
                Err(Error::UnexpectedErr(err)) => return ReadError::UnexpectedErr(err),
                _ => {}
            }

            // read input
            let input = match TCPServer::safe_read(conn) {
                Ok(ref s) if s == "" => continue,
                Ok(ref s) if s == "quit" || s == "exit" => return ReadError::Closed,
                Ok(s) => s,
                Err(ReadError::RetryableErr(_)) => continue,
                Err(ReadError::UnexpectedErr(err)) => return ReadError::UnexpectedErr(err),
                Err(ReadError::Closed) => return ReadError::Closed,
            };

            // process
            let out = self.handler.handle(&input);

            // write output
            match TCPServer::safe_write(conn, &out) {
                Err(Error::RetryableErr(_)) => continue,
                Err(Error::UnexpectedErr(err)) => return ReadError::UnexpectedErr(err),
                _ => continue,
            }
        }
    }

    fn safe_accept(sock_fd: i32) -> Result<i32, Error> {
        let conn = unsafe { libc::accept(sock_fd, ptr::null_mut(), ptr::null_mut()) };
        if conn == -1 {
            let err = io::Error::last_os_error();
            return match err.raw_os_error() {
                Some(x) if x == libc::EAGAIN || x == libc::EINTR => Err(Error::RetryableErr(x)),
                _ => Err(Error::UnexpectedErr(err.to_string())),
            };
        }
        Ok(conn)
    }

    fn safe_read(conn: i32) -> Result<String, ReadError> {
        let mut buf = [0u8; 1024];
        let n = unsafe { libc::read(conn, buf.as_mut_ptr().cast(), buf.len() as libc::size_t) };
        if n == -1 {
            let err = io::Error::last_os_error();
            match err.raw_os_error() {
                Some(x) if x == libc::EAGAIN || x == libc::EINTR => Err(ReadError::RetryableErr(x)),
                _ => Err(ReadError::UnexpectedErr(err.to_string())),
            }
        } else if n == 0 {
            Err(ReadError::Closed)
        } else {
            let bytes = &buf[..n as usize];
            match str::from_utf8(bytes) {
                Err(err) => Err(ReadError::UnexpectedErr(err.to_string())),
                Ok(s) => Ok(s.trim().to_owned()),
            }
        }
    }

    fn safe_write(conn: i32, s: &str) -> Result<(), Error> {
        if unsafe { libc::write(conn, s.as_ptr().cast(), s.len() as libc::size_t) } == -1 {
            let err = io::Error::last_os_error();
            return match err.raw_os_error() {
                Some(x) if x == libc::EAGAIN || x == libc::EINTR => Err(Error::RetryableErr(x)),
                _ => Err(Error::UnexpectedErr(err.to_string())),
            };
        }
        Ok(())
    }

    fn local_sockfd(port: &str) -> Result<i32, Box<dyn error::Error>> {
        let hints = libc::addrinfo {
            ai_flags: 0,
            ai_family: libc::AF_INET,
            ai_socktype: libc::SOCK_STREAM,
            ai_protocol: 0,
            ai_addrlen: 0,
            ai_canonname: ptr::null_mut(),
            ai_addr: ptr::null_mut(),
            ai_next: ptr::null_mut(),
        };
        let mut result = ptr::null_mut();
        let host = ffi::CString::new("localhost")?;
        let port = ffi::CString::new(port)?;
        unsafe {
            let status = libc::getaddrinfo(
                host.as_ptr(),
                port.as_ptr(),
                &hints as *const libc::addrinfo,
                &mut result,
            );
            if status != 0 {
                let err_msg_raw = libc::gai_strerror(status);
                let err_msg = ffi::CStr::from_ptr(err_msg_raw).to_str()?;
                return Err(format!("error calling getaddrinfo: {}", err_msg).into());
            }

            let mut result_ptr = result;
            let mut sock_fd = 0;
            while !result_ptr.is_null() {
                sock_fd = libc::socket(
                    (*result_ptr).ai_family,
                    (*result_ptr).ai_socktype,
                    (*result_ptr).ai_protocol,
                );
                if sock_fd == -1 {
                    result_ptr = (*result_ptr).ai_next;
                    continue;
                }

                if libc::bind(sock_fd, (*result_ptr).ai_addr, (*result_ptr).ai_addrlen) != 0 {
                    libc::close(sock_fd);
                    result_ptr = (*result_ptr).ai_next;
                    continue;
                }

                break;
            }
            libc::freeaddrinfo(result);

            if result_ptr.is_null() {
                return Err("could not bind".into());
            }

            Ok(sock_fd)
        }
    }

    fn setup_epoll(
        epoll_fd: i32,
        signal_fd: i32,
        sock_fd: i32,
    ) -> Result<(), Box<dyn error::Error>> {
        unsafe {
            let mut signal_ev = libc::epoll_event {
                events: libc::EPOLLIN as u32,
                u64: signal_fd as u64,
            };
            if libc::epoll_ctl(epoll_fd, libc::EPOLL_CTL_ADD, signal_fd, &mut signal_ev) == -1 {
                return Err(format!(
                    "error adding signal to epoll: {}",
                    io::Error::last_os_error()
                )
                .into());
            }

            let mut sock_ev = libc::epoll_event {
                events: libc::EPOLLIN as u32,
                u64: sock_fd as u64,
            };
            if libc::epoll_ctl(epoll_fd, libc::EPOLL_CTL_ADD, sock_fd, &mut sock_ev) == -1 {
                return Err(format!(
                    "error adding socket to epoll: {}",
                    io::Error::last_os_error()
                )
                .into());
            }
        }
        Ok(())
    }
}
