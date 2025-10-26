pub trait Handler {
    fn action(&self) -> String;
    fn handle(&self, s: &[u8]) -> &[u8];
}
