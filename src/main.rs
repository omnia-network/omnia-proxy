mod env;

use reqwest::header::FORWARDED;
use std::net::SocketAddr;
use warp::filters::path::FullPath;
use warp::hyper::body::Bytes;
use warp::{
    http::{HeaderMap, Method, Response},
    hyper::Body,
    Filter, Rejection, Reply,
};
use warp_reverse_proxy::{proxy_to_and_forward_response, query_params_filter, QueryParameters};

use env::get_env_var;

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
    let register_to_vpn = warp::path("register-to-vpn").and(warp::addr::remote()).map(
        |remote_address: Option<SocketAddr>| {
            format!(
                "{:} {:}",
                "register to vpn",
                remote_address.unwrap().to_string()
            )
        },
    );

    let target_uri = "https://swapi.dev";
    let proxy = warp::any()
        .and(custom_filter("".to_string(), target_uri.to_string()))
        .and_then(proxy_to_and_forward_response)
        .and_then(log_response);

    let app = warp::any().and(register_to_vpn.or(proxy));

    let port = get_env_var("LISTEN_PORT");

    println!("Listening on port: {}", port);

    // spawn proxy server
    warp::serve(app)
        .run(([0, 0, 0, 0], port.parse::<u16>().unwrap()))
        .await;
}
