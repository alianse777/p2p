#![allow(clippy::type_complexity)]

use anyhow::Result;
use async_bincode::{AsyncBincodeStream, AsyncDestination};
use sha2::{Digest, Sha256};
use std::{collections::HashMap, future::Future, net::SocketAddr, pin::Pin, sync::Arc, task::Poll};
use tokio::{net::TcpStream, sync::RwLock};
use tower::Service;

use crate::protocol::{Payload, ProtocolMessage};

type BincodeStream =
    AsyncBincodeStream<TcpStream, ProtocolMessage, ProtocolMessage, AsyncDestination>;

type PeerTable = HashMap<SocketAddr, BincodeStream>;

pub fn hash_for(peer_table: &PeerTable, addr: &SocketAddr) -> [u8; 32] {
    let hash = peer_table
        .keys()
        .filter(|&x| x != addr)
        .fold(Sha256::new(), |hasher, val| {
            hasher.chain(val.to_string().as_bytes())
        })
        .finalize();
    hash.into()
}

#[derive(Clone, Debug)]
pub struct Node {
    peer_id: SocketAddr,
    peer_table: Arc<RwLock<HashMap<SocketAddr, BincodeStream>>>,
}

impl Node {
    pub fn new(peer_id: SocketAddr) -> Self {
        Self {
            peer_id,
            peer_table: Arc::new(RwLock::new(PeerTable::new())),
        }
    }

    pub fn peer_table(&self) -> &RwLock<PeerTable> {
        &self.peer_table
    }

    pub fn peer_id(&self) -> &SocketAddr {
        &self.peer_id
    }

    /// Connects to peer if not already connected
    pub async fn connect_checked(&self, addr: SocketAddr) -> Result<bool> {
        if !self.peer_table.read().await.contains_key(&addr) && addr != self.peer_id {
            let socket = TcpStream::connect(addr).await.unwrap();
            let conn = AsyncBincodeStream::<_, ProtocolMessage, ProtocolMessage, _>::from(socket)
                .for_async();
            self.peer_table.write().await.insert(addr, conn);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn connection_handler(&self) -> ConnectionHandler {
        ConnectionHandler { node: self.clone() }
    }
}
pub struct ConnectionHandler {
    node: Node,
}

impl<'a> Service<ProtocolMessage> for ConnectionHandler {
    type Response = ProtocolMessage;
    type Error = std::io::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(
        &mut self,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: ProtocolMessage) -> Self::Future {
        let peer_id = self.node.peer_id().to_owned();
        let node_handle = self.node.clone();
        Box::pin(async move {
            match node_handle.connect_checked(req.peer_id).await {
                Ok(true) => log::info!("Connected to {}", req.peer_id),
                Err(e) => log::warn!("Failed to connect to foreign peer: {:?}", e),
                _ => (),
            }
            let payload = match req.payload {
                Payload::Message(peer_hash, msg) => {
                    log::info!("Received message [{}] from {}", msg, req.peer_id);
                    let peer_table = node_handle.peer_table().read().await;
                    let current_hash = hash_for(&peer_table, &req.peer_id);
                    if peer_hash == current_hash {
                        Payload::Response(None)
                    } else {
                        Payload::Response(Some(peer_table.keys().map(|x| x.to_owned()).collect()))
                    }
                }
                _ => unimplemented!(),
            };
            Ok(ProtocolMessage { peer_id, payload })
        })
    }
}
