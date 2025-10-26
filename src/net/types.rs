pub trait Handler {
    fn handle(&self, s: &[u8]) -> Vec<u8>;
}
