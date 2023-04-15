mod env;
mod http_api;
mod models;
mod proxy;

use std::sync::{Arc, Mutex};
use warp::reject;
use warp::{http::Response, hyper::Body, Filter, Rejection, Reply};
use warp_reverse_proxy::{proxy_to_and_forward_response, query_params_filter};

use env::load_env_variables;
use http_api::{
    handlers::{forward_request, handle_peer_info, handle_register_to_vpn},
    models::RegisterPeerRequestBody,
};
use proxy::{proxy_db::ProxyDb, vpn::check_vpn};

async fn log_response(response: Response<Body>) -> Result<impl Reply, Rejection> {
    println!("{:?}", response);
    Ok(response)
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
                // TODO: handle errors and return appropriate status codes, see https://docs.rs/warp/0.3.3/warp/reject/index.html#example
                Err(e) => Err(reject::custom(e)),
            }
        });

    let peer_info = warp::get()
        .and(warp::path("peer-info"))
        .and(shared_filter.clone())
        .and(warp::addr::remote())
        .and_then(|shared_proxy_db, remote_address| async move {
            match handle_peer_info(shared_proxy_db, remote_address) {
                Ok(res) => Ok(res),
                // TODO: handle errors and return appropriate status codes, see https://docs.rs/warp/0.3.3/warp/reject/index.html#example
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
        // TODO: improve this handler, we don't want to write every time the variables
        .and_then(
            |shared_proxy_db, path, query_params, method, remote_address, headers| async move {
                match forward_request(
                    shared_proxy_db,
                    path,
                    query_params,
                    method,
                    remote_address,
                    headers,
                ) {
                    Ok(res) => Ok(res),
                    Err(e) => Err(reject::custom(e)),
                }
            },
        )
        .untuple_one()
        .and(warp::body::bytes())
        .and_then(proxy_to_and_forward_response)
        .and_then(log_response);

    let app = warp::any().and(register_to_vpn.or(peer_info).or(proxy));

    let port = 8081;

    println!("Listening on port: {}", port);

    // spawn proxy server
    warp::serve(app).run(([0, 0, 0, 0], port)).await;
}

// TODO: write tests
