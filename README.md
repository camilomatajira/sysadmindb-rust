# SysadminDB Rust

A Rust rewrite of sysadmindb (originally Python). A lightweight log server that receives syslog messages over TCP, stores them in SQLite, and lets you query them over HTTP — piping results through standard Unix tools like `awk`, `grep`, `sed`, and `jq`.

**This tool is not ready for production use.**

**This tool was made YOLO to learn Tokio and Axum, I didn't write any tests.**

## Architecture

- **TCP server** (port 1999) — receives syslog messages, parses them, and inserts them into the database
- **HTTP server** (port 3000) — exposes endpoints to query stored logs
- **SQLite** — stores parsed log fields

## Security

The HTTP server executes shell commands supplied by the client, but restricts execution to a set of allowed binaries via `bash --restricted` and a controlled `PATH` pointing to `restricted_bin/`. Only commands symlinked into that directory can be run.

Allowed commands: `awk`, `cut`, `grep`, `jq`, `nl`, `sed`, `sort`, `uniq`, `wc`.

## Requirements

- Rust 1.86+
- `sqlx-cli` with SQLite support:

```bash
cargo install sqlx-cli --no-default-features --features sqlite
```

## Setup

1. Run migrations:

```bash
make init_db
```

2. Start the server:

```bash
make run
```

## Sending logs

Send syslog-formatted messages over TCP to port 1999:

```bash
echo '<34>1 2026-05-26T10:00:00.000Z myhost myapp 1234 - - Hello world' | nc localhost 1999
```

## Querying logs

All queries use `POST /` with a JSON body containing a `command` field. The server fetches the raw log lines from the database, pipes them through the given command, and returns the output as plain text.

**Get all logs:**
```bash
curl -X POST -H 'Content-Type: application/json' \
  --data '{"command": "sort"}' \
  localhost:3000/
```

**Filter by timestamp, hostname, or appname via query params:**
```bash
curl -X POST -H 'Content-Type: application/json' \
  --data '{"command": "sort"}' \
  "localhost:3000/?date_gt=2026-05-26T00:00:00Z"

curl -X POST -H 'Content-Type: application/json' \
  --data '{"command": "sort"}' \
  "localhost:3000/?hostname=myhost&appname=myapp"
```

**Pipe results through Unix tools:**
```bash
# Count log lines
curl -X POST -H 'Content-Type: application/json' \
  --data '{"command": "wc -l"}' \
  localhost:3000/

# Filter with grep
curl -X POST -H 'Content-Type: application/json' \
  --data '{"command": "grep error"}' \
  localhost:3000/

# Extract a field with awk
curl -X POST -H 'Content-Type: application/json' \
  --data '{"command": "awk \"{print $5}\""}' \
  localhost:3000/
```
this works with bash
curl -H 'Content-Type: application/json' --data '{"command": "awk \"{print \\$2}\""}' localhost:3000/                                                


## Log fields

| Field | Type | Description |
|---|---|---|
| `original_msg` | text | Raw syslog line |
| `prival` | integer | Priority value |
| `version` | integer | Syslog version |
| `date` | text | Log timestamp from message |
| `hostname` | text | Source hostname |
| `appname` | text | Application name |
| `procid` | text | Process ID |
| `msgid` | text | Message ID |
| `structureddata` | text | Structured data field |
| `msg` | text | Log message body |
| `timestamp` | text | Ingestion timestamp (RFC 3339) |
