use std::time::SystemTime;

// Simple rate limiter using minute long windows
// and recording the number of
#[derive(Debug)]
pub struct RateLimit {
    pub current_window: u64,
    pub requests: u64,
}

fn get_time_bucket() -> u64 {
    let current_time = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("Error in getting time bucket")
        .as_secs();
    current_time - (current_time % 60)
}

impl RateLimit {
    pub fn new() -> Self {
        Self {
            current_window: get_time_bucket(),
            requests: 0,
        }
    }

    pub fn update_request_count(&mut self) {
        let current_window = get_time_bucket();
        if current_window > self.current_window {
            self.current_window = current_window;
            self.requests = 0;
        }

        self.requests += 1;
    }
}
