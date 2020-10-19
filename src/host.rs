use std::convert::TryFrom;
use std::net::{IpAddr, Ipv6Addr};
use std::process::{Command, Stdio};
use std::str::FromStr;

use chrono::offset::Utc;
use chrono::DateTime;
use ipnet::{IpNet, Ipv6Net};
use lazy_static::lazy_static;
use rand;
use regex::Regex;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::uuidv1;

lazy_static! {
    pub static ref IFACE_ADDR_RE: Regex =
        Regex::new(r"inet (\d+\.\d+\.\d+\.\d+/\d+)|inet6 (([0-9a-f]:*)+/\d+)").unwrap();
    pub static ref IFACE_NAME: Regex = Regex::new(r"^\d+: ([0-9a-zA-Z\-@]+)").unwrap();
    pub static ref IFACE_STATE: Regex = Regex::new(r"state (\w+)").unwrap();
    pub static ref IFACE_MAC: Regex = Regex::new(r"link/\w+ (([0-9a-f]{2}:?){6})").unwrap();
}

/// Create a random ipv6 address in the unique local scope. Follows RFC 4193 reccomendations.
///
/// > global_id: 40 bits, default = 0
/// >
/// > subnet_id: 16 bits, default = 0
/// >
/// > iface_id: 64 bits, default = random()
///
/// https://tools.ietf.org/html/rfc4193#section-3.2.1
pub fn generate_ipv6(
    global_id: Option<u64>,
    subnet_id: Option<u16>,
    iface_id: Option<u64>,
) -> anyhow::Result<Ipv6Addr> {
    let base_prefix: u16 = 0xfc00;
    let global_id: u64 = global_id.unwrap_or_default();
    if global_id >= 40_u64.pow(2) {
        return Err(anyhow::anyhow!("global_id may only be 40 bits wide"));
    }
    let subnet_id: u16 = subnet_id.unwrap_or_default();
    let iface_id: u64 = iface_id.unwrap_or_else(|| rand::random());
    Ok(Ipv6Addr::new(
        base_prefix + ((global_id >> 32) as u16),
        (global_id >> 16) as u16,
        global_id as u16,
        subnet_id,
        (iface_id >> 48) as u16,
        (iface_id >> 32) as u16,
        (iface_id >> 16) as u16,
        iface_id as u16,
    ))
}

pub fn local_hostname() -> anyhow::Result<String> {
    Ok(
        String::from_utf8(Command::new("hostname").output()?.stdout)?
            .trim()
            .to_owned(),
    )
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Host {
    pub name: String,
    pub last_seen: Option<DateTime<Utc>>,
    pub wireguard_address: IpNet,
    pub public_key: String,
    pub private_key: String,
    interfaces: Vec<Interface>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Interface {
    name: String,
    mac: String,
    state: String,
    addresses: Vec<IpNet>,
}

impl Interface {
    pub fn local() -> anyhow::Result<Vec<Self>> {
        let cmd = Command::new("ip").args(&["addr", "show"]).output()?;
        let output = String::from_utf8(cmd.stdout)?;

        let interface_strings: Vec<String> =
            output
                .lines()
                .fold(Vec::new(), |mut acc: Vec<String>, line| {
                    if IFACE_NAME.is_match(&line) {
                        acc.push(line.into());
                    } else {
                        let iface = acc.pop().unwrap();
                        acc.push(String::from(format!("{}\n{}", iface, line)));
                    }
                    acc
                });

        Ok(interface_strings
            .iter()
            .map(|s| Interface::from_str(&s).unwrap())
            .collect())
    }
}

impl FromStr for Interface {
    type Err = anyhow::Error;

    fn from_str(data: &str) -> anyhow::Result<Self> {
        let lines: Vec<&str> = data.lines().collect();
        let name = IFACE_NAME
            .captures(lines[0])
            .and_then(|cap| cap.get(1))
            .map(|m| m.as_str().to_owned())
            .ok_or(anyhow::anyhow!("unable to parse interface name"))?;
        let state = IFACE_STATE
            .captures(lines[0])
            .and_then(|cap| cap.get(1))
            .map(|m| m.as_str().to_owned())
            .ok_or(anyhow::anyhow!("unable to parse interface state"))?;
        let addresses: Vec<IpNet> = lines
            .iter()
            .skip(2)
            .filter_map(|interface| {
                let cap = IFACE_ADDR_RE.captures(interface)?;
                // capture 1 is ipv4, capture2 is ipv6
                cap.get(1)
                    .or_else(|| cap.get(2))
                    .and_then(|c| c.as_str().parse().ok())
            })
            .collect();
        dbg!(lines[1]);
        let mac_cap = IFACE_MAC.captures(lines[1]);
        dbg!(&mac_cap);
        let mac = mac_cap
            .and_then(|cap| cap.get(1))
            .map(|m| m.as_str().to_owned())
            .ok_or(anyhow::anyhow!("unable to parse MAC address"))?;
        Ok(Interface {
            name,
            mac,
            state,
            addresses,
        })
    }
}

impl Host {
    /// Return the host object for the local system
    pub fn local() -> anyhow::Result<Self> {
        let name = local_hostname()?;
        Ok(Host {
            name,
            last_seen: None,
            wireguard_address: IpNet::V6(Ipv6Net::new(generate_ipv6(None, None, None)?, 64)?),
            public_key: String::new(),
            private_key: String::new(),
            interfaces: Interface::local()?,
        })
    }
}

impl Default for Host {
    fn default() -> Self {
        Host {
            name: String::new(),
            last_seen: None,
            wireguard_address: IpNet::V6(
                Ipv6Net::new(Ipv6Addr::new(0xfc00, 0, 0, 0, 0, 0, 0, 0), 64).unwrap(),
            ),
            public_key: String::new(),
            private_key: String::new(),
            interfaces: Vec::new(),
        }
    }
}

impl TryFrom<&clap::ArgMatches> for Host {
    type Error = anyhow::Error;

    fn try_from(m: &clap::ArgMatches) -> anyhow::Result<Self> {
        Ok(Host {
            name: m
                .value_of("name")
                .ok_or(anyhow::anyhow!("name argument not provided"))?
                .into(),
            wireguard_address: m
                .value_of("wireguard_address")
                .and_then(|s| s.parse().ok())
                .ok_or(anyhow::anyhow!("invalid wireguard address argument"))?,
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
