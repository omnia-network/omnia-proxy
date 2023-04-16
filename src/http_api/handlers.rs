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

use crate::{env::get_env_var, http_api::models::PeerInfoResponseBody, proxy::proxy_db::ProxyDb};

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
                let peer_id = proxy_db.insert_peer(peer_public_ip, peer_vpn_ip.clone());

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
            match proxy_db.get_peer_internal_ip(Uuid::try_parse(p).unwrap()) {
                Ok(peer_internal_ip) => {
                    println!("Peer internal IP: {}", peer_internal_ip);

                    let forward_to_port = match headers.get("x-forward-to-port") {
                        // TODO: handle unwrap
                        Some(port) => port.to_str().unwrap(),
                        // default to WoT servient default port
                        None => "8888",
                    };

                    // TODO: change the default port. For not, points to the Gateway WoT Servient port
                    format!("http://{peer_internal_ip}:{forward_to_port}/")
                }
                Err(e) => {
                    println!("Peer not registered {:?}", e);
                    "".to_string()
                }
            }
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
                            match proxy_db.get_peer_info(ip_v4) {
                                Ok(peer_info) => {
                                    // peer is registered
                                    // add the `X-Proxied-For` header
                                    headers.insert(
                                        "X-Proxied-For",
                                        peer_info
                                            .public_ip
                                            .parse()
                                            .expect("Failed to parse public IP in header"),
                                    );
                                    // add the peer's ID header
                                    headers.insert(
                                        "X-Peer-Id",
                                        peer_info
                                            .id
                                            .to_string()
                                            .parse()
                                            .expect("Failed to parse peer ID in header"),
                                    );

                                    // read `X-Destination-Url` header, which contains the url to where to forward the request
                                    match headers.get("x-destination-url") {
                                        Some(destination_url) => {
                                            destination_url.to_str().unwrap().to_string()
                                        }
                                        None => {
                                            println!("No destination URL found");
                                            "".to_string()
                                        }
                                    }
                                }
                                Err(e) => {
                                    // peer not registered
                                    println!("Peer not registered {:?}", e);
                                    "".to_string()
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

/// Returns information about the peer. The peer is identified by it's remote address (which should be the internal ip) and retrieved from the database
pub fn handle_peer_info(
    proxy_db: Arc<Mutex<ProxyDb>>,
    remote_address: Option<SocketAddr>,
) -> Result<Json, ApiError> {
    let mut proxy_db = proxy_db.lock().unwrap();

    match remote_address {
        Some(addr) => {
            println!("Retrieving peer information for remote address: {}", addr);
            match addr.ip() {
                IpAddr::V4(ip_v4) => {
                    match proxy_db.get_peer_info(ip_v4) {
                        Ok(peer_info) => {
                            // peer is registered
                            // we have to retrieve the public key from the vpn database
                            match proxy_db.vpn.refresh_and_get_peer(ip_v4) {
                                Ok(vpn_peer) => {
                                    println!("Peer found: {ip_v4}");
                                    let response = PeerInfoResponseBody {
                                        id: peer_info.id,
                                        internal_ip: ip_v4.to_string(),
                                        public_ip: peer_info.public_ip,
                                        proxy_address: get_env_var("PROXY_INTERNAL_ADDRESS"),
                                        public_key: vpn_peer.public_key,
                                    };
                                    Ok(json(&response))
                                }
                                Err(e) => {
                                    println!("Error retrieving peer public key: {}", e);
                                    Err(ApiError {
                                        message: format!("Error retrieving peer public key: {}", e),
                                    })
                                }
                            }
                        }
                        Err(e) => {
                            // peer is not registered, return an error
                            Err(ApiError {
                                message: format!("Peer not registered: {}", e),
                            })
                        }
                    }
                }
                IpAddr::V6(ip_v6) => {
                    println!("Peer is using IPv6: {}", ip_v6);
                    Err(ApiError {
                        message: format!("Peer is using IPv6: {}", ip_v6),
                    })
                }
            }
        }
        None => {
            // this should never happen
            let error = ApiError {
                message: format!("Error retrieving peer information: No remote address"),
            };

            println!("{:?}", error);
            Err(error)
        }
    }
}
