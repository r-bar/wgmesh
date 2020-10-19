use std::convert::TryFrom;

use actix;
use wgmesh::{cli, configure_logging, generate_private_key, Config, Host};

fn main() {
    let localhost = Host::local().unwrap();
    let private_key = generate_private_key().unwrap();
    let args = cli().get_matches();
    configure_logging(args.value_of("log_level").unwrap_or("info"));
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
            let name = m.value_of("name").expect("host name not provided");
            //config.remove_host(&name);
            println!("Removed {} from network", &name);
        }
        _ => unreachable!(),
    }
    //cli().print_long_help();
}
