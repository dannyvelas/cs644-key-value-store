use std::error;

mod handler;
mod net;
mod store;

fn main() -> Result<(), Box<dyn error::Error>> {
    // define deps
    let disk_map = store::DiskMap::new("/tmp/map")?;

    // define handlers
    let handler: Box<dyn net::types::Handler> = Box::new(handler::DiskHandler::new(disk_map));

    // start server
    let mut tcp_server = net::server::TCPServer::new(handler);
    tcp_server.start("8080")
}
