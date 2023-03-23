use std::{
    net::SocketAddr,
    sync::{Arc, Mutex},
};
use warp::reply::{json, Json};

use crate::{env::get_env_var, proxy::proxy_db::ProxyDB};

use super::models::{ApiError, RegisterPeerRequestBody, RegisterPeerResponseBody};

// registers the new peer to the vpn, sending a docker command to wireguard
// saves the remote_address of the peer to a mapping
pub fn handle_register_to_vpn(
    proxy_db: Arc<Mutex<ProxyDB>>,
    remote_address: Option<SocketAddr>,
    request_body: RegisterPeerRequestBody,
) -> Result<Json, ApiError> {
    let mut proxy_db = proxy_db.lock().unwrap();

    if let Some(addr) = remote_address {
        println!("Remote address: {}", addr);
        println!("Registering peer: {:?}", request_body);

        match proxy_db.vpn.add_or_update_peer(
            request_body.public_key,
            request_body.preshared_key,
            Some(addr),
        ) {
            Ok(peer) => {
                println!("Registered peer: {:?}", peer);

                let peer_public_ip = addr.ip().to_string();
                let peer_vpn_ip = peer.allowed_ips[0].to_string();
                proxy_db.map_peer_addresses(peer_public_ip, peer_vpn_ip.clone());

                let response = RegisterPeerResponseBody {
                    server_public_key: proxy_db.vpn.interface_public_key.clone(),
                    assigned_ip: peer_vpn_ip,
                    proxy_address: get_env_var("PROXY_INTERNAL_ADDRESS"),
                };

                Ok(json(&response))
            }
            Err(e) => {
                let error = ApiError {
                    message: format!("Error registering peer: {}", e),
                };

                println!("{:?}", error);
                Err(error)
            }
        }
    } else {
        let error = ApiError {
            message: format!("Error registering peer: No remote address"),
        };

        println!("{:?}", error);
        Err(error)
    }
}
