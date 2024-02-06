use std::net::{Ipv4Addr, SocketAddrV4, ToSocketAddrs};

use dotenv::dotenv;
use server_inner::ServerInner;
use tonic::transport::Server;
use log::info;

pub mod service {
    tonic::include_proto!("transcoder");
}

mod server_inner;
mod session;

type BoxError = Box<dyn std::error::Error + Send + Sync>;

#[tokio::main]
async fn main() -> Result<(), BoxError> {
    dotenv().ok();
    env_logger::init();

    let server = ServerInner::new();
    let addr = "0.0.0.0:50051";

    info!("Listening to {}", addr);

    Server::builder()
        .add_service(service::transcoder_server::TranscoderServer::new(server))
        .serve(addr.to_socket_addrs().unwrap().next().unwrap())
        .await?;    
    Ok(())
}
