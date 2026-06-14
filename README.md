# blackbox

A terminal chat app built in Rust. Create channels, join them, and send messages — either through the TUI or over a raw TCP connection.

---

## Requirements

- [Rust](https://rustup.rs) — install via `rustup`
- [Docker Desktop](https://www.docker.com/products/docker-desktop) — for the database

---

## Setup

```bash
# 1. clone the repo
git clone https://github.com/your-username/blackbox.git
cd blackbox

# 2. the project lives inside the bb/ folder — run everything from here
cd bb

# 3. rename the SQL file so Docker picks it up
mv black-box.sql init.sql

# 4. start the database
docker-compose up -d

# 5. build
cargo build
```

> All `cargo` and `docker-compose` commands must be run from inside the `bb/` folder. If you see `connection refused` when connecting with `nc`, the most likely cause is running the server from the wrong directory.

---

## How to run

### Terminal UI

```bash
cargo run --bin tui
```

You'll see the main menu. Type a command and press Enter.

```
/create general 50    ← make a channel called "general" with 50 max users
/join general         ← enter the channel
hello world           ← just type to send a message (no slash needed)
/help                 ← list all commands
/quit                 ← exit
```

Press `Escape` to go back to the main menu at any time. Press `Ctrl-C` to exit.

---

### TCP server + manual testing

The TCP server is what you want to test right now — it's the fastest way to verify that messages are actually moving between clients.

**Step 1 — make sure you're in the right directory:**

```bash
cd bb
```

**Step 2 — start the server:**

```bash
cargo run --bin tcp
```

Wait until you see this before doing anything else:

```
blackbox tcp server listening on :8080
```

**Step 3 — open a second terminal, go to the same directory, and connect:**

```bash
cd path/to/blackbox/bb
nc -v localhost 8080
```

You should see:
```
Connection to localhost port 8080 [tcp] succeeded!
welcome to blackbox. type /help for commands.
```

**Step 4 — open a third terminal, connect as a second client:**

```bash
cd path/to/blackbox/bb
nc -v localhost 8080
```

**Step 5 — try sending messages between clients:**

Client A types:
```
/create general 50
/join general
hey, is anyone here?
```

Client B types:
```
/join general
yeah I can see you
```

Client A's terminal should show `[general] user_XXXXX: yeah I can see you` as soon as client B sends it. That's the broadcast working.

When you're done, `Ctrl-C` the server. Both clients will drop.

---

## All commands

```
/create <name> <size>   create a new channel
/join <name>            join an existing channel
/msg <text>             send a message (or just type without a slash)
/help                   show this list
/exit                   disconnect
Ctrl-C                  force quit
```

---

## Run the tests

From inside `bb/`:

```bash
cargo test
```

Expected output:

```
running 3 tests
test command_handler::tests::test_parse_create_command ... ok
test command_handler::tests::test_parse_join_command ... ok
test command_handler::tests::test_parse_list_command ... ok

test result: ok. 3 passed; 0 failed
```

---

## Database (PostgreSQL)

The database runs in Docker. To open a shell and look around:

```bash
docker-compose exec postgres psql -U chatuser -d chatapp
```

```sql
\dt                   -- list tables: users, sessions, messages
SELECT * FROM users;  -- empty for now, auth isn't wired yet
\q                    -- exit
```

---

## Troubleshooting

**`connection refused` when running `nc localhost 8080`**
The server isn't running, or you're in the wrong directory. Make sure you ran `cargo run --bin tcp` from inside `bb/` and that it printed `blackbox tcp server listening on :8080` before you tried to connect.

**`cargo: command not found`**
Rust isn't installed. Run `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh` and follow the prompts.

**`docker-compose up -d` fails**
Docker Desktop isn't running. Open it from your Applications folder and wait for it to fully start before trying again.

---

## What works right now

- TCP server accepts multiple clients simultaneously
- `/create` and `/join` route clients into named channels
- Messages broadcast to everyone in the channel in real time
- TUI renders a main menu, channel view, and help screen
- Command parser is tested and handles bad input gracefully

## To start TCP (debugging process)
1. Run: `cargo run --bin tcp`
2. Expected: `Client/Server connection is created and user can send messages`
3. Press Ctrl-C to kill server

- Wire the TUI into the TCP server so both use the same connection
- Write messages to PostgreSQL on send
- Add login / registration flow