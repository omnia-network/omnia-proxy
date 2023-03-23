use std::{collections::HashMap, net::Ipv4Addr};

use uuid::Uuid;

use super::vpn::Vpn;

#[derive(Debug, Default)]
pub struct ProxyDB {
    /// The mapping between IP assigned in the VPN and the public IP of the peer
    pub internal_mapping: HashMap<Ipv4Addr, String>,
    /// The mapping between the public subdomain/id and peer IP assigned in the VPN
    pub external_mapping: HashMap<Uuid, Ipv4Addr>,

    /// The VPN instance
    pub vpn: Vpn,
}

impl ProxyDB {
    pub fn new() -> Self {
        let mut instance = ProxyDB::default();

        instance.vpn = instance.new_vpn();

        instance
    }

    fn new_vpn(&mut self) -> Vpn {
        let vpn = Vpn::new().expect("Error creating VPN");
        println!("Initialized VPN: {:?}", vpn);

        // we also need to map the registered peers in the DB
        vpn.peers.iter().for_each(|peer| {
            match peer.remote_address.clone() {
                Some(addr) => {
                    let peer_vpn_ip = peer.allowed_ips[0].clone();

                    self.map_peer_addresses(addr.ip().to_string(), peer_vpn_ip.to_string());
                }
                None => println!("Peer remote address not set, skipping mapping..."),
            };
        });

        vpn
    }

    pub fn map_peer_addresses(&mut self, peer_public_ip: String, peer_vpn_ip: String) {
        let peer_id = Uuid::new_v4();

        println!(
            "Mapping peer public IP {} to VPN IP {}. Assigned ID: {}",
            peer_public_ip, peer_vpn_ip, peer_id
        );

        // TODO: handle unwrap
        let peer_vpn_ip: Ipv4Addr = peer_vpn_ip.parse().unwrap();

        self.internal_mapping
            .insert(peer_vpn_ip.clone(), peer_public_ip);
        self.external_mapping.insert(peer_id, peer_vpn_ip);
    }

    // TODO: handle unwraps
    pub fn get_peer_public_ip(&mut self, peer_vpn_ip: Ipv4Addr) -> String {
        match self.internal_mapping.get(&peer_vpn_ip) {
            Some(peer_public_ip) => peer_public_ip.to_owned(),
            None => {
                // we need to read if from wg
                self.vpn
                    .refresh_and_get_peer(peer_vpn_ip)
                    .unwrap()
                    .remote_address
                    .unwrap()
                    .ip()
                    .to_string()
            }
        }
    }
}
