use std::error;

mod net;
mod store;

fn main() -> Result<(), Box<dyn error::Error>> {
    let tcp_server = net::TCPServer::new()?;
    tcp_server.start()
}
