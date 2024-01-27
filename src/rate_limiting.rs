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
        let key = format!("req_{}", &ip);
        let res: RedisResult<Vec<Vec<u32>>> = redis::pipe()
            .atomic()
            .cmd("BITFIELD").arg(&key).arg("OVERFLOW").arg("SAT").arg("INCRBY").arg("u32").arg(0).arg(1)
            .cmd("EXPIRE").arg(&key).arg(self.time.as_secs()).arg("NX").ignore()
            .query_async(&mut self.db).await;
        if res.is_err() {
            return true;
        }
        *res.unwrap().first().unwrap().first().unwrap() > self.limit
    }
}