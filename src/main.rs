use std::{
    env,
    net::{Ipv4Addr, SocketAddrV4, ToSocketAddrs},
};

use dotenv::dotenv;
use log::{error, info};
use server_inner::ServerInner;
use tokio::{
    fs::create_dir_all,
    process::Command,
    time::{sleep, Duration},
};
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
    let addr = "0.0.0.0:50051";

    info!("Listening to {}", addr);

    // Run AWS-CLI updater
    create_dir_all("./tmp/out").await?;
    let mut cmd = Command::new("aws");
    let aws_s3_output_uri = env::var("AWS_S3_OUTPUT_URI")?;
    cmd.args([
        "s3",
        "sync",
        "./tmp/out",
        &aws_s3_output_uri,
        "--exclude",
        "*.tmp",
        "--delete",
    ]);
    tokio::spawn(async move {
        let mut cmd = cmd;
        loop {
            if cmd.status().await.is_err() {
                error!("AWS S3 Sync failed");
                break;
            }
            sleep(Duration::from_millis(500)).await;
        }
    });

    Server::builder()
        .add_service(service::transcoder_server::TranscoderServer::new(server))
        .serve(addr.to_socket_addrs().unwrap().next().unwrap())
        .await?;
    Ok(())
}
