use std::error;

mod net;
mod store;

fn main() -> Result<(), Box<dyn error::Error>> {
    // define handlers
    let handler: Box<dyn net::types::Handler> = Box::new(net::handlers::GetHandler::new("hello"));

    // start server
    let tcp_server = net::server::TCPServer::new("8080", handler)?;
    tcp_server.start()
}
