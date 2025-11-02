pub trait Handler {
    fn handle(&self, s: &str) -> String;
}
