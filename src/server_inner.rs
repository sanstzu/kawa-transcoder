use std::borrow::BorrowMut;
use std::cell::RefCell;
use std::sync::Arc;
use tokio::fs::{create_dir_all, remove_dir_all, File, OpenOptions};
use tokio::process;

use log::{error, info, trace};

use slab::Slab;
use tokio::io::AsyncWriteExt;
use tokio::sync::RwLock;

use crate::service::{
    transcoder_server::Transcoder, InitializeSessionRequest, InitializeSessionResponse,
    StreamSessionData, StreamSessionResponse,
};
use crate::service::{CloseSessionRequest, CloseSessionResponse, StreamDataType};
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
        info!(
            "Receiving connection initialization for publish_url: {}",
            publish_url
        );

        let mut sessions = self.sessions.write().await;

        let mut session = Session::new(publish_url.clone());

        let pipe_path = format!("./tmp/pipe/{}", publish_url);
        let out_path = format!("./tmp/out/{}", publish_url);

        // Create folder in ./tmp/pipe/{}
        if create_dir_all(&pipe_path).await.is_err() {
            error!("Failed to create folder");
            return Err(Status::internal("Failed to create folder"));
        };

        // Create out folder in ./tmp/out/{publish_url}
        if create_dir_all(&out_path).await.is_err() {
            error!("Failed to create out folder");
            return Err(Status::internal("Failed to create out folder"));
        };

        // Create audio and video name pipe in ./tmp/{publish_url}
        let video_pipe_path = format!("{pipe_path}/raw.aac");
        if process::Command::new("mkfifo")
            .arg(&video_pipe_path)
            .status()
            .await
            .is_err()
        {
            error!("Failed to create audio named pipe");
            return Err(Status::internal("Failed to create audio named pipe"));
        };

        let audio_pipe_path = format!("{pipe_path}/raw.h264");
        if process::Command::new("mkfifo")
            .arg(&audio_pipe_path)
            .status()
            .await
            .is_err()
        {
            error!("Failed to create video named pipe");
            return Err(Status::internal("Failed to create video named pipe"));
        };

        let mut ffmpeg_command = process::Command::new("ffmpeg");
        let audio_arg = &audio_pipe_path;
        let video_arg = &video_pipe_path;
        ffmpeg_command
            .arg("-re")
            .args(["-thread_queue_size", "1024", "-i", &audio_arg])
            .args(["-thread_queue_size", "1024", "-i", &video_arg])
            .args([
                "-force_key_frames:v",
                "expr:gte(t,n_forced*5)",
                "-f",
                "hls",
                "-hls_time",
                "5",
                "-hls_list_size",
                "10",
                "-hls_segment_filename",
                &format!("{out_path}/file%05d.ts"),
                "-hls_flags",
                "append_list+delete_segments+omit_endlist",
                "-preset",
                "veryfast",
                &format!("{out_path}/out.m3u8"),
            ]);

        match ffmpeg_command.spawn() {
            Ok(child) => {
                let pid = match child.id() {
                    Some(pid) => pid,
                    None => {
                        error!("Failed to get ffmpeg PID");
                        return Err(Status::internal("Failed to get ffmpeg PID"));
                    }
                };
                info!("Starting ffmpeg process with PID: {}", pid);
                session.set_ffmpeg_pid(pid);
            }
            Err(_) => {
                error!("Failed to start ffmpeg");
                return Err(Status::internal("Failed to start ffmpeg"));
            }
        };

        let session_id = sessions.insert(session);
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

        let first_iter = in_stream.next().await;

        let data = match first_iter {
            None => {
                error!("Connection closed before receiving any data");
                return Err(Status::internal(
                    "Connection closed before receiving any data",
                ));
            }
            Some(Ok(data)) => data,
            Some(Err(_)) => {
                error!("Failed to receive first data");
                return Err(Status::internal("Failed to receive first data"));
            }
        };

        let session_tmp = self.sessions.read().await;

        let session = match session_tmp.get(data.session_id as usize) {
            Some(session) => session,
            None => {
                error!("Session not found");
                return Err(Status::internal("Session not found"));
            }
        };

        let publish_url = session.get_publish_url().to_string();

        let (audio_tx, audio_rx) = tokio::sync::mpsc::unbounded_channel::<Vec<u8>>();
        let (video_tx, video_rx) = tokio::sync::mpsc::unbounded_channel::<Vec<u8>>();

        match data.r#type {
            2 => {
                if (&video_tx).send(data.data).is_err() {
                    error!("Failed to send first data");
                }
            }
            1 => {
                if (&audio_tx).send(data.data).is_err() {
                    error!("Failed to send first data");
                };
            }
            _ => {
                error!("Unknown data received");
            }
        };

        let url = publish_url.clone();
        tokio::spawn(async move {
            let mut rx = audio_rx;
            let audio_file = OpenOptions::new()
                .write(true)
                .append(true)
                .open(format!("./tmp/pipe/{}/raw.aac", url))
                .await;

            let mut audio_file = match audio_file {
                Ok(file) => file,
                Err(_) => {
                    return error!("Failed to open audio file");
                }
            };

            trace!("Audio file writer has been started");

            while let Some(data) = rx.recv().await {
                //trace!("Consuming audio");
                if audio_file.write_all(&data).await.is_err() {
                    error!("Failed to write audio data");
                    return;
                }
            }
        });

        let url = publish_url.clone();
        tokio::spawn(async move {
            let mut rx = video_rx;
            let video_file = OpenOptions::new()
                .write(true)
                .append(true)
                .open(format!("./tmp/pipe/{}/raw.h264", url))
                .await;

            let mut video_file = match video_file {
                Ok(file) => file,
                Err(_) => {
                    error!("Failed to open video file");
                    return;
                }
            };

            trace!("Video file writer has been started");

            while let Some(data) = rx.recv().await {
                //trace!("Consuming video");
                if video_file.write_all(&data).await.is_err() {
                    error!("Failed to write video data");
                    return;
                }
            }
        });

        tokio::spawn(async move {
            let mut stream = in_stream;

            let v_tx = video_tx;
            let a_tx = audio_tx;

            while let Some(data) = (&mut stream).next().await {
                match data {
                    Ok(cur_data) => match cur_data.r#type {
                        2 => {
                            if v_tx.send(cur_data.data).is_err() {
                                error!("Failed to write audio data");
                                return;
                            }
                        }
                        1 => {
                            if a_tx.send(cur_data.data).is_err() {
                                error!("Failed to write video data");
                                return;
                            }
                        }
                        _ => {
                            error!("Received data with unknown type: {}", cur_data.r#type);
                        }
                    },
                    Err(_) => {
                        error!("Failed to receive data");
                        return;
                    }
                }
            }
        })
        .await;

        Ok(tonic::Response::new(StreamSessionResponse { status: 0 }))
    }

    async fn close_session(
        &self,
        request: Request<CloseSessionRequest>,
    ) -> Result<Response<CloseSessionResponse>, Status> {
        info!("Closing session");

        let session_id = request.into_inner().session_id;

        let session = self.sessions.write().await.remove(session_id as usize);
        let ffmpeg_pid = match session.get_ffmpeg_pid() {
            Some(pid) => pid,
            None => return Err(Status::internal("Failed to get ffmpeg pid")),
        };

        let mut kill_ffmpeg = process::Command::new("kill");
        kill_ffmpeg.arg(ffmpeg_pid.to_string());

        if kill_ffmpeg.status().await.is_err() {
            return Err(Status::internal("Failed to kill ffmpeg"));
        };

        Ok(tonic::Response::new(CloseSessionResponse { status: 0 }))
    }
}
