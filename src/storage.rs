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

    pub async fn get(&mut self, params: &Vec<String>) -> Result<String, String> {
        let val = self.db.get::<String, String>(params.get(0).unwrap().to_string())
            .await
            .unwrap();
        Ok(val)
    }

    pub async fn set(&mut self, params: &Vec<String>) -> Result<String, String> {
        self.db.set::<String, String, ()>(params.get(0).unwrap().to_string(), params.get(1).unwrap().to_string())
            .await
            .unwrap();
        Ok("ok".to_string())
    }
}