use std::{
    borrow::BorrowMut,
    collections::BTreeMap,
    net::{Ipv4Addr, SocketAddr},
    str::FromStr,
};

use serde::{Deserialize, Serialize};

use crate::models::GenericError;

use super::{
    docker::wg_docker_command,
    ip::next_available_ipv4_address,
    models::{AssignedIpsMap, RegisteredPeer, RegisteredPeersMap},
};

const WG_NETMASK: Ipv4Addr = Ipv4Addr::new(255, 255, 255, 0);
// this is the address reserved for the wireguard interface
const WG_FIRST_ADDR: Ipv4Addr = Ipv4Addr::new(10, 13, 13, 1);

/// Checks if Wireguard is running
pub fn check_vpn() -> Result<String, GenericError> {
    wg_docker_command(vec!["show"], false)
}

/// Gets the interface name of the VPN
pub fn get_interface_name() -> Result<String, GenericError> {
    let output = wg_docker_command(vec!["show", "interfaces"], false);

    match output {
        Ok(result) => {
            // `result` should just contain the interface name, let's trim it to be sure
            Ok(result.trim().to_string())
        }
        Err(e) => panic!("Error getting interface name: {}", e),
    }
}

/// Gets the public key of the VPN
/// This is the public key of the interface
pub fn get_public_key(interface_name: &str) -> Result<String, GenericError> {
    let output = wg_docker_command(vec!["show", interface_name, "public-key"], false);

    match output {
        Ok(result) => {
            // `result` should just contain the public key, let's trim it to be sure
            Ok(result.trim().to_string())
        }
        Err(e) => panic!("Error getting public key: {}", e),
    }
}

