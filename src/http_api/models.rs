use serde::{Deserialize, Serialize};
use uuid::Uuid;
use warp::filters::path::FullPath;
use warp::http::{HeaderMap, Method};
use warp_reverse_proxy::QueryParameters;

#[derive(Deserialize, Debug)]
pub struct RegisterPeerRequestBody {
    pub public_key: String,
    pub preshared_key: Option<String>,
}

#[derive(Serialize, Debug)]
pub struct RegisterPeerResponseBody {
    pub server_public_key: String,
    pub assigned_ip: String,
    pub assigned_id: Uuid,
    /// the address assigned inside docker network to wireguard
    /// since proxy runs in wireguard container's network
    /// a port must be specified also
    pub proxy_address: String,
}

#[derive(Serialize, Debug)]
pub struct PeerInfoResponseBody {
    pub id: Uuid,
    pub internal_ip: String,
    pub public_ip: String,
    pub public_key: String,
    pub proxy_address: String,
}

#[derive(Debug)]
pub struct ApiError {
    pub message: String,
}

impl warp::reject::Reject for ApiError {}

pub type ProxyParams = (String, String, FullPath, QueryParameters, Method, HeaderMap);
