use std::collections::HashMap;
use std::convert::TryFrom;
use std::fs::File;
use std::io::prelude::*;
use std::net::{IpAddr, SocketAddr};
use std::process::{Command, Stdio};

use anyhow;
use chrono::offset::Utc;
use chrono::DateTime;
use clap::Arg;
use ipnet::IpNet;
use serde::{Deserialize, Serialize};

pub mod server;

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
    subnet: IpNet,
    hosts: Vec<Host>,
}

impl std::default::Default for Config {
    fn default() -> Self {
        Config {
            subnet: "10.42.0.0/24".parse().unwrap(),
            hosts: Vec::new(),
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
        for existing_host in self.hosts.iter() {
            if existing_host.name == host.name {
                return Err(anyhow::anyhow!(
                    "host with name \"{}\" already exists",
                    host.name
                ));
            }
        }
        self.hosts.push(host);
        Ok(())
    }

    /// Remove a host from the config by name
    pub fn remove_host(&mut self, name: &str) {
        self.hosts.retain(|host| host.name != name);
    }

    pub fn hosts_by_name<'a>(&'a self) -> HashMap<String, &'a Host> {
        let mut out = HashMap::new();
        for host in self.hosts.iter() {
            out.insert(host.name.clone(), host);
        }
        out
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Host {
    name: String,
    last_seen: Option<DateTime<Utc>>,
    wireguard_address: Option<IpAddr>,
    public_key: String,
    private_key: String,
    interfaces: Vec<IpNet>,
}

impl TryFrom<&clap::ArgMatches> for Host {
    type Error = anyhow::Error;

    fn try_from(m: &clap::ArgMatches) -> anyhow::Result<Self> {
        Ok(Host {
            name: m.value_of("name").unwrap().into(),
            wireguard_address: m.value_of("wireguard_address").and_then(|s| s.parse().ok()),
            public_key: m
                .value_of("public_key")
                .map(String::from)
                .unwrap_or_else(|| String::new()),
            private_key: m
                .value_of("private_key")
                .map(String::from)
                .unwrap_or_else(|| String::new()),
            last_seen: None,
            interfaces: m
                .value_of("interfaces")
                .iter()
                .filter_map(|i| i.parse().ok())
                .collect(),
        })
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
