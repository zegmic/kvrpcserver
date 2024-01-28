# JSON RPC server with rate limiting feature for storing key value pairs.

### Prerequisites before running the server
1. Run Redis e.g. `docker run -d redis`
2. set `REDIS_URL` env var to point to Redis instance e.g. `redis://0.0.0.0:6379/`
3. set `RUST_LOG` to logging level e.g. debug
4. 
