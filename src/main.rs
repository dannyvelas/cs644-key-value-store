use std::error;

mod net;
mod store;

fn main() -> Result<(), Box<dyn error::Error>> {
    // define handlers
    let handlers: Vec<Box<dyn net::types::Handler>> =
        vec![Box::new(net::handlers::GetHandler::new("hello"))];

    // start server
    let tcp_server = net::server::TCPServer::new("8080", handlers)?;
    tcp_server.start()
}
