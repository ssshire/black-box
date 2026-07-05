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

**You'll land on a Login screen first** — the TUI requires an account before it'll let you into the main menu:

```
Tab          ← move to the next field
Ctrl-R       ← switch between Login and Register
Enter        ← submit (on the last field)
```

First time using it, press `Ctrl-R` to switch to **Register**, fill in username / email / password, and press `Enter`. After that you can just `Login` with the same username/password on future runs — there's no need to register again.

Once you're in, you'll see the main menu. Type a command and press Enter.

```
/create general 50    ← make a channel called "general" with 50 max users
/join general         ← enter the channel
hello world           ← just type to send a message (no slash needed)
/help                 ← list all commands
/quit                 ← exit
```

Press `Escape` to leave the current channel and return to the main menu. Press `Ctrl-C` to force-quit immediately.

> Logging in matters beyond just showing your username in chat: a channel only gets persisted to Postgres if its creator was logged in when they ran `/create`. Anonymous `/create`s (e.g. from `nc`, see below) stay in-memory only — no chat history is saved for them.

---

### TCP server + manual testing (nc)

The server only binds to `127.0.0.1` — it's not reachable from the network, just from this machine. `nc` is a quick way to poke at the server directly or simulate a second client without opening the TUI.

`nc` clients stay anonymous by default — no login screen, just like before. But `/login` and `/register` are server-side commands, not TUI-only, so you can type them over `nc` too (e.g. `/login alice pass123`) if you want an `nc` session's channels/messages to be persisted to Postgres.

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
/register <user> <email> <pass>   create an account
/login <user> <pass>              log in to an existing account
/create <name> <size>             create a new channel
/join <name>                      join an existing channel
/leave                            leave the current channel
/list                             list available channels (outside a channel only)
/msg <text>                       send a message (or just type without a slash)
/help                             show this list
/exit                             disconnect
Ctrl-C                            force quit
```

---

## Run the tests

From inside `bb/`:

```bash
cargo test
```

Expected output:

```
running 7 tests
test command_handler::tests::test_parse_create_command ... ok
test command_handler::tests::test_parse_join_command ... ok
test command_handler::tests::test_parse_leave_command ... ok
test command_handler::tests::test_parse_list_command ... ok
test command_handler::tests::test_parse_login_command ... ok
test command_handler::tests::test_parse_register_command ... ok
test db::tests::hash_password_round_trips_with_argon2_verify ... ok

