use std::time::Duration;
use log::error;
use redis::{RedisResult, aio::ConnectionManager};
use tokio::sync::{mpsc, oneshot};
use tokio::sync::mpsc::Sender;

#[derive(Clone)]
pub struct Service {
    time: Duration,
    limit: u32,
    db: ConnectionManager,
}

pub enum Command {
    LimitReached {
        ip: String,
        res: oneshot::Sender<bool>,
    }
}

impl Service {
    pub fn new(time: Duration, limit: u32, db: ConnectionManager) -> Self {
        Service {
            time,
            limit,
            db,
        }
    }

    pub fn run(mut self) -> Sender<Command> {
        let (tx, mut rx) = mpsc::channel::<Command>(256);

        tokio::spawn(async move {
            while let Some(cmd) = rx.recv().await {
                match cmd {
                    Command::LimitReached { ip, res } => {
                        let limit_reached = self.limit_reached(ip.as_str()).await;
                        res.send(limit_reached).unwrap();
                    }
                }
            }
        });

        tx
    }

    pub async fn limit_reached(&mut self, ip: &str) -> bool {
        let key = format!("req_{}", &ip);
        let res: RedisResult<Vec<Vec<u32>>> = redis::pipe()
            .atomic()
            .cmd("BITFIELD").arg(&key).arg("OVERFLOW").arg("SAT").arg("INCRBY").arg("u32").arg(0).arg(1)
            .cmd("EXPIRE").arg(&key).arg(self.time.as_secs()).arg("NX").ignore()
            .query_async(&mut self.db).await;
        if res.is_err() {
            error!("Couldn't access Redis: {}", res.err().unwrap());
            return true;
        }
        *res.unwrap().first().unwrap().first().unwrap() > self.limit
    }
}