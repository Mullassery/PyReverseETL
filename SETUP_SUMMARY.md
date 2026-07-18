# PyReverseETL: Complete Setup Guide Summary

**Multi-platform setup for Windows, Linux, and Macbooks**

---

## 🚀 Quick Links

### Setup Guides by Platform
- **[Windows WSL2](SETUP_WINDOWS_WSL.md)** — Windows 10/11 with WSL2 (Ubuntu 22.04)
- **[Linux](SETUP_LINUX.md)** — Ubuntu, Debian, CentOS, Fedora, Alpine
- **[Macbook](SETUP_MACBOOK.md)** — Intel and Apple Silicon (M1/M2/M3)
- **[Quick Start](QUICKSTART.md)** — 5-minute setup for all platforms
- **[Full Development Guide](DEVELOPMENT.md)** — Complete dev & server deployment

### Technology Docs
- **[Distributed Processing](docs/DISTRIBUTED_PROCESSING.md)** — PySpark (micro-batch) & PyFlink (streaming)
- **[v2.1 Implementation Plan](docs/V2.1_IMPLEMENTATION_PLAN.md)** — 50 core connectors roadmap
- **[Connector Ecosystem](docs/CONNECTOR_ECOSYSTEM.md)** — 280+ available connectors

---

## Platform-Specific Quick Reference

### Windows WSL2 (5 minutes)
```powershell
# In PowerShell (Admin)
wsl --install -d Ubuntu-22.04
wsl

# In WSL Terminal
sudo apt-get update
sudo apt-get install -y build-essential curl git rustup
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
git clone https://github.com/Mullassery/PyReverseETL.git
cd PyReverseETL
cargo build --release
./target/release/pyreverseetl server --port 8080
```

### Linux / Ubuntu (5 minutes)
```bash
# Ubuntu 22.04+
sudo apt-get update
sudo apt-get install -y build-essential pkg-config curl git libssl-dev python3-dev docker.io
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env
git clone https://github.com/Mullassery/PyReverseETL.git
cd PyReverseETL
cargo build --release
docker-compose -f docker-compose.local.yml up -d
./target/release/pyreverseetl server --port 8080
```

### Macbook (5 minutes)
```bash
# Intel or Apple Silicon (M1/M2/M3)
/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
brew install rustup docker git python@3.11
rustup-init
source ~/.cargo/env
git clone https://github.com/Mullassery/PyReverseETL.git
cd PyReverseETL
cargo build --release
docker-compose -f docker-compose.local.yml up -d
./target/release/pyreverseetl server --port 8080
```

---

## Key Differences by Platform

| Aspect | Windows WSL2 | Linux | Macbook |
|--------|-------------|-------|---------|
| **Package Manager** | `apt-get` | `apt/dnf` | Homebrew |
| **Installation Time** | 30-40 min | 20-30 min | 15-20 min |
| **Build Time** | 5-7 min | 3-5 min | 2-3 min (M1/M2) |
| **Docker** | Docker Desktop | Docker CE | Docker Desktop |
| **Performance** | 80% native | 100% native | 100% native |
| **Best For** | Windows users | Servers | Mac developers |

---

## Development Workflow (All Platforms)

### Terminal Setup
```bash
# Terminal 1: Start infrastructure
docker-compose -f docker-compose.local.yml up

# Terminal 2: Watch for code changes
cargo watch -x "build --release"

# Terminal 3: Run server
./target/release/pyreverseetl server

# Terminal 4: Run tests
cargo watch -x "test --all"

# Terminal 5: Test API
curl http://localhost:8080/health
```

### IDE Integration
- **VS Code**: Install "Rust Analyzer" extension, open folder in WSL/Container
- **JetBrains**: Use "Remote Development" to connect to WSL
- **Vim/Neovim**: Works natively with Rust tools

---

## Important Notes

