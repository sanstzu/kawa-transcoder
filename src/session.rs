pub struct Session {
    publish_url: String,
    #[allow(dead_code)]
    status: Status,
    ffmpeg_pid: Option<u32>,
}

impl Session {
    pub fn new(publish_url: String) -> Self {
        Session {
            publish_url,
            status: Status::Initialized,
            ffmpeg_pid: None,
        }
    }

    pub fn get_ffmpeg_pid(&self) -> Option<u32> {
        self.ffmpeg_pid
    }

    pub fn set_ffmpeg_pid(&mut self, pid: u32) {
        self.ffmpeg_pid = Some(pid);
    }

    pub fn get_publish_url(&self) -> &str {
        &self.publish_url
    }
}

#[allow(dead_code)]
enum Status {
    Initialized,
    Streaming,
}
