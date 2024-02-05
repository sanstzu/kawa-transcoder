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

        let session = Session::new(publish_url);
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
        unimplemented!()
    }
}
