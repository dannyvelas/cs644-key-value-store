use crate::{net::types::Handler, store::DiskMap};
use std::error;

pub struct DiskHandler {
    disk_map: DiskMap,
}

impl DiskHandler {
    pub fn new(disk_map: DiskMap) -> DiskHandler {
        DiskHandler { disk_map }
    }

    fn handle_result(&self, bytes: &[u8]) -> Result<&[u8], Box<dyn error::Error>> {
        let action = str::from_utf8(bytes)?
            .split_whitespace()
            .next()
            .ok_or("empty body")?;
        match action {
            "get" => {
                Ok("get".as_bytes())
                // let key = sub_matches.get_one::<String>("key").expect("required");
                // if let Some(value) = disk_map.get(key) {
                //     println!("{}", value);
                // } else {
                //     println!("No value found for {}", key);
                // }
            }
            "set" => {
                Ok("set".as_bytes())
                // let k = sub_matches
                //     .get_one::<String>("key")
                //     .expect("required")
                //     .as_str();
                // let v = sub_matches
                //     .get_one::<String>("value")
                //     .expect("required")
                //     .as_str();
                // disk_map.set(k, v).unwrap();
            }
            "size" => {
                Ok("size".as_bytes())
                // if let Err(e) = disk_map.size() {
                //     println!("error calling size: {}", e)
                // }
            }
            "dump" => {
                Ok("dump".as_bytes())
                // println!("{:#?}", disk_map.m);
            }
            _ => Ok("unrecognized".as_bytes()),
        }
    }
}

impl Handler for DiskHandler {
    fn handle(&self, bytes: &[u8]) -> &[u8] {
        match self.handle_result(bytes) {
            Ok(out_bytes) => out_bytes,
            Err(_) => "error encountered".as_bytes(),
        }
    }
}
