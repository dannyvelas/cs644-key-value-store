mod store;

use clap::{Command, arg};

fn cli() -> Command {
    Command::new("git")
        .about("A fictional key-value store")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(
            Command::new("set")
                .about("sets a value in the key-value store")
                .arg(arg!(-k --key <KEY> "The key of your new entry"))
                .arg(arg!(-v --value <VALUE> "The value of your new entry"))
                .arg_required_else_help(true),
        )
        .subcommand(
            Command::new("get")
                .about("get a value from the key-value store")
                .arg(arg!(-k --key <KEY> "The key to query the key-value store"))
                .arg_required_else_help(true),
        )
}

fn main() {
    let mut disk_map = store::DiskMap::new();
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
            disk_map.set(k, v);
        }
        Some(("get", sub_matches)) => {
            let key = sub_matches.get_one::<String>("key").expect("required");
            disk_map.get(key);
        }
        _ => unreachable!(),
    }
}
