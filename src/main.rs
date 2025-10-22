/*
* What is the difference between the type <code>&amp;String</code> and <code>*String</code>
* */
mod store;

use nix::libc;
use std::{
    error,
    ffi::{self, CString},
    io, ptr,
};

fn get_localhost() -> Result<libc::addrinfo, Box<dyn error::Error>> {
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
    let port = ffi::CString::new("8080")?;
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
    let a = "asdfs";

    Ok(unsafe { *result })
}

fn close_fd(fd: i32) {
    let close_status = unsafe { libc::close(fd) };
    if close_status == -1 {
        eprintln!("{}", io::Error::last_os_error());
    }
}

fn handle_connection(conn: i32) {
    loop {
        let mut buf = [0u8; 1024];
        let n = unsafe { libc::read(conn, buf.as_mut_ptr().cast(), buf.len() as libc::size_t) };
        if n == 0 {
            break;
        }
        if n == -1 {
            eprintln!("{}", io::Error::last_os_error());
            close_fd(conn);
            return;
        }
        println!("{:?}", &buf[..n as usize]);
    }
    close_fd(conn);
}

fn main() -> Result<(), Box<dyn error::Error>> {
    //let disk_map = store::DiskMap::new("/tmp/map").unwrap();
    let localhost = get_localhost()?;
    let sockfd = unsafe {
        libc::socket(
            localhost.ai_family,
            localhost.ai_socktype,
            localhost.ai_protocol,
        )
    };

    if unsafe { libc::bind(sockfd, localhost.ai_addr, localhost.ai_addrlen) } != 0 {
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
            println!("connection was -1");
            break;
        }
        println!("got connection: {}", conn);

        handle_connection(conn)
    }

    Ok(())
}
