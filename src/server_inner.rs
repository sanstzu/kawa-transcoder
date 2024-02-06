use std::borrow::BorrowMut;
use std::cell::RefCell;
use std::fs::{File, OpenOptions};
use std::sync::Arc;

use log::{info, error};

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
        info!("Receiving connection initialization for publish_url: {}", publish_url);

        let mut sessions = self.sessions.write().await;

        let session = Session::new(publish_url.clone());
        let session_id = sessions.insert(session);

        // Create folder in ./tmp
        if std::fs::create_dir_all(format!("./tmp/{}", publish_url)).is_err() {
            error!("Failed to create folder");
            return Err(Status::internal("Failed to create folder"));
        };

        // Create out folder in ./tmp/{publish_url}
        if std::fs::create_dir_all(format!("./tmp/{}/out", publish_url)).is_err() {
            error!("Failed to create out folder");
            return Err(Status::internal("Failed to create out folder"));
        };

        // Create audio and video name pipe in ./tmp/{publish_url}
        let path = format!("./tmp/{publish_url}/raw.aac");
        if std::process::Command::new("mkfifo")
            .arg(path)
            .status()
            .is_err()
        {
            error!("Failed to craete audio named pipe");
            return Err(Status::internal("Failed to create audio named pipe"));
        };

        let path = format!("./tmp/{publish_url}/raw.h264");
        if std::process::Command::new("mkfifo")
            .arg(path)
            .status()
            .is_err()
        {
            error!("Failed to create video named pipe");
            return Err(Status::internal("Failed to create video named pipe"));
        };

        let mut ffmpeg_command = std::process::Command::new("ffmpeg");
        let audio_arg = format!("./tmp/{publish_url}/raw.aac");
        let video_arg = format!("./tmp/{publish_url}/raw.h264");
        let format_arg = format!("hls \"v.m3u8\"");
        ffmpeg_command.arg("-re").args(["-i", &audio_arg, "-c:a", "aac" ,"-b:a", "128k"]).args(["-i", &video_arg, "-c:v", "libx264", "-b:v", "1M"]).args(["-f", &format_arg]);
        

        match ffmpeg_command.spawn() {
            Ok(child) => {
                info!("Starting ffmpeg process with PID: {}", child.id());
            }
            Err(_) => {
                error!("Failed to start ffmpeg");
                return Err(Status::internal("Failed to start ffmpeg"));
            }
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
        info!("Starting stream session");
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
        info!("Closing session");
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
