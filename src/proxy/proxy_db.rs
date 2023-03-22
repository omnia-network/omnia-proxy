use std::collections::HashMap;

use uuid::Uuid;

use super::vpn::Vpn;

#[derive(Debug)]
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
        Self {
            internal_mapping: HashMap::new(),
            external_mapping: HashMap::new(),
            vpn: Self::new_vpn(),
        }
    }

    fn new_vpn() -> Vpn {
        let vpn = Vpn::new().expect("Error creating VPN");
        println!("Initialized VPN: {:?}", vpn);

        vpn
    }

    pub fn map_peer_addresses(&mut self, peer_public_ip: String, peer_vpn_ip: String) {
        let peer_id = Uuid::new_v4();

        self.internal_mapping
            .insert(peer_public_ip, peer_vpn_ip.clone());
        self.external_mapping.insert(peer_id, peer_vpn_ip);
    }
}
