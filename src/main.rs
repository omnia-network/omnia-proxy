mod env;
mod http_api;
mod models;
mod proxy;

use reqwest::header::FORWARDED;
use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
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
use proxy::{proxy_db::ProxyDB, vpn::check_vpn};

async fn log_response(response: Response<Body>) -> Result<impl Reply, Rejection> {
    println!("{:?}", response);
    Ok(response)
}

// async fn log_request(
//     request: Request,
// ) -> Result<Request, Rejection> {
//     println!("{:?}", request);
//     Ok(request)
// }

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
fn add_remote_address_to_headers(
    proxy_db: Arc<Mutex<ProxyDB>>,
    proxy_address: String,
    base_path: String,
    path: FullPath,
    query_params: QueryParameters,
    method: Method,
    remote_addr: Option<SocketAddr>,
    request_headers: HeaderMap,
) -> (String, String, FullPath, QueryParameters, Method, HeaderMap) {
    let proxy_db = proxy_db.lock().unwrap();

    let remote_ip = match proxy_db
        .internal_mapping
        .get(&remote_addr.unwrap().ip().to_string())
    {
        Some(ip) => ip.parse().unwrap(),
        None => String::new(),
    };

    println!("Proxying for remote IP: {}", remote_ip);

    let mut headers: HeaderMap = request_headers.clone();
    headers.insert(FORWARDED, remote_ip.parse().unwrap());

    (
        proxy_address,
        base_path,
        path,
        query_params,
        method,
        headers,
    )
}

fn custom_filter() -> impl Filter<Extract = Request, Error = Infallible> + Clone {
    let proxy_address = warp::any().map(move || {
        get_env_var("OMNIA_BACKEND_CANISTER_URL")
            .to_string()
            .clone()
    });
    let base_path = warp::any().map(move || "".to_string().clone());

    proxy_address
        .and(base_path)
        .and(warp::path::full())
        .and(query_params_filter())
        .and(warp::method())
        .and(warp::addr::remote())
        .and(warp::header::headers_cloned())
}

#[tokio::main]
async fn main() {
    // load env variables
    assert!(load_env_variables().is_ok(), "Failed to load env variables");

    // check if wireguard is running, otherwise throw
    assert!(check_vpn().is_ok(), "Wireguard is not running");

    let proxy_db = ProxyDB::new();
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
        .and(shared_filter)
        .and(custom_filter())
        .map(add_remote_address_to_headers)
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
