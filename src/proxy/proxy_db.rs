use std::{collections::BTreeMap, fs, net::Ipv4Addr};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{models::PeerInfo, vpn::Vpn};

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct ProxyDb {
    /// The mapping between IP assigned in the VPN and the public IP of the peer
    pub internal_mapping: BTreeMap<Ipv4Addr, PeerInfo>,
    /// The mapping between the public subdomain/id and peer IP assigned in the VPN
    pub external_mapping: BTreeMap<Uuid, Ipv4Addr>,

    /// The VPN instance
    pub vpn: Vpn,
}

impl ProxyDb {
    pub fn new() -> Self {
        let mut instance = ProxyDb::default();

        instance.vpn = instance.new_vpn();

        instance
    }

    fn new_vpn(&mut self) -> Vpn {
        let vpn = Vpn::new().expect("Error creating VPN");
        println!("Initialized VPN: {:?}", vpn);

        // we also need to map the registered peers in the DB
        vpn.peers.iter().for_each(|(_, peer)| {
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

    pub fn map_peer_addresses(&mut self, peer_public_ip: String, peer_vpn_ip: String) -> Uuid {
        let peer_id = Uuid::new_v4();

        println!(
            "Mapping peer public IP {} to VPN IP {}. Assigned ID: {}",
            peer_public_ip, peer_vpn_ip, peer_id
        );

        // TODO: handle unwrap
        let peer_vpn_ip: Ipv4Addr = peer_vpn_ip.parse().unwrap();

        self.internal_mapping.insert(
            peer_vpn_ip.clone(),
            PeerInfo {
                id: peer_id,
                public_ip: peer_public_ip,
            },
        );
        self.external_mapping.insert(peer_id, peer_vpn_ip);

        // save db
        // TODO: improve the logic to save db to file
        self.save_db();

        peer_id
    }

    // TODO: handle unwraps
    /// Get the public IP of a peer given its VPN IP
    /// If the public IP can't be found, reads the public IP from the VPN and updates the DB accordingly
    pub fn get_peer_public_ip(&mut self, peer_vpn_ip: Ipv4Addr) -> Result<String, String> {
        match self.internal_mapping.get(&peer_vpn_ip) {
            Some(peer_info) => Ok(peer_info.public_ip.to_owned()),
            None => {
                // we need to read it from wg
                match self.vpn.refresh_and_get_peer(peer_vpn_ip) {
                    Ok(peer) => {
                        match peer.remote_address.clone() {
                            Some(addr) => {
                                let peer_public_ip = addr.ip().to_string();

                                // save the DB to disk
                                // TODO: change the logic for saving the db to file
                                self.save_db();

                                Ok(peer_public_ip)
                            }
                            None => Err("Peer remote address not set".to_string()),
                        }
                    }
                    Err(e) => Err(e),
                }
            }
        }
    }

    /// Get the internal VPN IP of a peer given its ID
    pub fn get_peer_internal_ip(&self, peer_id: Uuid) -> Result<Ipv4Addr, String> {
        match self.external_mapping.get(&peer_id) {
            Some(peer_internal_ip) => Ok(peer_internal_ip.to_owned()),
            None => Err(format!("Peer with id {peer_id} not found")),
        }
    }

    /// Get the peer info
    pub fn get_peer_info(&self, peer_vpn_ip: Ipv4Addr) -> Result<PeerInfo, String> {
        match self.internal_mapping.get(&peer_vpn_ip) {
            Some(peer_info) => Ok(peer_info.to_owned()),
            None => Err(format!("Peer with internal IP {peer_vpn_ip} not found")),
        }
    }

    /// Load the DB from disk
    /// If the DB doesn't exist, create a new one
    pub fn load_db() -> Self {
        match fs::read_to_string("data/db.json") {
            Ok(db_json) => {
                println!("Loading DB from disk...");
                // TODO: handle unwrap
                let instance: ProxyDb = serde_json::from_str(&db_json).unwrap();

                println!("Initialized VPN: {:?}", instance.vpn);

                instance
            }
            Err(_) => {
                println!("DB not found, creating new one...");
                Self::new()
            }
        }
    }

    /// Save the DB to disk
    pub fn save_db(&self) {
        // TODO: handle unwrap
        let db_json = serde_json::to_string(&self).unwrap();

        println!("serialized db: {db_json}");

        fs::write("data/db.json", db_json).unwrap();
    }
}
