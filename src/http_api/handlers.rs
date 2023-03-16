use std::{
    net::SocketAddr,
    sync::{Arc, Mutex},
};
use warp::reply::{json, Json};

use crate::vpn::vpn::VPN;

use super::models::{ApiError, RegisterPeerRequestBody, RegisterPeerResponseBody};

// registers the new peer to the vpn, sending a docker command to wireguard
// saves the remote_address of the peer to a mapping
pub fn handle_register_to_vpn(
    vpn: Arc<Mutex<VPN>>,
    remote_address: Option<SocketAddr>,
    request_body: RegisterPeerRequestBody,
) -> Result<Json, ApiError> {
    let mut vpn = vpn.lock().unwrap();

    if let Some(addr) = remote_address {
        println!("Remote address: {}", addr);
        println!("Registering peer: {:?}", request_body);

        match vpn.add_peer(request_body.public_key, request_body.preshared_key) {
            Ok(peer) => {
                println!("Registered peer: {:?}", peer);
                let response = RegisterPeerResponseBody {};
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