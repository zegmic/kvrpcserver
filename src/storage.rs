use anyhow::anyhow;
use redis::aio::MultiplexedConnection;
use redis::AsyncCommands;

#[derive(Clone)]
pub struct KVService {
    db: MultiplexedConnection,
}

impl KVService {
    pub fn new(db: MultiplexedConnection) -> Self {
        Self {
            db
        }
    }

    pub async fn get(&mut self, params: &[String]) -> anyhow::Result<String> {
        let key = params.first().ok_or(anyhow!("No key provided"))?.to_string();
        let val = self.db.get::<String, String>(key).await?;
        Ok(val)
    }

    pub async fn set(&mut self, params: &[String]) -> anyhow::Result<()> {
        let key = params.get(0).ok_or(anyhow!("No key provided"))?.to_string();
        let value = params.get(1).ok_or(anyhow!("No value provided"))?.to_string();
        self.db.set::<String, String, ()>(key, value).await?;
        Ok(())
    }
}