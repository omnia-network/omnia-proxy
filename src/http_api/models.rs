use serde::{Deserialize, Serialize};

#[derive(Deserialize, Debug)]
pub struct RegisterPeerRequestBody {
    pub public_key: String,
    pub preshared_key: Option<String>,
}

#[derive(Serialize)]
pub struct RegisterPeerResponseBody {
    pub server_public_key: String,
    pub assigned_ip: String,
    /// the address assigned inside docker network to wireguard
    /// since proxy runs in wireguard container's network
    /// a port must be specified also
    pub proxy_address: String,
}

#[derive(Debug)]
pub struct ApiError {
    pub message: String,
}

impl warp::reject::Reject for ApiError {}
