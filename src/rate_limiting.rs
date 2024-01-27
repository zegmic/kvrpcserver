use std::time::Duration;
use redis::aio::MultiplexedConnection;
use redis::RedisResult;

#[derive(Clone)]
pub struct Service {
    time: Duration,
    limit: u32,
    db: MultiplexedConnection,
}

impl Service {
    pub fn new(time: Duration, limit: u32, db: MultiplexedConnection) -> Self {
        Service {
            time,
            limit,
            db
        }
    }

    pub async fn limit_reached(&mut self, ip: &str) -> bool {
        // let res: RedisResult<String> = redis::pipe()
        //     .hget("", ip)
        //     .query_async(&mut self.db).await;
        false
    }
}