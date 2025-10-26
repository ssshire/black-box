## Setup
1. Install Docker
2. Run: `docker-compose up -d`
3. Run: `cargo run`

## Opening PostgreSQL shell inside the container
1. Run: `docker-compose exec postgres psql -U chatuser -d chatapp`

## To start CLI 
1. Run: `cargo run --bin tui`
2. Expected: `welcome to blackbox...` 
3. Press any key to exit
