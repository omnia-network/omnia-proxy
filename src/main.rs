mod env;
mod http_api;
mod models;
mod proxy;
mod vpn;

use reqwest::header::FORWARDED;
use warp::reject;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use warp::filters::path::FullPath;
use warp::hyper::body::Bytes;
use warp::{
    http::{HeaderMap, Method, Response},
    hyper::Body,
    Filter, Rejection, Reply,
};
use warp_reverse_proxy::{proxy_to_and_forward_response, query_params_filter, QueryParameters};

use env::{get_env_var, load_env_variables};
use http_api::{handlers::handle_register_to_vpn, models::RegisterPeerRequestBody};
use vpn::vpn::{check_vpn, VPN};

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
    HeaderMap,
    Bytes,
);

fn add_remote_address_to_headers(
    proxy_address: String,
    base_path: String,
    path: FullPath,
    query_params: QueryParameters,
    method: Method,
    remote_addr: Option<SocketAddr>,
    request_headers: HeaderMap,
) -> (String, String, FullPath, QueryParameters, Method, HeaderMap) {
    let mut headers: HeaderMap = request_headers.clone();
    headers.insert(FORWARDED, remote_addr.unwrap().to_string().parse().unwrap());

    (
        proxy_address,
        base_path,
        path,
        query_params,
        method,
        headers,
    )
}

fn custom_filter(
    base_path: String,
    proxy_address: String,
) -> impl Filter<Extract = Request, Error = Rejection> + Clone {
    let proxy_address = warp::any().map(move || proxy_address.clone());
    let base_path = warp::any().map(move || base_path.clone());

    proxy_address
        .and(base_path)
        .and(warp::path::full())
        .and(query_params_filter())
        .and(warp::method())
        .and(warp::addr::remote())
        .and(warp::header::headers_cloned())
        .map(add_remote_address_to_headers)
        .untuple_one()
        .and(warp::body::bytes())
}

#[tokio::main]
async fn main() {
    // load env variables
    assert!(load_env_variables().is_ok(), "Failed to load env variables");

    // check if wireguard is running, otherwise throw
    assert!(check_vpn().is_ok(), "Wireguard is not running");

    let vpn = VPN::new().expect("Failed to initialize VPN");

    let shared_vpn = Arc::new(Mutex::new(vpn));

    println!("Initialized VPN: {:?}", shared_vpn);

    let register_to_vpn = warp::post().and(warp::path("register-to-vpn"))
        .and(warp::any().map(move || shared_vpn.clone()))
        .and(warp::addr::remote())
        .and(warp::body::json::<RegisterPeerRequestBody>())
        .and_then(|shared_vpn, remote_address, request_body| async move {
            match handle_register_to_vpn(shared_vpn, remote_address, request_body) {
                Ok(res) => Ok(res),
                Err(e) => Err(reject::custom(e)),
            }
        });
    // .map(|remote_address, request_body, shared_vpn| {
    //     handle_register_to_vpn(shared_vpn, remote_address, request_body)
    // });

    let target_uri = get_env_var("OMNIA_BACKEND_CANISTER_URL");
    let proxy = warp::any()
        .and(custom_filter("".to_string(), target_uri.to_string()))
        .and_then(proxy_to_and_forward_response)
        .and_then(log_response);

    let app = warp::any().and(register_to_vpn.or(proxy));

    let port = get_env_var("LISTEN_PORT");

    println!("Listening on port: {}", port);

    // spawn proxy server
    warp::serve(app).run(([0, 0, 0, 0], 8080)).await;
}
