use std::collections::HashMap;
use std::fs::File;
use std::io::prelude::*;
use std::process::{Command, Stdio};
use std::str::FromStr;

use anyhow;
use chrono::{DateTime, Utc};
use clap::Arg;
use ipnet::IpNet;
use log;
use rand;
use serde::{Deserialize, Serialize};
use simple_logger::SimpleLogger;
use uuid::v1::{Context, Timestamp};
use uuid::Uuid;

pub mod host;
pub mod server;

pub use host::Host;

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Event {
    id: Uuid,
    created_at: DateTime<Utc>,
    data: EventData,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum EventData {
    Connect { host: Host },
    Disconnect { host: Host },
}

impl Event {
    fn new(data: EventData) -> Self {
        Event {
            id: uuidv1(None).unwrap(),
            created_at: Utc::now(),
            data,
        }
    }

    pub fn connect(host: Host) -> Self {
        Event::new(EventData::Connect { host })
    }

    pub fn disconnect(host: Host) -> Self {
        Event::new(EventData::Disconnect { host })
    }

    pub async fn send(self, address: &str) -> anyhow::Result<()> {
        unimplemented!()
    }
}

pub fn configure_logging(log_level: &str) -> anyhow::Result<()> {
    let level = log::LevelFilter::from_str(log_level)?;
    let logger = SimpleLogger::new().with_level(level);
    logger.init()?;
    Ok(())
}

fn timestamp() -> anyhow::Result<u64> {
    use std::time::SystemTime;
    let now = SystemTime::now();
    Ok(now.duration_since(SystemTime::UNIX_EPOCH)?.as_secs())
}

// FIXME: make private again
/// Create a v1 uuid. If no node_id is passed uses the local machine's hostname instead
pub fn uuidv1(node_id: Option<&str>) -> anyhow::Result<Uuid> {
    let context = Context::new(rand::random());
    let mut node_id = match node_id {
        Some(node_id) => String::from(node_id),
        None => host::local_hostname()?,
    };
    let padding = " ".repeat(0_isize.max(6 - (node_id.len() as isize)) as usize);
    node_id.push_str(&padding);
    let ts = Timestamp::from_unix(context, timestamp()?, 0);
    Ok(Uuid::new_v1(ts, node_id.as_bytes())?)
}

/// Build the command line interface
pub fn cli() -> clap::App<'static> {
    clap::App::new("wgmesh")
        .version(clap::crate_version!())
        .about("Generate configuration to run a wireguard mesh network")
        .arg(
            Arg::new("config")
                .short('c')
                .long("config")
                .default_value("network.yaml"),
        )
        .arg(
            Arg::new("log_level")
                .long("log-level")
                .short('l')
                .takes_value(true)
                .default_value("info"),
        )
        .subcommand(
            clap::App::new("add-host")
                .about("Add a host to the config")
                .long_about("Add a host to the config")
                .arg(Arg::new("name"))
                .arg(
                    Arg::new("interfaces")
                        .short('i')
                        .long("interface")
                        .multiple(true),
                )
                .arg(
                    Arg::new("wireguard_address")
                        .short('a')
                        .long("wireguard-address")
                        .takes_value(true),
                )
                .arg(
                    Arg::new("public_key")
                        .short('u')
                        .long("public-key")
                        .takes_value(true),
                )
                .arg(
                    Arg::new("private_key")
                        .short('k')
                        .long("private-key")
                        .takes_value(true),
                )
                .arg(
                    Arg::new("wireguard_port")
                        .short('p')
                        .long("wireguard-port")
                        .takes_value(true),
                ),
        )
        .subcommand(
            clap::App::new("remove-host")
                .about("Remove host from the config")
                .arg(Arg::new("name")),
        )
        .subcommand(clap::App::new("render").about("Render wireguard script from the config"))
        .subcommand(
            clap::App::new("server").about("Start server daemon").arg(
                Arg::new("bind")
                    .long("bind")
                    .short('b')
                    .default_value("0.0.0.0:64001"),
            ),
        )
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Config {
    version: String,
    network_id: Uuid,
    subnet: IpNet,
    host: Host,
    remote_hosts: HashMap<IpNet, Host>,
}

impl std::default::Default for Config {
    fn default() -> Self {
        let host = Host::local().unwrap();
        Config {
            version: String::from("v1"),
            network_id: uuidv1(Some(&host.name)).unwrap(),
            subnet: "10.42.0.0/24".parse().unwrap(),
            host,
            remote_hosts: HashMap::new(),
        }
    }
}

impl Config {
    /// Load config from the given path
    pub fn try_from_path(path: &str) -> anyhow::Result<Self> {
        let file = File::open(path)?;
        Ok(serde_yaml::from_reader(file)?)
    }

    /// Save the config to the given file path.
    pub fn save(&self, path: &str) -> anyhow::Result<()> {
        let file = File::create(path)?;
        serde_yaml::to_writer(file, self)?;
        Ok(())
    }

    /// Render the config into wireguard setup scripts. Scripts will be placed in the given
    /// directory. Any existing files will be overwritten.
    pub fn render(&self, directory: &str) {
        unimplemented!()
    }

    /// Adds a host to the config. Can fail if a host with the same name or addresses already
    /// exists.
    pub fn add_host(&mut self, host: Host) -> anyhow::Result<()> {
        for (_, existing_host) in self.remote_hosts.iter() {
            if existing_host.name == host.name {
                return Err(anyhow::anyhow!(
                    "host with name \"{}\" already exists",
                    host.name
                ));
            }
        }
        self.remote_hosts.insert(host.wireguard_address, host);
        Ok(())
    }

    /// Remove a host from the config by name
    pub fn remove_host(&mut self, ip: &IpNet) {
        self.remote_hosts.remove(ip);
    }

    pub fn hosts_by_name<'a>(&'a self) -> HashMap<String, &'a Host> {
        let mut out = HashMap::new();
        for host in self.remote_hosts.values() {
            out.insert(host.name.clone(), host);
        }
        out
    }
}

/// Equivalent to `wg pubkey < private_key`
pub fn generate_public_key(private_key: &str) -> anyhow::Result<String> {
    let mut cmd = Command::new("wg")
        .arg("pubkey")
        .stdin(Stdio::piped())
        .spawn()?;
    {
        let stdin = cmd
            .stdin
            .as_mut()
            .ok_or(anyhow::anyhow!("could not open process stdin"))?;
        //.ok_or(Err("Could not open process stdin"))?;
        stdin.write_all(private_key.as_bytes())?;
    }
    Ok(String::from_utf8(cmd.wait_with_output()?.stdout)?)
}

/// Equivalent to `wg genkey`
pub fn generate_private_key() -> anyhow::Result<String> {
    let cmd = Command::new("wg").arg("genkey").output()?;
    Ok(String::from_utf8(cmd.stdout)?)
}
