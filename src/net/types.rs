pub trait Handler {
    fn handle(&mut self, s: &[u8]) -> Vec<u8>;
}
