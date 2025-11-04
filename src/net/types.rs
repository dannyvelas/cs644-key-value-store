pub trait Handler {
    fn handle(&self, s: &str) -> String;
    fn supported_commands(&self) -> &[&str];
}
