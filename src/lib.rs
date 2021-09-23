pub mod node;
pub mod protocol;
use anyhow::Result;
use async_bincode::AsyncBincodeStream;
use futures::{sink::SinkExt, stream::StreamExt};
use node::hash_for;
use protocol::Payload;
use rand::{thread_rng, Rng};
use std::{collections::HashSet, net::SocketAddr, time::Duration};
use tokio::net::{TcpListener, TcpStream};
use tokio_tower::multiplex::Server;

use crate::{node::Node, protocol::ProtocolMessage};

pub async fn handle_peer_connection(stream: TcpStream, peer: SocketAddr, node: Node) -> Result<()> {
    let item_stream =
        AsyncBincodeStream::<_, ProtocolMessage, ProtocolMessage, _>::from(stream).for_async();
    tokio::spawn(async move {
        if let Err(e) = Server::new(item_stream, node.connection_handler()).await {
            log::warn!("{}: {} (Disconnected)", peer, e);
        }
    });
    Ok(())
}

pub async fn server(listener: TcpListener, node: Node) {
    loop {
        match listener.accept().await {
            Ok((stream, peer)) => {
                if let Err(e) = handle_peer_connection(stream, peer, node.clone()).await {
                    log::error!("Failed to handle peer {}: {}", peer, e);
                }
            }
            Err(e) => log::error!("Failed to accept connection: {:?}", e),
        }
    }
}

pub async fn message_sender(node: &Node, period: u32) {
    loop {
        let msg = format!("message {}", thread_rng().gen_range(0..100000000));
        let all_peers_addr: Vec<_> = {
            let peer_table_lock = node.peer_table().read().await;
            peer_table_lock
                .iter()
                .map(|(addr, _)| addr.to_owned())
                .collect()
        };
        if !all_peers_addr.is_empty() {
            log::info!("Sending message [{}] to {:?}", msg, all_peers_addr);
            let mut new_peers = HashSet::new();
            for addr in all_peers_addr {
                let mut table_write_lock = node.peer_table().write().await;
                let hash = hash_for(&table_write_lock, &addr);
                let payload = ProtocolMessage {
                    peer_id: node.peer_id().to_owned(),
                    payload: Payload::Message(hash, msg.clone()),
                };
                let conn = table_write_lock.get_mut(&addr).unwrap();
                match conn.send(payload.clone()).await {
                    Ok(()) => {
                        if let Some(Ok(ProtocolMessage {
                            payload: Payload::Response(Some(peer_table)),
                            ..
                        })) = conn.next().await
                        {
                            log::debug!("Updating peer table from {}", addr);
                            for addr in peer_table {
                                new_peers.insert(addr);
                            }
                        }
                    }
                    Err(e) => {
                        log::warn!("Failed to send to {}: {}", addr, e);
                        table_write_lock.remove(&addr);
                    }
                }
            }
            for peer in new_peers {
                match node.connect_checked(peer).await {
                    Ok(true) => log::info!("Connected to {}", peer),
                    Err(e) => log::warn!("Failed to connect to received peer {}: {}", peer, e),
                    _ => (),
                }
            }
        }
        tokio::time::sleep(Duration::from_secs(period as _)).await;
    }
}
