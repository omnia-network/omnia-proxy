use std::collections::HashMap;

use uuid::Uuid;

use super::vpn::Vpn;

#[derive(Debug, Default)]
pub struct ProxyDB {
    /// The mapping between peer public IP and IP assigned in the VPN
    pub internal_mapping: HashMap<String, String>,
    /// The mapping between the public subdomain/id and peer IP assigned in the VPN
    pub external_mapping: HashMap<Uuid, String>,

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
            let peer_public_ip = peer.remote_address.clone().unwrap();
            let peer_vpn_ip = peer.allowed_ips[0].clone();

            self.map_peer_addresses(peer_public_ip.ip().to_string(), peer_vpn_ip.to_string());
        });

        vpn
    }

    pub fn map_peer_addresses(&mut self, peer_public_ip: String, peer_vpn_ip: String) {
        let peer_id = Uuid::new_v4();

        self.internal_mapping
            .insert(peer_public_ip, peer_vpn_ip.clone());
        self.external_mapping.insert(peer_id, peer_vpn_ip);
    }
}
