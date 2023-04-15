use std::{
    collections::BTreeMap,
    net::{Ipv4Addr, SocketAddr},
};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
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

/// The peers of the VPN: peer public key -> peer
pub type RegisteredPeersMap = BTreeMap<String, RegisteredPeer>;
/// The assigned ips of the peers: ip -> peer public key
pub type AssignedIpsMap = BTreeMap<Ipv4Addr, String>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerInfo {
    /// The id of the peer
    pub id: Uuid,
    /// The public ip of the peer
    pub public_ip: String,
}
