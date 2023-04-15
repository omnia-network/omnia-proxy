use std::{
    net::{IpAddr, SocketAddr},
    sync::{Arc, Mutex},
};
use uuid::Uuid;
use warp::{
    http::{HeaderMap, Method},
    path::FullPath,
    reply::{json, Json},
};
use warp_reverse_proxy::QueryParameters;

use crate::{env::get_env_var, proxy::proxy_db::ProxyDb};

use super::models::{ApiError, ProxyParams, RegisterPeerRequestBody, RegisterPeerResponseBody};

// registers the new peer to the vpn, sending a docker command to wireguard
// saves the remote_address of the peer to a mapping
pub fn handle_register_to_vpn(
    proxy_db: Arc<Mutex<ProxyDb>>,
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
                let peer_id = proxy_db.map_peer_addresses(peer_public_ip, peer_vpn_ip.clone());

                let response = RegisterPeerResponseBody {
                    server_public_key: proxy_db.vpn.interface_public_key.clone(),
                    assigned_ip: peer_vpn_ip,
                    assigned_id: peer_id,
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

/// The `remote_addr` parameter in this case is the address of the peer inside the VPN
/// This function maps the peer's public IP to the peer's VPN IP
pub fn forward_request(
    proxy_db: Arc<Mutex<ProxyDb>>,
    path: FullPath,
    query_params: QueryParameters,
    method: Method,
    remote_addr: Option<SocketAddr>,
    request_headers: HeaderMap,
) -> Result<ProxyParams, ApiError> {
    let mut proxy_db = proxy_db.lock().unwrap();

    println!("Proxying for remote address: {:?}", remote_addr);

    // here we handle two cases:
    // peer -> backend: the peer is sending a request to the backend, we proxy it attaching the peer's public IP
    // backend -> peer: the backend is requesting the peer, inserting peer's ID in the `X-Forward-To-Peer` header
    //                  and eventually the peer port in the `X-Forward-To-Port` header

    let mut headers: HeaderMap = request_headers.clone();

    // TODO: remove empty string return and use better logic
    let proxy_address = match headers.get("x-forward-to-peer") {
        Some(peer_id) => {
            println!("Backend -> Peer");
            // backend -> peer
            // have to forward the request to the peer
            // TODO: handle unwrap
            let p = peer_id.to_str().unwrap();
            println!("Peer ID: {}", p);

            // TODO: handle unwrap
            let peer_internal_ip = match proxy_db.external_mapping.get(&Uuid::try_parse(p).unwrap())
            {
                Some(ip) => ip.to_string(),
                None => "".to_string(),
            };

            println!("Peer internal IP: {}", peer_internal_ip);

            let forward_to_port = match headers.get("x-forward-to-port") {
                Some(port) => port.to_str().unwrap(),
                // default to WoT servient default port
                None => "8888",
            };

            // TODO: change the default port. For not, points to the Gateway WoT Servient port
            format!("http://{peer_internal_ip}:{forward_to_port}/")
        }
        None => {
            println!("Peer -> Backend");
            // peer -> backend
            // we need to add the `Forwarded` header
            // but first we check if the peer is registered
            // if not, we return an empty proxy address

            // just debugging...
            proxy_db.save_db();

            match remote_addr {
                Some(addr) => {
                    match addr.ip() {
                        IpAddr::V4(ip_v4) => {
                            match proxy_db.internal_mapping.get(&ip_v4) {
                                Some(peer_public_ip) => {
                                    // peer is registered
                                    // we add the `Forwarded` header
                                    headers.insert(
                                        "X-Forwarded-For",
                                        peer_public_ip
                                            .parse()
                                            .expect("Failed to parse forwarded header"),
                                    );
                                    get_env_var("OMNIA_BACKEND_CANISTER_URL")
                                }
                                None => {
                                    // peer doesn't have a public ip, let's try to read it from vpn
                                    match proxy_db.get_peer_public_ip(ip_v4) {
                                        Ok(public_ip) => {
                                            format!("http://{}", public_ip).to_string()
                                        }
                                        Err(e) => {
                                            // peer not registered
                                            println!("Peer not registered {:?}", e);
                                            "".to_string()
                                        }
                                    }
                                }
                            }
                        }
                        IpAddr::V6(ip_v6) => {
                            println!("Peer is using IPv6: {}", ip_v6);
                            "".to_string()
                        }
                    }
                }
                None => "".to_string(),
            }
        }
    };

    if proxy_address.is_empty() {
        return Err(ApiError {
            message: "Peer not registered".to_string(),
        });
    }

    println!(
        "Proxied request: {method} {proxy_address} {:?} {:?}",
        path, query_params
    );

    Ok((
        proxy_address,
        "".to_string(),
        path,
        query_params,
        method,
        headers,
    ))
}
