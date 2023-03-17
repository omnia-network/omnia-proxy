use std::collections::HashMap;

use super::vpn::VPN;

#[derive(Debug)]
pub struct ProxyDB {
    /// The mapping between peer public IP and IP assigned in the VPN
    pub internal_mapping: HashMap<String, String>,
    /// The mapping between peer IP assigned in the VPN and the public subdomain
    pub external_mapping: HashMap<String, String>,

    /// The VPN instance
    pub vpn: VPN,
}

impl ProxyDB {
    pub fn new() -> Self {
        Self {
            internal_mapping: HashMap::new(),
            external_mapping: HashMap::new(),
            vpn: Self::new_vpn(),
        }
    }

    fn new_vpn() -> VPN {
        let vpn = VPN::new().expect("Error creating VPN");
        println!("Initialized VPN: {:?}", vpn);

        vpn
    }

    pub fn add_peer(
        &mut self,
        peer_public_ip: String,
        peer_vpn_ip: String,
        peer_subdomain: String,
    ) {
        self.internal_mapping
            .insert(peer_public_ip, peer_vpn_ip.clone());
        self.external_mapping.insert(peer_vpn_ip, peer_subdomain);
    }
}
