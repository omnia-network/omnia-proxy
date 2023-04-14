mod env;
mod http_api;
mod models;
mod proxy;

use std::net::{IpAddr, SocketAddr};
use std::sync::{Arc, Mutex};
use uuid::Uuid;
use warp::filters::path::FullPath;
use warp::reject;
use warp::{
    http::{HeaderMap, Method, Response},
    hyper::Body,
    Filter, Rejection, Reply,
};
use warp_reverse_proxy::{proxy_to_and_forward_response, query_params_filter, QueryParameters};

use env::{get_env_var, load_env_variables};
use http_api::{handlers::handle_register_to_vpn, models::RegisterPeerRequestBody};
use proxy::{proxy_db::ProxyDb, vpn::check_vpn};

async fn log_response(response: Response<Body>) -> Result<impl Reply, Rejection> {
    println!("{:?}", response);
    Ok(response)
}

pub type Request = (
    String,
    String,
    FullPath,
    QueryParameters,
    Method,
    Option<SocketAddr>,
    HeaderMap,
);

/// The `remote_addr` parameter in this case is the address of the peer inside the VPN
/// This function maps the peer's public IP to the peer's VPN IP
fn forward_request(
    proxy_db: Arc<Mutex<ProxyDb>>,
    path: FullPath,
    query_params: QueryParameters,
    method: Method,
    remote_addr: Option<SocketAddr>,
    request_headers: HeaderMap,
) -> (String, String, FullPath, QueryParameters, Method, HeaderMap) {
    let mut proxy_db = proxy_db.lock().unwrap();

    println!("Proxying for remote address: {:?}", remote_addr);

    // here we handle two cases:
    // peer -> backend: the peer is sending a request to the backend, we proxy it attaching the peer's public IP
    // backend -> peer: the backend is requesting the peer inserting peer's public IP in the `X-Forward-To` header

    let mut headers: HeaderMap = request_headers.clone();

    let proxy_address = match headers.get("x-forward-to") {
        Some(peer_id) => {
            println!("Backend -> Peer");
            // backend -> peer
            // have to forward the request to the peer
            let p = peer_id.to_str().unwrap();
            println!("Peer ID: {}", p);

            let peer_internal_ip = match proxy_db.external_mapping.get(&Uuid::try_parse(p).unwrap())
            {
                Some(ip) => ip.to_string(),
                None => "".to_string(),
            };

            println!("Peer internal IP: {}", peer_internal_ip);

            // TODO: change the default port. For not, points to the Gateway WoT Servient port
            format!("http://{peer_internal_ip}:8888/")
        }
        None => {
            println!("Peer -> Backend");
            // peer -> backend
            // we need to add the `Forwarded` header
            // but first we check if the peer is registered
            // if not, we return an empty proxy address

            // just debugging...
            proxy_db.save_db();

            // TODO: handle empty string returns and expect
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
                                    format!("http://{}", proxy_db.get_peer_public_ip(ip_v4))
                                        .to_string()
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

    (
        proxy_address,
        "".to_string(),
        path,
        query_params,
        method,
        headers,
    )
}

#[tokio::main]
async fn main() {
    // load env variables
    assert!(load_env_variables().is_ok(), "Failed to load env variables");

    // check if wireguard is running, otherwise throw
    assert!(check_vpn().is_ok(), "Wireguard is not running");

    let proxy_db = ProxyDb::load_db();
    let shared_proxy_db = Arc::new(Mutex::new(proxy_db));

    let shared_filter = warp::any().map(move || shared_proxy_db.clone());

    let register_to_vpn = warp::post()
        .and(warp::path("register-to-vpn"))
        .and(shared_filter.clone())
        .and(warp::addr::remote())
        .and(warp::body::json::<RegisterPeerRequestBody>())
        .and_then(|shared_proxy_db, remote_address, request_body| async move {
            match handle_register_to_vpn(shared_proxy_db, remote_address, request_body) {
                Ok(res) => Ok(res),
                Err(e) => Err(reject::custom(e)),
            }
        });

    let proxy = warp::any()
        // not sure how this impacts memory, but it should be cloned to avoid locking the mutex
        .and(shared_filter.clone())
        .and(warp::path::full())
        .and(query_params_filter())
        .and(warp::method())
        .and(warp::addr::remote())
        .and(warp::header::headers_cloned())
        .map(forward_request)
        .untuple_one()
        .and(warp::body::bytes())
        .and_then(proxy_to_and_forward_response)
        .and_then(log_response);

    let app = warp::any().and(register_to_vpn.or(proxy));

    let port = 8081;

    println!("Listening on port: {}", port);

    // spawn proxy server
    warp::serve(app).run(([0, 0, 0, 0], port)).await;
}
