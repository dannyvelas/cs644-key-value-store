use nix::libc::{self, addrinfo};
use std::{error, ffi, io, ptr};

pub struct TCPServer {
    localhost: addrinfo,
}

impl TCPServer {
    pub fn new(port: &str) -> Result<TCPServer, Box<dyn error::Error>> {
        Ok(TCPServer {
            localhost: TCPServer::get_localhost(port)?,
        })
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
            return Err("error calling bind".into());
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

            if let Err(err) = TCPServer::handle_connection(conn) {
                return Err(err.into());
            }
        }
    }

    fn handle_connection(conn: i32) -> Result<(), io::Error> {
        loop {
            let mut buf = [0u8; 1024];
            let n = unsafe { libc::read(conn, buf.as_mut_ptr().cast(), buf.len() as libc::size_t) };
            if n == 0 {
                break;
            }
            if n == -1 {
                return TCPServer::close_fd(conn, Some(io::Error::last_os_error()));
            }
            println!("{:?}", &buf[..n as usize]);
        }
        TCPServer::close_fd(conn, None)
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