### 🚫 **Not Included (By Design)**
- ❌ Web UI Dashboard — Not part of this tool
  - Use **Grafana** (http://localhost:3001) for dashboards
  - Use **Prometheus** (http://localhost:9090) for metrics
  - Use **Jaeger** (http://localhost:16686) for traces

### ✅ **Included for Distributed Processing**
- ✅ **PySpark** — Micro-batch transformations
  - Use for: Scheduled ETL (hourly, daily)
  - Throughput: 1-100 GB/min
  - Latency: 100ms - 10s

- ✅ **PyFlink** — True streaming
  - Use for: Real-time event processing
  - Throughput: 100MB - 10 GB/min
  - Latency: 10-100ms

See [Distributed Processing Guide](docs/DISTRIBUTED_PROCESSING.md) for details.

---

## System Requirements Summary

### Minimum (Development)
- **RAM**: 4GB (8GB better)
- **Disk**: 30GB free
- **CPU**: 2 cores (4+ recommended)
- **Network**: 1 Mbps+

### Recommended (Development)
- **RAM**: 16GB+
- **Disk**: 100GB SSD
- **CPU**: 4+ cores
- **Network**: 10+ Mbps

### Production Server
- **RAM**: 8-16GB
- **Disk**: 50-200GB SSD
- **CPU**: 4+ cores
- **Network**: 100+ Mbps

---

## Build Performance Benchmarks

### First Build (all deps compiled)
- Windows WSL2: 5-7 minutes
- Linux: 3-5 minutes
- Macbook M1/M2: 2-3 minutes
- Macbook Intel: 3-5 minutes

### Incremental Build
- Windows WSL2: 1-2 minutes
- Linux: 30-60 seconds
- Macbook M1/M2: 15-30 seconds
- Macbook Intel: 30-60 seconds

### Full Test Suite
- Windows WSL2: 10-15 minutes
- Linux: 5-10 minutes
- Macbook M1/M2: 3-5 minutes
- Macbook Intel: 5-8 minutes

---

## Troubleshooting by Platform

### Windows WSL2
See [SETUP_WINDOWS_WSL.md#troubleshooting](SETUP_WINDOWS_WSL.md#troubleshooting)
- **Most common**: Docker connection, port conflicts, memory issues

### Linux
See [SETUP_LINUX.md#troubleshooting](SETUP_LINUX.md#troubleshooting)
- **Most common**: Missing dev headers, permission issues, package conflicts

### Macbook
See [SETUP_MACBOOK.md#troubleshooting](SETUP_MACBOOK.md#troubleshooting)
- **Most common**: Rust PATH issues, OpenSSL linking, Rosetta warnings

---

## Common Commands (All Platforms)

```bash
# Build
cargo build --release

# Test
cargo test --all --release

# Run server
./target/release/pyreverseetl server --port 8080

# Start infrastructure
docker-compose -f docker-compose.local.yml up -d

# Stop infrastructure
docker-compose -f docker-compose.local.yml down

# View logs
docker-compose -f docker-compose.local.yml logs -f

# Clean build
cargo clean
cargo build --release

# Format code
cargo fmt --all

# Lint
cargo clippy --all -- -D warnings

# Benchmark
cargo bench --all
```

---

## Accessing Local Services

| Service | URL | Purpose |
|---------|-----|---------|
| API | `http://localhost:8080` | REST API |
| Grafana | `http://localhost:3001` | Dashboards (admin/admin) |
| Prometheus | `http://localhost:9090` | Metrics |
| Jaeger | `http://localhost:16686` | Traces |
| PostgreSQL | `localhost:5432` | Test database |
| MySQL | `localhost:3306` | Test database |
| MongoDB | `localhost:27017` | Test database |
| Redis | `localhost:6379` | Cache |
| Kafka | `localhost:9092` | Message queue |

---

## Next Steps

### 1. Choose Your Platform
- Pick the relevant setup guide above
- Follow installation steps (15-40 minutes)

### 2. Verify Installation
```bash
rustc --version
cargo --version
docker --version
curl http://localhost:8080/health  # Should show {"status":"ok"}
```

### 3. Start Development
- See [QUICKSTART.md](QUICKSTART.md) for quick start
- See [DEVELOPMENT.md](DEVELOPMENT.md) for full workflow
- Read [v2.1 Implementation Plan](docs/V2.1_IMPLEMENTATION_PLAN.md)

### 4. Implement Connectors
- Start with [Top 10 connectors](docs/V2.1_IMPLEMENTATION_PLAN.md#tier-1-core-top-10---priority-1)
- PostgreSQL template already documented
- Follow test harness framework in `core/src/testing/`

### 5. Deploy to Server
- Follow [Server Deployment](DEVELOPMENT.md#server-deployment) guide
- Use `docker-compose.server.yml` for production
- Monitor with Prometheus + Grafana

---

## Support & Resources

### Official Docs
- GitHub: [github.com/Mullassery/PyReverseETL](https://github.com/Mullassery/PyReverseETL)
- Issues: [Report bugs](https://github.com/Mullassery/PyReverseETL/issues)
- Discussions: [Ask questions](https://github.com/Mullassery/PyReverseETL/discussions)

### External Resources
- [Rust Book](https://doc.rust-lang.org/book/)
- [Cargo Guide](https://doc.rust-lang.org/cargo/)
- [Docker Docs](https://docs.docker.com/)
- [Kubernetes Docs](https://kubernetes.io/docs/)

---

## Summary Table: Choose Your Setup

```
┌─────────────────┬──────────────────┬─────────────────┬──────────────────┐
│ Platform        │ Install Time     │ Build Time      │ Performance      │
├─────────────────┼──────────────────┼─────────────────┼──────────────────┤
│ Windows WSL2    │ 30-40 min        │ 5-7 min         │ 80% native       │
│ Ubuntu Linux    │ 20-30 min        │ 3-5 min         │ 100% native      │
│ Macbook Intel   │ 15-20 min        │ 3-5 min         │ 100% native      │
│ Macbook M1/M2   │ 15-20 min        │ 2-3 min         │ 100% native ⭐   │
└─────────────────┴──────────────────┴─────────────────┴──────────────────┘
```

**Recommended**: Macbook M1/M2 for fastest development iteration.

---

**Ready to start?** Pick your platform above and follow the setup guide!

**Status**: ✅ All platforms production-ready  
**Last Updated**: 2026-07-18  
**Version**: v2.0.1+