/// Get the peer configuration from the VPN
/// It executes the command `wg show wg0 dump` and extracts the peer config
pub fn get_peer_config_by_vpn_ip(peer_vpn_ip: Ipv4Addr) -> Result<RegisteredPeer, GenericError> {
    let output = wg_docker_command(vec!["show", "wg0", "dump"], false);

    match output {
        Ok(result) => {
            // first line is the interface config, so we skip it
            let lines = result.lines().skip(1);

            for line in lines {
                let mut split = line.split('\t');

                let public_key = split.next().unwrap();
                let preshared_key = split.next().unwrap();
                let remote_address = split.next().unwrap();
                let allowed_ips = split.next().unwrap();

                let allowed_ips = allowed_ips
                    .split(',')
                    // TODO: handle unwrap
                    .map(|ip| Ipv4Addr::from_str(ip.split('/').next().unwrap()).unwrap())
                    .collect::<Vec<Ipv4Addr>>();

                if allowed_ips.contains(&peer_vpn_ip) {
                    return Ok(RegisteredPeer {
                        public_key: public_key.to_string(),
                        preshared_key: if preshared_key == "" || preshared_key == "(none)" {
                            None
                        } else {
                            Some(preshared_key.to_string())
                        },
                        remote_address: if remote_address == "" || remote_address == "(none)" {
                            None
                        } else {
                            Some(
                                remote_address
                                    .parse()
                                    .expect("Error parsing remote ip for peer"),
                            )
                        },
                        allowed_ips,
                    });
                }
            }
            Err(format!("Peer {} not found", peer_vpn_ip))
        }
        Err(e) => Err(format!("Error getting peer config: {}", e)),
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Vpn {
    pub interface_name: String,
    pub interface_public_key: String,
    /// The peers of the VPN: peer public key -> peer
    pub peers: RegisteredPeersMap,
    /// The assigned ips of the peers: ip -> peer public key
    pub assigned_ips: AssignedIpsMap,
}

impl Vpn {
    pub fn new() -> Result<Self, GenericError> {
        match get_interface_name() {
            Ok(interface_name) => match get_public_key(interface_name.as_str()) {
                Ok(public_key) => {
                    let mut vpn = Self {
                        interface_name,
                        interface_public_key: public_key,
                        peers: BTreeMap::new(),
                        assigned_ips: BTreeMap::new(),
                    };

                    match vpn.get_registered_peers() {
                        Ok(peers) => {
                            vpn.peers = peers;
                            Ok(vpn)
                        }
                        Err(e) => Err(e),
                    }
                }
                Err(e) => Err(format!("Error getting public key: {}", e)),
            },
            Err(e) => Err(format!("Error creating VPN: {}", e)),
        }
    }

    /// Gets the registered peers of the VPN
    /// and saves them to the `peers` field
    pub fn get_registered_peers(&mut self) -> Result<RegisteredPeersMap, GenericError> {
        let output = wg_docker_command(vec!["show", self.interface_name.as_str(), "dump"], false);

        match output {
            Ok(result) => {
                let mut peers: RegisteredPeersMap = BTreeMap::new();

                // first line is the interface config, so we skip it
                result.lines().skip(1).for_each(|peer| {
                    let mut split = peer.split('\t');

                    let public_key = split.next().unwrap();
                    let preshared_key = split.next().unwrap();
                    let remote_address = split.next().unwrap();
                    let allowed_ips = split.next().unwrap();

                    let allowed_ips = allowed_ips.split(',').fold(
                        Vec::new(),
                        |mut allowed_ips: Vec<Ipv4Addr>, ip| {
                            if ip == "" || ip == "(none)" {
                                println!("No ip found for peer {public_key}, skipping...");
                            } else {
                                allowed_ips.push(
                                    // `ip` should be in the format of `ip/mask`, so we need to remove the mask
                                    Ipv4Addr::from_str(ip.split('/').next().unwrap())
                                        .expect(format!("Error parsing ip {ip}").as_str()),
                                )
                            }

                            allowed_ips
                        },
                    );

                    if allowed_ips.len() == 0 {
                        println!("No ip found for peer {public_key}, skipping...");
                    } else {
                        peers.insert(
                            public_key.to_string(),
                            RegisteredPeer {
                                public_key: public_key.to_string(),
                                preshared_key: if preshared_key == "" || preshared_key == "(none)" {
                                    None
                                } else {
                                    Some(preshared_key.to_string())
                                },
                                remote_address: if remote_address == ""
                                    || remote_address == "(none)"
                                {
                                    None
                                } else {
                                    Some(
                                        remote_address
                                            .parse()
                                            .expect("Error parsing remote ip for peer"),
                                    )
                                },
                                allowed_ips: allowed_ips.clone(),
                            },
                        );

                        self.assigned_ips
                            .insert(allowed_ips[0].clone(), public_key.to_string());
                    }
                });

                Ok(peers)
            }
            Err(e) => panic!("Error getting registered peers: {}", e),
        }
    }

    /// Adds a peer to the VPN. If the peer already exists, updates its the remote address.
    /// If the peer is new, it automatically assigns the next available vpn ip to the peer
    /// `public_key`: the public key of the peer to add to the vpn
    /// `preshared_key`: the preshared key of the peer to add to the vpn
    pub fn add_or_update_peer(
        &mut self,
        public_key: String,
        preshared_key: Option<String>,
        remote_address: Option<SocketAddr>,
    ) -> Result<RegisteredPeer, GenericError> {
        // if peer already exists, update its remote address and return it

        match self.peers.get(&public_key).borrow_mut() {
            Some(peer) => {
                peer.to_owned().remote_address = remote_address;

                Ok(peer.to_owned())
            }
            None => {
                // otherwise, add the peer to the vpn
                let ip_addr =
                    next_available_ipv4_address(&self.assigned_ips, WG_NETMASK, WG_FIRST_ADDR);

                match ip_addr {
                    Some(ip_addr) => {
                        match wg_docker_command(
                            vec![
                                "set",
                                self.interface_name.as_str(),
                                "peer",
                                public_key.as_str(),
                                "allowed-ips",
                                ip_addr.to_string().as_str(),
                            ],
                            false,
                        ) {
                            Ok(_) => {
                                // we need to restart the interface to apply the changes
                                wg_docker_command(vec!["down", self.interface_name.as_str()], true)
                                    .expect("Error restarting interface");
                                wg_docker_command(vec!["up", self.interface_name.as_str()], true)
                                    .expect("Error restarting interface");

                                let peer = RegisteredPeer {
                                    public_key,
                                    preshared_key,
                                    remote_address,
                                    allowed_ips: vec![ip_addr],
                                };

                                self.peers.insert(peer.public_key.clone(), peer.clone());
                                self.assigned_ips.insert(ip_addr, peer.public_key.clone());

                                Ok(peer)
                            }
                            Err(e) => Err(format!(
                                "Error adding peer with public key {public_key}: {e}"
                            )
                            .to_string()),
                        }
                    }
                    None => Err(format!(
                        "Error adding peer with public key {public_key}: No available ip address"
                    )
                    .to_string()),
                }
            }
        }
    }

    /// searches for the peer with the given internal vpn ip
    /// and updates the internal list of peers
    /// `ip`: the internal vpn ip of the peer to search for
    /// returns the peer with the given internal vpn ip
    /// TODO: extremely inefficient, improve this
    pub fn refresh_and_get_peer(&mut self, ip: Ipv4Addr) -> Result<RegisteredPeer, GenericError> {
        // we update the internal list of peers
        // and then we search for the peer with the given internal vpn ip
        match get_peer_config_by_vpn_ip(ip) {
            Ok(peer_config) => self.add_or_update_peer(
                peer_config.public_key,
                peer_config.preshared_key,
                peer_config.remote_address,
            ),
            Err(e) => Err(format!("Error getting peer config: {}", e)),
        }
    }
}
