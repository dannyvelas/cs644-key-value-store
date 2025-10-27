use crate::{net::types::Handler, store::DiskMap};
use std::error;

pub struct DiskHandler {
    disk_map: DiskMap,
}

impl DiskHandler {
    pub fn new(disk_map: DiskMap) -> DiskHandler {
        DiskHandler { disk_map }
    }

    fn handle_result(&mut self, bytes: &[u8]) -> Result<String, Box<dyn error::Error>> {
        let mut split = str::from_utf8(bytes)?.split_whitespace();
        match split.next().ok_or("empty body")? {
            "get" => {
                let key = split.next().ok_or("no key argument to get")?;
                self.disk_map.get(key)
            }
            "set" => {
                let k = split.next().ok_or("no key argument to set")?;
                let v = split.next().ok_or("no value argument to set")?;
                let n = self.disk_map.set(k, v)?;
                Ok(format!("wrote {}={}. {} bytes", k, v, n))
            }
            "size" => match self.disk_map.size() {
                Err(err) => Err(format!("error calling size: {}", err).into()),
                Ok(_) => Ok("got size".into()),
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
    fn handle(&mut self, bytes: &[u8]) -> Vec<u8> {
        match self.handle_result(bytes) {
            Ok(out_bytes) => (out_bytes + "\n").into(),
            Err(err) => format!("error encountered: {}\n", err).into(),
        }
    }
}
