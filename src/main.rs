use std::net::{IpAddr, SocketAddr};
use structopt::StructOpt;
use tokio::net::TcpListener;

use p2p::node::Node;

#[derive(Debug, StructOpt)]
struct Opt {
    #[structopt(long)]
    period: u32,
    #[structopt(long, default_value = "127.0.0.1")]
    bind_addr: IpAddr,
    #[structopt(long, default_value = "8000")]
    port: u16,
    #[structopt(long)]
    connect: Option<SocketAddr>,
}

#[tokio::main]
async fn main() {
    env_logger::Builder::new()
        .filter_level(log::LevelFilter::Info)
        .parse_default_env()
        .init();
    let opt = Opt::from_args();
    let bind = SocketAddr::new(opt.bind_addr, opt.port);
    log::info!("Binding to {}", bind);
    let node = Node::new(bind);
    if let Some(peer) = opt.connect {
        node.connect_checked(peer)
            .await
            .expect("Failed to connect to given peer");
        log::info!("Connected to {}", peer);
    }
    let listener = TcpListener::bind(&bind).await.expect("Bind failed");
    tokio::join!(
        p2p::server(listener, node.clone()),
        p2p::message_sender(&node, opt.period)
    );
}
