use std::time::Duration;

#[derive(Clone)]
pub struct Service {
    time: Duration,
    limit: u32,
}

impl Service {
    pub fn new(time: Duration, limit: u32) -> Self {
        Service {
            time,
            limit,
        }
    }

    pub fn limit_reached(&self, ip: &str) -> bool {
        false
    }
}