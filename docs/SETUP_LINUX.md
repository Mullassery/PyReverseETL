# PyReverseETL: Linux Setup Guide

**For Ubuntu 22.04+, Debian 11+, CentOS 8+, Fedora 37+**

---

## Quick Setup (5 minutes)

```bash
# Update system
sudo apt-get update && sudo apt-get upgrade -y

# Install dependencies
sudo apt-get install -y build-essential pkg-config curl git

# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env

# Clone project
git clone https://github.com/Mullassery/PyReverseETL.git
cd PyReverseETL

# Build
cargo build --release

# Run
./target/release/pyreverseetl server --port 8080
```

---

## Detailed Setup by Distribution

### Ubuntu 22.04 LTS (Recommended)

#### System Requirements
- **RAM**: 8GB+ (4GB minimum)
- **Disk**: 30GB free
- **CPU**: 4+ cores recommended
- **Network**: 1 Mbps+ (for dependencies)

#### Step 1: Update System

```bash
sudo apt-get update
sudo apt-get upgrade -y
sudo apt-get install -y software-properties-common curl wget git
```

#### Step 2: Install Build Tools

```bash
# Essential build tools
sudo apt-get install -y \
  build-essential \
  pkg-config \
  cmake \
  gcc \
  g++ \
  make

# Verify gcc
gcc --version
```

#### Step 3: Install Dependencies

```bash
# Required for compiling Rust projects
sudo apt-get install -y \
  libssl-dev \
  libpq-dev \
  postgresql-client \
  python3-dev \
  python3-pip \
  libsqlite3-dev

# Optional but recommended
sudo apt-get install -y \
  git-core \
  curl \
  htop \
  vim \
  tmux
```

#### Step 4: Install Rust

```bash
# Download and install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Follow prompts (accept defaults)
# Add to PATH
source "$HOME/.cargo/env"

# Verify
rustc --version
cargo --version

# Update Rust periodically
rustup update
```

#### Step 5: Install Python Tools

```bash
# Upgrade pip
python3 -m pip install --upgrade pip

# Install maturin for Python bindings
pip3 install maturin wheel

# Verify
maturin --version
```

#### Step 6: Install Docker (for test infrastructure)

```bash
# Install Docker
curl -fsSL https://get.docker.com -o get-docker.sh
sudo sh get-docker.sh

# Add user to docker group (optional but recommended)
sudo usermod -aG docker $USER
newgrp docker

# Verify
docker --version
docker ps

# Install Docker Compose
sudo curl -L "https://github.com/docker/compose/releases/latest/download/docker-compose-$(uname -s)-$(uname -m)" \
  -o /usr/local/bin/docker-compose
sudo chmod +x /usr/local/bin/docker-compose

docker-compose --version
```

#### Step 7: Clone & Build

```bash
# Create workspace
mkdir -p ~/projects
cd ~/projects

# Clone project
git clone https://github.com/Mullassery/PyReverseETL.git
cd PyReverseETL

# Build (optimized)
cargo build --release

# Takes 3-5 minutes on first build
# Check progress with: ls -lh target/release/pyreverseetl
```

---

### Debian 11+

Mostly same as Ubuntu, but:

```bash
# Update
sudo apt-get update && sudo apt-get upgrade -y

# Install core packages
sudo apt-get install -y \
  build-essential pkg-config curl git \
  libssl-dev libpq-dev python3-dev python3-pip

# Follow Ubuntu steps 4-7 above
```

---

### CentOS 8+ / RHEL / Fedora

```bash
# Update system
sudo dnf update -y
sudo dnf groupinstall -y "Development Tools"

# Install dependencies
sudo dnf install -y \
  pkg-config \
  openssl-devel \
  libpq-devel \
  python3-devel \
  python3-pip \
  git \
  curl

# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env

# Install Docker
sudo dnf install -y docker docker-compose
sudo systemctl start docker
sudo systemctl enable docker
sudo usermod -aG docker $USER

# Continue with Ubuntu steps 6-7 above
```

---

### Alpine Linux (Minimal)

