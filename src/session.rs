pub struct Session {
    publish_url: String,
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
}

enum Status {
    Initialized,
    Streaming,
}
