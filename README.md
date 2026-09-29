# 🪷 Lotus Connect System

Modular backend service for the Lotus Connect real-time chat, feed, audio/video calling, and media storage platform built with **Rust**, **Axum**, and **Podman**.

---

## 🚀 Quick Start

### 1. Ensure Podman is in your PATH
If installed via Podman Desktop / macOS installer, add the following to your `~/.zshrc` (if not already present):
```bash
export PATH="/opt/podman/bin:$PATH"
alias docker=podman
```
Then reload your shell:
```bash
source ~/.zshrc
```

### 2. Environment Configuration
Ensure your local `.env` exists:
```bash
cp -n .env.example .env
```

### 3. Start Infrastructure Services (Podman)
Start PostgreSQL, Redis, and MinIO in the background:
```bash
make up
# or: podman compose -f compose.yaml up -d
```
Check status:
```bash
make ps
# or: podman compose -f compose.yaml ps
```

### 4. Run the Backend Application
```bash
make run
# or: cargo run
```
The server will start listening on **`http://0.0.0.0:8080`**. Database migrations will apply automatically upon startup.

---

## 🛠️ Makefile Commands

| Command | Action |
| :--- | :--- |
| `make up` | Start PostgreSQL, Redis, and MinIO containers via Podman |
| `make down` | Stop all running containers |
| `make restart` | Restart all containers |
| `make ps` | View container health and exposed ports |
| `make logs` | Follow live container logs |
| `make run` | Compile and run the Lotus Connect backend server |
| `make test` | Run the complete test suite |
| `make clean` | Stop containers and erase volume data |

---

## 📦 Service Architecture & Ports

| Service | Internal Port | Host Port | Credentials / Notes |
| :--- | :--- | :--- | :--- |
| **Lotus Connect Gateway** | `8080` | `8080` | REST API (`/api/v1`) & WebSocket (`/ws`) |
| **PostgreSQL 16** | `5432` | `5432` | `user=postgres`, `password=postgres`, `db=lotus_connect` |
| **Redis 7** | `6379` | `6379` | Presence cache & Pub/Sub broker |
| **MinIO S3 API** | `9000` | `9000` | `minioadmin` / `minioadmin` |
| **MinIO Web Console** | `9001` | `9001` | Web UI at [http://localhost:9001](http://localhost:9001) |

---

## 🧪 Testing

Run all unit and integration tests:
```bash
make test
# or: cargo test
```

---

## 🐳 Building Container Image

You can build a production container image using Podman:
```bash
podman build -t lotus-connect-system:latest -f Containerfile .
```

---

## 📖 API Documentation & Client Integration

For detailed REST endpoints, WebSocket event specifications, payload models, and WebRTC signalling contracts, see:
* [Developer Integration Guide](developer_integration_guide.md)