test result: ok. 7 passed; 0 failed
```

---

## Database (PostgreSQL)

The database runs in Docker. To verify everything is set up correctly (Docker is running, Postgres is healthy, and `init.sql` created the expected tables), run:

```bash
./scripts/check-db.sh
```

### Useful docker-compose commands

```bash
docker-compose up -d        # start Postgres in the background
docker-compose ps           # check it's running and see the port mapping
docker-compose logs -f postgres   # tail Postgres's logs (useful if connections are failing)
docker-compose down          # stop the container, keep the data
docker-compose down -v       # stop the container AND wipe the data volume — full reset
```

To open a shell and look around:

```bash
docker-compose exec postgres psql -U chatuser -d chatapp
```

```sql
\dt                      -- list tables: users, sessions, messages
SELECT * FROM users;     -- registered accounts (password_hash is argon2, never plaintext)
SELECT * FROM sessions;  -- channels created by a logged-in user
SELECT * FROM messages;  -- messages sent into those channels
\q                       -- exit
```

To clear out test data without dropping the schema:

```bash
docker-compose exec postgres psql -U chatuser -d chatapp -c "delete from messages; delete from sessions; delete from users;"
```

> **Restart the TCP server after wiping `sessions` or `users`.** The server caches each channel's `session_id` in memory from the moment it's `/create`d, for as long as the process runs. If you delete `sessions` (or `users`, via the cascade) while the server is still up, any channel created before the wipe is left holding a `session_id` that no longer exists — every message sent into it will then fail with `violates foreign key constraint "messages_session_id_fkey"` until you `Ctrl-C` and restart `cargo run --bin tcp` (which clears the in-memory channel map) and `/create` the channel again. If you only need to clear chat history and want existing channels to keep working, delete from `messages` alone and leave `sessions`/`users` untouched.

> **Port 5432 conflict:** if you also have a native (non-Docker) Postgres running — e.g. via Homebrew (`brew services list | grep postgres`) — it can bind `127.0.0.1:5432` ahead of Docker's container, silently swallowing every connection meant for the container. `cargo run --bin tcp` would then fail with `role "chatuser" does not exist` even though `check-db.sh` passes (that script runs `psql` *inside* the container, bypassing the host port, so it can't detect this). Fix with `brew services stop postgresql@14` (or whichever version is running), then confirm with `lsof -nP -iTCP:5432 -sTCP:LISTEN` that only Docker is listed.

---

## Troubleshooting

**`connection refused` when running `nc localhost 8080`**
The server isn't running, or you're in the wrong directory. Make sure you ran `cargo run --bin tcp` from inside `bb/` and that it printed `blackbox tcp server listening on 127.0.0.1:8080` before you tried to connect.

**`cargo: command not found`**
Rust isn't installed. Run `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh` and follow the prompts.

**`docker-compose up -d` fails**
Docker Desktop isn't running. Open it from your Applications folder and wait for it to fully start before trying again.

**`role "chatuser" does not exist` when running `cargo run --bin tcp`**
Something other than Docker's Postgres is listening on port 5432 — see the port-conflict note in the [Database](#database-postgresql) section above.

**`error: username already taken` / `error: invalid username or password`**
These are normal `/register`/`/login` rejections, not bugs — pick a different username, or double check the password if logging in.

---

## What works right now

- TCP server accepts multiple clients simultaneously, bound to localhost only
- `/create` and `/join` route clients into named channels
- Messages broadcast to everyone in the channel in real time
- TUI is a real client of the TCP server — same connection, same protocol as `nc`
- Command parser is tested and handles bad input gracefully
- `/register` and `/login` create/authenticate real users in Postgres (argon2-hashed passwords); the TUI requires logging in before reaching the main menu, while raw `nc` clients stay anonymous
- Channels created by a logged-in user get a real `session_id` in Postgres, and messages sent into those channels by logged-in users are persisted; anonymous-created channels (e.g. from `nc`) stay in-memory only, matching the previous behavior
- TUI properly detects terminal resizes (`check_for_resize`) so the channel view's scroll position stays correct when the window changes size

## What's next

In priority order:

1. Add a `/logout` command — there's currently no way to de-authenticate without quitting the whole client. Needs a `Command::Logout` variant, a server-side arm that resets the connection's `authed_user` back to `None`, and a TUI path back to the Login screen from the main menu (today `AppScreen::Login` is only ever reached at startup).
2. Reload existing `sessions` from Postgres when the TCP server starts, instead of always booting with an empty in-memory channel map. Right now persistence is effectively write-only — messages and sessions land in Postgres, but a server restart wipes every channel from memory, so nobody can `/join` a channel that existed five minutes ago even though its history is sitting in the database.
3. Backfill a channel's chat history from Postgres when a client `/join`s it, instead of starting every join from a blank scrollback — only matters for channels that have a `session_id` (i.e. were created by a logged-in user). Depends on #2: without it, history backfill only helps within a single server run, not across restarts.
4. Add clear form validation to the TUI's Login/Register screen — right now it only checks that fields aren't empty before submitting (`"all fields are required"`), with no client-side checks on email format, minimum password length, or allowed username characters. Bad input currently round-trips to the server and comes back as a generic `error: ...` line instead of pointing at the specific field that's wrong. Independent of the rest — can be picked up any time.

**Tabled for future tickets** (not scoped yet, no priority assigned):

- Revamp the TUI's overall look and feel

