use crate::{disk::map::DiskMap, net::types::Handler};
use std::error;

pub struct DiskHandler {
    disk_map: DiskMap,
    supported_commands: Vec<&'static str>,
}

impl DiskHandler {
    pub fn new(disk_map: DiskMap) -> DiskHandler {
        DiskHandler {
            disk_map,
            supported_commands: vec![
                "get <key>",
                "set <key> <value>",
                "delete <key>",
                "compact",
                "size",
                "dump",
            ],
        }
    }

    fn handle_result(&self, str: &str) -> Result<String, Box<dyn error::Error>> {
        let mut split = str.split_whitespace();
        match split.next().ok_or("empty body")? {
            "get" => {
                let key = split.next().ok_or("missing key argument")?;
                self.disk_map.get(key)
            }
            "set" => {
                let k = split.next().ok_or("missing key argument")?;
                let v = split.next().ok_or("missing value argument")?;
                let n = self.disk_map.set(k, v)?;
                Ok(format!("wrote {}={}. {} bytes", k, v, n))
            }
            "delete" => {
                let k = split.next().ok_or("missing key argument")?;
                self.disk_map.delete(k)?;
                Ok(format!("deleted {k}"))
            }
            "compact" => match self.disk_map.compact() {
                Err(err) => Err(err),
                Ok(n) => Ok(format!("compacted to {n} bytes")),
            },
            "size" => match self.disk_map.size() {
                Err(err) => Err(format!("error calling size: {}", err).into()),
                Ok(size) => Ok(size),
            },
            "dump" => {
                let m = self.disk_map.dump()?;
                Ok(format!("{:#?}", m))
            }
            _ => Ok("unrecognized".into()),
        }
    }
}

impl Handler for DiskHandler {
    fn handle(&self, s: &str) -> String {
        match self.handle_result(s) {
            Ok(out_string) => out_string,
            Err(err) => err.to_string().to_owned(),
        }
    }

    fn supported_commands(&self) -> &[&str] {
        &self.supported_commands
    }
}
