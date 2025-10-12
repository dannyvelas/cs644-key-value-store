mod store;

use clap::{Command, arg};

fn cli() -> Command {
    Command::new("diskmap")
        .about("A disk key-value store")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(
            Command::new("set")
                .about("sets a value in the key-value store")
                .arg(arg!(key: <KEY> "The key of your new entry"))
                .arg(arg!(value: <VALUE> "The value of your new entry"))
                .arg_required_else_help(true),
        )
        .subcommand(
            Command::new("get")
                .about("get a value from the key-value store")
                .arg(arg!(key: <KEY> "The key to query the key-value store"))
                .arg_required_else_help(true),
        )
        .subcommand(Command::new("size").about("get the size of the key-value store"))
        .subcommand(Command::new("dump").about("print key-value store"))
}

fn main() {
    let disk_map = store::DiskMap::new("/tmp/map").unwrap();
    let matches = cli().get_matches();

    match matches.subcommand() {
        Some(("set", sub_matches)) => {
            let k = sub_matches
                .get_one::<String>("key")
                .expect("required")
                .as_str();
            let v = sub_matches
                .get_one::<String>("value")
                .expect("required")
                .as_str();
            disk_map.set(k, v).unwrap();
        }
        Some(("get", sub_matches)) => {
            let key = sub_matches.get_one::<String>("key").expect("required");
            if let Some(value) = disk_map.get(key) {
                println!("{}", value);
            } else {
                println!("No value found for {}", key);
            }
        }
        Some(("size", _)) => {
            if let Err(e) = disk_map.size() {
                println!("error calling size: {}", e)
            }
        }
        Some(("dump", _)) => {
            println!("{:#?}", disk_map.m);
        }
        _ => unreachable!(),
    }
}
