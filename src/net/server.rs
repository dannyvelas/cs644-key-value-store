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

    pub fn start(&self, port: &str) -> Result<(), Box<dyn error::Error>> {
        let sock_fd = TCPServer::local_sockfd(port)?;

        if unsafe { libc::listen(sock_fd, 128) } != 0 {
            return Err("error calling listen".into());
        }

        let mut address = libc::sockaddr {
            sa_len: 0,
            sa_family: 0,
            sa_data: [0; 14],
        };
        let mut address_len: libc::socklen_t = 0;
        loop {
            let conn = unsafe {
                libc::accept(
                    sock_fd,
                    &mut address as *mut libc::sockaddr,
                    &mut address_len as *mut libc::socklen_t,
                )
            };
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
                0 => continue,
                libc::EAGAIN => return Err("insufficient resources".into()),
                libc::EINVAL => return Err("invalid settings in attr".into()),
                libc::EPERM => return Err("insufficient permissions".into()),
                ret => return Err(format!("unexpected return value: {}", ret).into()),
            }
        }
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
