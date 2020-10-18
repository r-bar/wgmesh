use std::convert::TryFrom;
use std::net::SocketAddr;

use actix;
use wgmesh::{cli, generate_private_key, generate_public_key, Config, Host};

fn main() {
    let private_key = generate_private_key().unwrap();
    print!("{}", &private_key);
    print!("{}", generate_public_key(&private_key).unwrap());
    print!("{}", generate_public_key(&private_key).unwrap());
    let args = cli().get_matches();
    let config_path = args.value_of("config").unwrap();
    let mut config = match Config::try_from_path(&config_path) {
        Ok(config) => config,
        Err(_) => Config::default(),
    };
    config.save(&config_path).expect("could not save config");
    match args.subcommand() {
        Some(("add-host", m)) => {
            let host = Host::try_from(m).unwrap();
            config.add_host(host).unwrap();
        }
        Some(("server", m)) => {
            println!("server");
            let bind = m.value_of("bind").and_then(|b| b.parse().ok()).unwrap();
            actix::run(async move {
                wgmesh::server::server(bind, config).await.unwrap();
            })
            .unwrap();
        }
        Some(("remove-host", m)) => {
            println!("remove-host");
        }
        _ => {
            panic!("unknown command");
        }
    }
    //cli().print_long_help();
}
