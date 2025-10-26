use nix::libc::{self, addrinfo};
use std::{collections, error, ffi, io, ptr};

use crate::net::types::Handler;

pub struct TCPServer {
    localhost: addrinfo,
    handlers: collections::HashMap<String, Box<dyn Handler>>,
}

impl TCPServer {
    pub fn new(
        port: &str,
        handlers: Vec<Box<dyn Handler>>,
    ) -> Result<TCPServer, Box<dyn error::Error>> {
        Ok(TCPServer {
            localhost: TCPServer::get_localhost(port)?,
            handlers: TCPServer::handlers_to_map(handlers),
        })
    }

    fn handlers_to_map(
        handlers: Vec<Box<dyn Handler>>,
    ) -> collections::HashMap<String, Box<dyn Handler>> {
        let mut hm = collections::HashMap::new();
        for handler in handlers {
            hm.insert(handler.action().to_string(), handler);
        }
        hm
    }

    pub fn start(&self) -> Result<(), Box<dyn error::Error>> {
        let sockfd = unsafe {
            libc::socket(
                self.localhost.ai_family,
                self.localhost.ai_socktype,
                self.localhost.ai_protocol,
            )
        };

        if unsafe { libc::bind(sockfd, self.localhost.ai_addr, self.localhost.ai_addrlen) } != 0 {
            return Err(io::Error::last_os_error().into());
        }

        if unsafe { libc::listen(sockfd, 128) } != 0 {
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
                    sockfd,
                    &mut address as *mut libc::sockaddr,
                    &mut address_len as *mut libc::socklen_t,
                )
            };
            if conn == -1 {
                return Err("connection was -1".into());
            }

            self.handle_connection(conn)?
        }
    }

    fn handle_connection(&self, conn: i32) -> Result<(), Box<dyn error::Error>> {
        loop {
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
            let mut out = self.dispatch_handler(bytes)?.to_owned();

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

    fn dispatch_handler(&self, bytes: &[u8]) -> Result<&[u8], Box<dyn error::Error>> {
        let parsed = std::str::from_utf8(bytes)?;
        let action = parsed.split_whitespace().next().ok_or("empty body")?;

        let handler = self
            .handlers
            .get(action)
            .ok_or_else(|| format!("unrecognized action: {}", action))?;

        Ok(handler.handle(bytes))
    }

    fn get_localhost(port: &str) -> Result<libc::addrinfo, Box<dyn error::Error>> {
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
        let status = unsafe {
            libc::getaddrinfo(
                host.as_ptr(),
                port.as_ptr(),
                &hints as *const libc::addrinfo,
                &mut result,
            )
        };
        if status != 0 {
            let err_msg_raw = unsafe { libc::gai_strerror(status) };
            let err_msg = unsafe { ffi::CStr::from_ptr(err_msg_raw) }.to_str()?;
            return Err(format!("error calling getaddrinfo: {}", err_msg).into());
        }

        Ok(unsafe { *result })
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
