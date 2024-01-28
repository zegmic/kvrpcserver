use log::{debug, error};
use redis::{aio::ConnectionManager, AsyncCommands};
use tokio::sync::{mpsc, oneshot};
use tokio::sync::mpsc::Sender;

#[derive(Clone)]
pub struct KVService {
    db: ConnectionManager,
}

#[derive(Debug)]
pub enum Command {
    Get { key: String, res: oneshot::Sender<anyhow::Result<String>> },
    Set { key: String, value: String, res: oneshot::Sender<anyhow::Result<()>> }
}

impl KVService {
    pub fn new(db: ConnectionManager) -> Self {
        Self {
            db
        }
    }

    pub fn run(mut self) -> Sender<Command> {
        let (tx, mut rx) = mpsc::channel::<Command>(256);

        tokio::spawn(async move {
            while let Some(cmd) = rx.recv().await {
                debug!("Processing storage command {:?}", cmd);
                match cmd {
                    Command::Get { key, res} => {
                        let db_result = self.get(key).await;
                        if let Err(err) = &db_result {
                            error!("Error getting the value from storage service: {}", err);
                        }
                        res.send(db_result).unwrap();
                    },
                    Command::Set { key, value, res} => {
                        let db_result = self.set(key, value).await;
                        if let Err(err) = &db_result {
                            error!("Error setting the value in storage service: {}", err);
                        }
                        res.send(db_result).unwrap();
                    }
                }
            }
        });

        tx
    }

    pub async fn get(&mut self, key: String) -> anyhow::Result<String> {
        let val = self.db.get::<String, String>(key).await?;
        Ok(val)
    }

    pub async fn set(&mut self, key: String, value: String) -> anyhow::Result<()> {
        self.db.set::<String, String, ()>(key, value).await?;
        Ok(())
    }
}