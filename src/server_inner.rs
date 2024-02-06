use std::borrow::BorrowMut;
use std::cell::RefCell;
use std::sync::Arc;

use slab::Slab;
use tokio::sync::RwLock;

use crate::service::{
    transcoder_server::Transcoder, InitializeSessionRequest, InitializeSessionResponse,
    StreamSessionData, StreamSessionResponse,
};
use crate::service::{CloseSessionRequest, CloseSessionResponse};
use crate::session::Session;
use tokio_stream::StreamExt;
use tonic::{Request, Response, Status, Streaming};

pub struct ServerInner {
    sessions: Arc<RwLock<Slab<Session>>>,
}

impl ServerInner {
    pub fn new() -> Self {
        ServerInner {
            sessions: Arc::new(RwLock::new(Slab::new())),
        }
    }
}

#[tonic::async_trait]
impl Transcoder for ServerInner {
    async fn initialize_session(
        &self,
        request: Request<InitializeSessionRequest>,
    ) -> Result<Response<InitializeSessionResponse>, Status> {
        let publish_url = request.into_inner().publish_url;

        let mut sessions = self.sessions.write().await;

        let session = Session::new(publish_url.clone());
        let session_id = sessions.insert(session);

        // Create folder in ./tmp
        if std::fs::create_dir_all(format!("./tmp/{}", publish_url)).is_err() {
            return Err(Status::internal("Failed to create folder"));
        };

        // Create out folder in ./tmp/{publish_url}
        if std::fs::create_dir_all(format!("./tmp/{}/out", publish_url)).is_err() {
            return Err(Status::internal("Failed to create out folder"));
        };

        // Create audio and video name pipe in ./tmp/{publish_url}
        if std::process::Command::new("mkfifo ./tmp/{publish_url}/raw.aac")
            .status()
            .is_err()
        {
            return Err(Status::internal("Failed to create audio name pipe"));
        };

        if std::process::Command::new("mkfifo ./tmp/{publish_url}/raw.h264")
            .status()
            .is_err()
        {
            return Err(Status::internal("Failed to create video name pipe"));
        };

        let mut ffmpeg_command = std::process::Command::new("ffmpeg -i ./tmp/{publish_url}/raw.aac -c:a aac -b:a 128k -i ./tmp/{publish_url}/raw.h264 -c:v libx264 -b:v 1M -f hls \"v.m3u8\" & echo $! > ./tmp/{publish_url}/ffmpeg.pid");
        if ffmpeg_command.spawn().is_err() {
            return Err(Status::internal("Failed to start ffmpeg"));
        };

        Ok(tonic::Response::new(InitializeSessionResponse {
            session_id: session_id as u64,
            status: 0,
        }))
    }

    async fn stream_session(
        &self,
        request: Request<Streaming<StreamSessionData>>,
    ) -> Result<Response<StreamSessionResponse>, Status> {
        let mut in_stream = request.into_inner();

        tokio::spawn(async move {
            while let Some(data) = in_stream.next().await {
                match data {
                    Ok(data) => {
                        println!("Received data: {:?}", data.r#type);
                    }
                    Err(e) => {
                        eprintln!("Error receiving stream: {:?}", e);
                        break;
                    }
                }
            }
        });

        Ok(tonic::Response::new(StreamSessionResponse { status: 0 }))
    }

    async fn close_session(
        &self,
        request: Request<CloseSessionRequest>,
    ) -> Result<Response<CloseSessionResponse>, Status> {
        let mut kill_ffmpeg = std::process::Command::new(format!(
            "kill $(cat ./tmp/{}/ffmpeg.pid)",
            request.into_inner().session_id
        ));

        if kill_ffmpeg.status().is_err() {
            return Err(Status::internal("Failed to kill ffmpeg"));
        };

        Ok(tonic::Response::new(CloseSessionResponse { status: 0 }))
    }
}
