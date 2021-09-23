use std::net::SocketAddr;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum Payload {
    Message([u8; 32], String),
    Response(Option<Vec<SocketAddr>>),
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ProtocolMessage {
    pub peer_id: SocketAddr,
    pub payload: Payload,
}