```bash
# Alpine is minimal, add required packages
apk add --no-cache \
  build-base \
  curl \
  git \
  openssl-dev \
  sqlite-dev \
  postgresql-dev \
  python3-dev \
  docker \
  docker-compose

# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Source and continue
source $HOME/.cargo/env
```

---

## Build & Run

### Build Optimized Release

```bash
# Standard release build
cargo build --release

# Build time depends on hardware
# Expected: 2-5 minutes on 4-core, 8GB RAM
# First build is slower (compiles all deps)
```

### Start Test Infrastructure

```bash
# Ensure Docker is running
sudo systemctl start docker

# Or with Docker Desktop
# Start Docker Desktop from GUI

# Start all test services
docker-compose -f docker-compose.local.yml up -d

# Wait for services to be healthy (~30 seconds)
docker-compose -f docker-compose.local.yml ps

# Watch logs
docker-compose -f docker-compose.local.yml logs -f
```

### Run Server

```bash
# Run API server (foreground)
./target/release/pyreverseetl server --port 8080

# Or run in background
nohup ./target/release/pyreverseetl server --port 8080 > server.log 2>&1 &

# Or with tmux
tmux new-session -d -s pyreverseetl "./target/release/pyreverseetl server --port 8080"
tmux attach -t pyreverseetl
```

### Test Connection

```bash
# Health check
curl http://localhost:8080/health

# List connectors
curl http://localhost:8080/connectors | jq .

# View Grafana dashboards
# http://localhost:3001
# http://localhost:9090  (Prometheus)
# http://localhost:16686 (Jaeger)
```

---

## Testing

### Run Tests

```bash
# Full test suite
cargo test --all --release

# Specific tests
cargo test connector_test --lib

# Watch mode (requires cargo-watch)
cargo install cargo-watch
cargo watch -x "test --all --release"

# With logging
RUST_LOG=debug cargo test --all -- --nocapture
```

### Performance Benchmarks

```bash
# Run benchmarks
cargo bench --all

# Specific benchmark
cargo bench --bench sync_engine

# Profile with perf
sudo apt-get install linux-perf
sudo perf record -g ./target/release/pyreverseetl server --port 8080
sudo perf report
```

---

## Systemd Service (Optional)

Create `/etc/systemd/system/pyreverseetl.service`:

```ini
[Unit]
Description=PyReverseETL Data Activation Runtime
After=network.target docker.service
Wants=docker.service

[Service]
Type=simple
User=pyreverseetl
WorkingDirectory=/opt/pyreverseetl
EnvironmentFile=/opt/pyreverseetl/.env
ExecStart=/opt/pyreverseetl/pyreverseetl server --port 8080
Restart=on-failure
RestartSec=10

# Resource limits
MemoryLimit=4G
CPUQuota=80%

# Logging
StandardOutput=journal
StandardError=journal
SyslogIdentifier=pyreverseetl

[Install]
WantedBy=multi-user.target
```

Setup:

```bash
# Create user
sudo useradd -r -s /bin/false pyreverseetl

# Copy binary
sudo cp target/release/pyreverseetl /opt/pyreverseetl/
sudo chown -R pyreverseetl:pyreverseetl /opt/pyreverseetl

# Enable service
sudo systemctl daemon-reload
sudo systemctl enable pyreverseetl
sudo systemctl start pyreverseetl

# Check status
sudo systemctl status pyreverseetl
sudo journalctl -u pyreverseetl -f
```

---

## Server Deployment

### Docker Compose (Recommended)

```bash
# Create deployment directory
sudo mkdir -p /opt/pyreverseetl/{config,backups,certs}
cd /opt/pyreverseetl

# Create environment file
sudo cat > .env << 'EOF'
DB_USER=pyreverseetl
DB_PASSWORD=$(openssl rand -base64 32)
JWT_SECRET=$(openssl rand -base64 32)
ENCRYPTION_KEY=$(openssl rand -base64 32)
GRAFANA_USER=admin
GRAFANA_PASSWORD=$(openssl rand -base64 16)
EOF

sudo chmod 600 .env

# Copy docker-compose
sudo cp ~/projects/PyReverseETL/docker-compose.server.yml .

# Start services
sudo docker-compose -f docker-compose.server.yml up -d

# Verify
sudo docker-compose ps
curl http://localhost:8080/health
```

