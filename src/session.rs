pub struct Session {
    publish_url: String,
    status: Status,
}

impl Session {
    pub fn new(publish_url: String) -> Self {
        Session {
            publish_url,
            status: Status::Initialized,
        }
    }
}

enum Status {
    Initialized,
    Streaming,
}
