use std::net::{Ipv4Addr, SocketAddr};

#[derive(Debug, Clone)]
pub struct RegisteredPeer {
    /// The public key of the peer
    pub public_key: String,
    /// The preshared key of the peer
    pub preshared_key: Option<String>,
    /// The remote address of the peer
    pub remote_address: Option<SocketAddr>,
    /// The allowed ips of the peer, which in our case should only contain the ip of the peer
    pub allowed_ips: Vec<Ipv4Addr>,
}