### Kubernetes (Optional)

```bash
# Install kubectl
curl -LO "https://dl.k8s.io/release/$(curl -L -s https://dl.k8s.io/release/stable.txt)/bin/linux/amd64/kubectl"
sudo install -o root -g root -m 0755 kubectl /usr/local/bin/kubectl

# Deploy
kubectl apply -f kubernetes/
kubectl get pods
```

---

## Troubleshooting

### Build Fails with "Cannot find -lssl"

```bash
# Ubuntu/Debian
sudo apt-get install libssl-dev

# CentOS/RHEL
sudo dnf install openssl-devel

# Retry
cargo clean
cargo build --release
```

### Build Fails with "Python.h not found"

```bash
# Ubuntu/Debian
sudo apt-get install python3-dev

# CentOS/RHEL
sudo dnf install python3-devel

# Retry
cargo build --release
```

### Docker Permission Denied

```bash
# Add user to docker group
sudo usermod -aG docker $USER
newgrp docker

# Verify
docker ps

# Or use sudo
sudo docker-compose -f docker-compose.local.yml up -d
```

### Port 8080 Already in Use

```bash
# Find what's using it
sudo lsof -i :8080
sudo netstat -tlnp | grep 8080

# Kill the process
sudo kill -9 <PID>

# Or use different port
./target/release/pyreverseetl server --port 8081
```

### Out of Memory During Build

```bash
# Limit parallel jobs
cargo build --release -j 2

# Or clean and retry
cargo clean
cargo build --release

# Increase swap (if needed)
sudo fallocate -l 4G /swapfile
sudo chmod 600 /swapfile
sudo mkswap /swapfile
sudo swapon /swapfile
```

### DNS Issues with Docker

```bash
# Edit daemon.json
sudo nano /etc/docker/daemon.json

# Add:
{
  "dns": ["8.8.8.8", "8.8.4.4"]
}

# Restart Docker
sudo systemctl restart docker
```

---

## Performance Tuning

### System Limits

```bash
# Check file descriptor limit
ulimit -n

# Increase if needed
sudo nano /etc/security/limits.conf
# Add:
# * soft nofile 65535
# * hard nofile 65535

# Apply
ulimit -n 65535
```

### Network Tuning

```bash
# Increase network buffer sizes
sudo sysctl -w net.core.rmem_max=134217728
sudo sysctl -w net.core.wmem_max=134217728

# Persist in /etc/sysctl.conf
sudo nano /etc/sysctl.conf
# Add above lines
sudo sysctl -p
```

### Kernel Parameters

```bash
# Increase max connections
sudo sysctl -w net.core.somaxconn=65535

# Increase backlog
sudo sysctl -w net.ipv4.tcp_max_syn_backlog=65535

# Persist these in /etc/sysctl.conf
```

---

## Build Times by Hardware

| Hardware | First Build | Incremental | Release |
|----------|-------------|-------------|---------|
| 2-core VM | 15-20m | 2-3m | 10-15m |
| 4-core 8GB | 5-7m | 30-60s | 3-5m |
| 8-core 16GB | 2-3m | 15-30s | 1-2m |
| 16-core 32GB | 1-2m | 10-15s | 30-60s |

---

## Next Steps

1. **Complete Setup**: Follow steps above (20-40 minutes)
2. **Run Tests**: `cargo test --all` (5-10 minutes)
3. **Start Development**: See [QUICKSTART.md](QUICKSTART.md)
4. **Deploy to Production**: Use [docker-compose.server.yml](docker-compose.server.yml)

---

## Resources

- [Rust on Linux](https://www.rust-lang.org/tools/install)
- [Docker Docs](https://docs.docker.com/)
- [PyReverseETL Docs](DEVELOPMENT.md)
- [Linux Performance Tuning](https://www.kernel.org/doc/html/latest/)

---

**Status**: ✅ Production Ready on Linux  
**Last Updated**: 2026-07-18  
**Support**: github.com/Mullassery/PyReverseETL/issues
