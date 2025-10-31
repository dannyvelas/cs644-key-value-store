use nix::libc;
use std::{error, ffi, io, mem, ptr};

use crate::net::types::Handler;

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
        println!("AFTER LISTEN");

        // set up epoll
        let epoll_fd = unsafe { libc::epoll_create(1) };
        println!("AFTER CREATE EPOLL");
        TCPServer::setup_epoll(epoll_fd, signal_fd, sock_fd)?;
        println!("AFTER SETUP");

        const MAX_EVENTS: usize = 256;
        let mut events: [libc::epoll_event; MAX_EVENTS] = unsafe { mem::zeroed() };
        loop {
            let count =
                unsafe { libc::epoll_wait(epoll_fd, events.as_mut_ptr(), MAX_EVENTS as i32, -1) };
            if count == -1 {
                let last_err = io::Error::last_os_error();
                if last_err.raw_os_error() == Some(libc::EINTR) {
                    continue;
                }
                return Err(format!("got -1 from epoll_wait: {}", last_err).into());
            }
            for i in 0..count as usize {
                self.handle_event(events[i], signal_fd, sock_fd)?;
            }
        }
    }

    fn handle_event(
        &self,
        event: libc::epoll_event,
        signal_fd: i32,
        sock_fd: i32,
    ) -> Result<(), Box<dyn error::Error>> {
        let fd = event.u64 as i32;
        if fd == signal_fd {
            println!("accepting signal!");
            self.accept_signal(signal_fd)
        } else if fd == sock_fd {
            self.accept_conn(sock_fd)
        } else {
            Err(format!("received unexpected event of fd: {}", fd).into())
        }
    }

    fn accept_signal(&self, signal_fd: i32) -> Result<(), Box<dyn error::Error>> {
        let mut buf: [char; 1] = ['\0'; 1];
        let len = buf.len() as libc::size_t;
        match unsafe { libc::read(signal_fd, buf.as_mut_ptr().cast(), len) } {
            0 => Err("read 0 bytes from signal pipe...somehow".into()),
            -1 => Err(io::Error::last_os_error().into()),
            signal => {
                println!("received {}", signal);
                Ok(())
            }
        }
    }

    fn accept_conn(&self, sock_fd: i32) -> Result<(), Box<dyn error::Error>> {
        let conn = unsafe { libc::accept(sock_fd, ptr::null_mut(), ptr::null_mut()) };
        if conn == -1 {
            return Err("connection was -1".into());
        }

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

    extern "C" fn handle_c(arg: *mut ffi::c_void) -> *mut ffi::c_void {
        let arg = unsafe { Box::from_raw(arg as *mut ConnectionCtx) };
        if let Err(err) = arg.server.handle_connection(arg.conn) {
            eprintln!("internal server error encountered: {}", err)
        }
        ptr::null_mut()
    }

    fn handle_connection(&self, conn: i32) -> Result<(), Box<dyn error::Error>> {
        loop {
            // show prompt
            let mut p = String::from("\n~> ");
            if unsafe { libc::write(conn, p.as_mut_ptr().cast(), p.len() as libc::size_t) } == -1
                && let Err(err) = TCPServer::close_fd(conn, Some(io::Error::last_os_error()))
            {
                return Err(err.into());
            }

            // read input
            let mut buf = [0u8; 1024];
            let n = unsafe { libc::read(conn, buf.as_mut_ptr().cast(), buf.len() as libc::size_t) };
            if n == 0 {
                break;
            }
            if n == -1
                && let Err(err) = TCPServer::close_fd(conn, Some(io::Error::last_os_error()))
            {
                return Err(err.into());
            }
            let bytes = &buf[..n as usize];

            // process
            let mut out = self.handler.handle(bytes);

            // write output
            if unsafe { libc::write(conn, out.as_mut_ptr().cast(), out.len() as libc::size_t) }
                == -1
                && let Err(err) = TCPServer::close_fd(conn, Some(io::Error::last_os_error()))
            {
                return Err(err.into());
            }
        }
        TCPServer::close_fd(conn, None)?;
        Ok(())
    }

    fn close_fd(fd: i32, error: Option<io::Error>) -> Result<(), io::Error> {
        let close_status = unsafe { libc::close(fd) };
        match (close_status, error) {
            (-1, None) => Err(io::Error::last_os_error()),
            (-1, Some(err)) => {
                let merged = format!("error: {}, close: {}", err, io::Error::last_os_error());
                Err(io::Error::new(err.kind(), merged))
            }
            (_, Some(err)) => Err(err),
            (_, None) => Ok(()),
        }
    }
}
