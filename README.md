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

# 3. start the database
docker-compose up -d

# 4. build
cargo build
```

> All `cargo` and `docker-compose` commands must be run from inside the `bb/` folder. If you see `connection refused` when connecting with `nc`, the most likely cause is running the server from the wrong directory.

---

## How to run

### Terminal UI

The TUI is a real client of the TCP server — start the server first:

```bash
cargo run --bin tcp
```

Then, in a second terminal:

```bash
cargo run --bin tui
```

If the server isn't running yet, the TUI prints an error and exits rather than opening a blank screen.

You'll see the main menu. Type a command and press Enter.

```
/create general 50    ← make a channel called "general" with 50 max users
/join general         ← enter the channel
hello world           ← just type to send a message (no slash needed)
/help                 ← list all commands
/quit                 ← exit
```

Press `Escape` to leave the current channel and return to the main menu. Press `Ctrl-C` to force-quit immediately.

---

### TCP server + manual testing (nc)

The server only binds to `127.0.0.1` — it's not reachable from the network, just from this machine. `nc` is a quick way to poke at the server directly or simulate a second client without opening the TUI.

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
blackbox tcp server listening on 127.0.0.1:8080
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
/leave                  leave the current channel
/list                   list available channels (outside a channel only)
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

The database runs in Docker. To verify everything is set up correctly (Docker is running, Postgres is healthy, and `init.sql` created the expected tables), run:

```bash
./scripts/check-db.sh
```

To open a shell and look around:

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
The server isn't running, or you're in the wrong directory. Make sure you ran `cargo run --bin tcp` from inside `bb/` and that it printed `blackbox tcp server listening on 127.0.0.1:8080` before you tried to connect.

**`cargo: command not found`**
Rust isn't installed. Run `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh` and follow the prompts.

**`docker-compose up -d` fails**
Docker Desktop isn't running. Open it from your Applications folder and wait for it to fully start before trying again.

---

## What works right now

- TCP server accepts multiple clients simultaneously, bound to localhost only
- `/create` and `/join` route clients into named channels
- Messages broadcast to everyone in the channel in real time
- TUI is a real client of the TCP server — same connection, same protocol as `nc`
- Command parser is tested and handles bad input gracefully

## What's next

- Write messages to PostgreSQL on send
- Add login / registration flow
- Handle terminal resizing in the channel view — re-clamp scroll position when the viewport grows/shrinks so the scrollback doesn't jump or clip when the window changes size mid-scroll

