use std::net::{Ipv4Addr, SocketAddrV4, ToSocketAddrs};

use dotenv::dotenv;
use server_inner::ServerInner;
use tonic::transport::Server;

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

    Server::builder()
        .add_service(service::transcoder_server::TranscoderServer::new(server))
        .serve("[::1]:50051".to_socket_addrs().unwrap().next().unwrap())
        .await?;
    Ok(())
}
