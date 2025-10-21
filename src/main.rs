mod store;

use nix::libc::{
    AF_INET, SOCK_STREAM, accept, addrinfo, bind, close, getaddrinfo, listen, read, size_t,
    sockaddr, socket, socklen_t,
};

fn get_localhost() -> Result<addrinfo, Box<dyn std::error::Error>> {
    let hints = addrinfo {
        ai_flags: 0,
        ai_family: AF_INET,
        ai_socktype: SOCK_STREAM,
        ai_protocol: 0,
        ai_addrlen: 0,
        ai_canonname: std::ptr::null_mut(),
        ai_addr: std::ptr::null_mut(),
        ai_next: std::ptr::null_mut(),
    };
    let mut result: *mut addrinfo = std::ptr::null_mut();
    let host = std::ffi::CString::new("localhost")?;
    let port = std::ffi::CString::new("8080")?;
    let status = unsafe {
        getaddrinfo(
            host.as_ptr(),
            port.as_ptr(),
            &hints as *const addrinfo,
            &mut result,
        )
    };
    if status != 0 {
        return Err("status was 0".into());
    }
    println!("get status: {}", status);

    Ok(unsafe { *result })
}

fn handle_connection(conn: i32) {
    loop {
        let mut buf = [0u8; 1024];
        let n = unsafe { read(conn, buf.as_mut_ptr().cast(), buf.len() as size_t) };
        if n == 0 {
            break;
        }
        if n == -1 {
            println!("read returned -1");
            break;
        }
        println!("{:?}", &buf[..n as usize]);
    }
    unsafe { close(conn) };
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    //let disk_map = store::DiskMap::new("/tmp/map").unwrap();
    let localhost = get_localhost()?;
    println!("got localhost: {:?}", localhost);
    let sockfd = unsafe {
        socket(
            localhost.ai_family,
            localhost.ai_socktype,
            localhost.ai_protocol,
        )
    };

    if unsafe { bind(sockfd, localhost.ai_addr, localhost.ai_addrlen) } != 0 {
        return Err("error calling bind".into());
    }

    if unsafe { listen(sockfd, 128) } != 0 {
        return Err("error calling listen".into());
    }

    let mut address = sockaddr {
        sa_len: 0,
        sa_family: 0,
        sa_data: [0; 14],
    };
    let mut address_len: socklen_t = 0;
    loop {
        let conn = unsafe {
            accept(
                sockfd,
                &mut address as *mut sockaddr,
                &mut address_len as *mut socklen_t,
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
