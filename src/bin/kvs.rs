#[macro_use]
extern crate clap;
#[macro_use]
extern crate failure;

use clap::{App, Arg, SubCommand};

fn main() -> kvs::Result<()> {
    let key_arg = Arg::with_name("key").required(true);
    let matches = App::new(crate_name!())
        .version(crate_version!())
        .author(crate_authors!())
        .about(crate_description!())
        .subcommand(
            SubCommand::with_name("set")
                .about("Sets a value for a key")
                .arg(key_arg.clone())
                .arg(Arg::with_name("value").required(true)),
        )
        .subcommand(
            SubCommand::with_name("get")
                .about("Gets the value for a key")
                .arg(key_arg.clone()),
        )
        .subcommand(
            SubCommand::with_name("rm")
                .about("Delete an entry")
                .arg(key_arg.clone()),
        )
        .get_matches();

    let mut kvs = kvs::KvStore::open(std::path::Path::new("."))?;

    match matches.subcommand() {
        ("set", Some(set_matches)) => {
            if let (Some(key), Some(value)) =
                (set_matches.value_of("key"), set_matches.value_of("value"))
            {
                kvs.set(key.to_string(), value.to_string())?;
                Ok(())
            } else {
                panic!("WTF: {:?}", set_matches);
            }
        }
        ("get", Some(get_matches)) => {
            if let Some(key) = get_matches.value_of("key") {
                match kvs.get(key.to_string())? {
                    Option::Some(value) => {
                        println!("{}", value);
                        Ok(())
                    }
                    Option::None => Err(format_err!("Key not found")),
                }
            } else {
                panic!("WTF: {:?}", get_matches);
            }
        }
        ("rm", Some(rm_matches)) => {
            if let Some(key) = rm_matches.value_of("key") {
                kvs.remove(key.to_string())?;
                Ok(())
            } else {
                panic!("WTF: {:?}", rm_matches);
            }
        }
        ("", None) => panic!("A valid subcommand is required"),
        other => panic!("WTF {:?}", other),
    }
}
