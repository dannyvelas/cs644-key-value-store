use crate::net::types::Handler;

pub struct GetHandler {
    temp: String,
}

impl GetHandler {
    pub fn new(s: &str) -> GetHandler {
        GetHandler {
            temp: String::from(s),
        }
    }
}

impl Handler for GetHandler {
    fn handle(&self, _in_bytes: &[u8]) -> &[u8] {
        self.temp.as_bytes()
    }
}
