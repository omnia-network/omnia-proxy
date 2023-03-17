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
}

#[derive(Debug)]
pub struct ApiError {
  pub message: String,
}

impl warp::reject::Reject for ApiError {}
