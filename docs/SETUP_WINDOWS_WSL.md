# PyReverseETL: Windows WSL2 Setup Guide

**For Windows 10/11 with WSL2 (Ubuntu 22.04 LTS)**

---

## Prerequisites

### System Requirements
- **Windows**: Windows 10 (build 19041+) or Windows 11
- **RAM**: 8GB+ (16GB+ recommended)
- **Disk**: 50GB free on WSL drive
- **CPU**: 4+ cores

### Step 1: Install WSL2

#### Enable WSL2 in PowerShell (Run as Administrator)
```powershell
# Enable WSL feature
wsl --install

# Or manually:
dism.exe /online /enable-feature /featurename:Microsoft-Windows-Subsystem-Linux /all /norestart
dism.exe /online /enable-feature /featurename:VirtualMachinePlatform /all /norestart

# Set default version to WSL2
wsl --set-default-version 2

# Restart computer when prompted
```

#### Install Ubuntu 22.04
```powershell
# From Microsoft Store or command line:
wsl --install -d Ubuntu-22.04

# Or manually download from Microsoft Store
# Search for "Ubuntu 22.04 LTS"
```

#### Verify WSL2 Installation
```powershell
wsl --version
wsl --list --verbose

# Should show:
# NAME              STATE           VERSION
# Ubuntu-22.04      Running         2
```

### Step 2: Configure WSL2 Performance

Create `.wslconfig` in `C:\Users\<YourUsername>\.wslconfig`:

```ini
[wsl2]
# Allocate 4 CPU cores (adjust as needed)
processors=4

# Allocate 8GB RAM (adjust as needed)
memory=8GB

# Allow localhostForwarding
localhostForwarding=true

# Swap size (optional)
swap=2GB

# Disable page reporting for better performance
pageReporting=false

# Virtual disk growth limit
sparseVhd=true

# Use WSL 2 light distro mode (faster)
guiApplications=true
```

Restart WSL:
```powershell
wsl --shutdown
wsl
```

### Step 3: Install Docker Desktop

1. Download [Docker Desktop for Windows](https://www.docker.com/products/docker-desktop)
2. Install and restart
3. In Docker Desktop Settings:
   - ✅ Enable "Use the WSL 2 based engine"
   - ✅ Resources → WSL Integration → Enable Ubuntu-22.04

Verify:
```bash
docker --version
docker run hello-world
```

---

## Setup in WSL2 Terminal

### Launch WSL2
```powershell
# From PowerShell or Windows Terminal
wsl

# Or open Ubuntu from Start Menu
# Or type: wsl in Run dialog (Win+R)
```

### Step 1: Update System

```bash
# Update package lists
sudo apt-get update
sudo apt-get upgrade -y

# Install build essentials
sudo apt-get install -y \
  build-essential \
  pkg-config \
  curl \
  git \
  wget \
  libssl-dev \
  libpq-dev \
  python3-dev \
  python3-pip
```

### Step 2: Install Rust

```bash
# Install Rust toolchain
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Follow prompts (accept defaults)
# Source cargo environment
source $HOME/.cargo/env

# Verify
rustc --version
cargo --version
```

### Step 3: Install Python & Maturin

```bash
# Python already installed, ensure pip is up-to-date
python3 -m pip install --upgrade pip

# Install maturin for Python bindings
pip3 install maturin

# Verify
python3 --version
maturin --version
```

### Step 4: Clone PyReverseETL

```bash
# Create workspace
mkdir -p ~/projects
cd ~/projects

# Clone repository
git clone https://github.com/Mullassery/PyReverseETL.git
cd PyReverseETL

# Verify git
git log --oneline | head -5
```

---

## Build & Run

### Build Optimized Binary

```bash
# Build release version (optimized)
# This takes 3-5 minutes on first build
cargo build --release

# Binary location:
ls -lh target/release/pyreverseetl

# Test binary
./target/release/pyreverseetl --version
```

### Start Test Infrastructure

```bash
# Ensure Docker is running
docker --version

# Start services (from PyReverseETL directory)
docker-compose -f docker-compose.local.yml up -d

# Wait for services to be healthy (~30 seconds)
docker-compose -f docker-compose.local.yml ps

# Should show all services as "healthy" or "Up"
```

### Run Server

```bash
# Terminal 1: Run API server
./target/release/pyreverseetl server --port 8080

# Should print:
# Server listening on 0.0.0.0:8080
# [INFO] Started sync scheduler
```

### Test It Works

```bash
# Terminal 2: Test API
curl http://localhost:8080/health

# Expected: {"status":"ok"}

# List connectors
curl http://localhost:8080/connectors | jq .

# View services
docker-compose -f docker-compose.local.yml ps
```

---

## Running Tests

### Full Test Suite

```bash
# Run all tests (5-10 minutes)
cargo test --all --release

# Watch mode (auto-rerun on changes)
cargo install cargo-watch
cargo watch -x "test --all"

# Specific test
cargo test connector_test --lib

# With logging
RUST_LOG=debug cargo test --all -- --nocapture --test-threads=1
```

### Access Dashboards

```bash
# From Windows (not in WSL terminal):
# Grafana: http://localhost:3001 (admin/admin)
# Prometheus: http://localhost:9090
# Jaeger: http://localhost:16686
# Postgres: localhost:5432 (test_user/test_password)
```

---

## Development Tips

### VS Code Integration

1. Install "Remote - WSL" extension in VS Code
2. Open folder in WSL:
   ```bash
   code .
   # Or click "Reopen in WSL" in bottom-left
   ```
3. Continue development as normal (VS Code handles WSL transparently)

### File Sharing Between Windows & WSL

```bash
# WSL2 file location in Windows Explorer:
# \\wsl$\Ubuntu-22.04\home\username\projects\

# Access Windows files from WSL:
cd /mnt/c/Users/YourUsername/Documents
```

### Performance Tips

```bash
# Store projects in WSL filesystem (~2x faster)
# ✅ Good: ~/projects/PyReverseETL
# ❌ Slow: /mnt/c/Users/.../PyReverseETL

# Use WSL native tools, not Windows versions
# Install in WSL: python3, git, rust (not Windows versions)

# Enable WSL resource monitoring
wsl --mount
```

---

## Troubleshooting

### WSL2 Not Starting
```bash
# Check WSL version
wsl --version

# List distros
wsl --list --verbose

# If not running:
wsl -d Ubuntu-22.04

# If ports conflict (Docker Desktop):
netsh int ipv4 set dyn tcp start=49152 num=16384
```

### Docker Connection Refused
```bash
# Restart Docker Desktop
# File → Exit
# Start Docker Desktop again

# Verify connection
docker ps

# If still failing:
wsl --shutdown
wsl
```

### Build Errors

#### "Cannot find -lssl"
```bash
sudo apt-get install libssl-dev libpq-dev

# Retry build
cargo build --release
```

#### "Python development headers not found"
```bash
sudo apt-get install python3-dev

# Retry
cargo build --release
```

#### "Linker errors"
```bash
# Clean and rebuild
cargo clean
cargo build --release

# Or use different linker
RUSTFLAGS="-C link-arg=-fuse-ld=lld" cargo build --release
```

### Memory/Resource Issues

```bash
# Check WSL memory usage
wsl --list --memory

# Shutdown WSL to free memory
wsl --shutdown

# Reduce .wslconfig memory if needed
# Edit C:\Users\<User>\.wslconfig
# [wsl2]
# memory=4GB  # Reduce if needed
```

### Slow Builds

```bash
# Increase CPU cores in .wslconfig
# [wsl2]
# processors=8  # Use more cores

# Or use mold linker (faster)
curl -L https://github.com/rui314/mold/releases/download/v1.11.0/mold-1.11.0-x86_64-linux.tar.gz | tar xz
./mold-1.11.0-x86_64-linux/bin/mold --version

RUSTFLAGS="-C link-arg=-fuse-ld=$(pwd)/mold-1.11.0-x86_64-linux/bin/mold" cargo build --release
```

### Port Already in Use

```bash
# Find what's using port 8080
sudo lsof -i :8080

# Kill the process
sudo kill -9 <PID>

# Or use different port
./target/release/pyreverseetl server --port 8081
```

---

## Upgrading WSL2

```bash
# Check current version
wsl --version

# Update WSL2
wsl --update

# Update Ubuntu packages
sudo apt-get update && sudo apt-get upgrade

# Update Rust
rustup update
```

---

## Performance Benchmarks

### Build Times (WSL2 on 4-core / 8GB RAM allocation)
- Clean build: 5-7 minutes
- Incremental build: 30-60 seconds
- Release build: 3-5 minutes

### Runtime Performance
- API response time: 5-20ms
- Full sync (1000 records): 1-2 seconds
- Connector test suite: 2-5 minutes

### Tips for Better Performance
1. **Allocate more cores**: `processors=8` in .wslconfig
2. **Allocate more RAM**: `memory=16GB` in .wslconfig
3. **Use SSD**: Don't use /mnt/c, use ~/projects in WSL
4. **Clean build**: `cargo clean` between major changes

---

## WSL2-Specific Considerations

### Network Access
- **WSL to Windows**: Use `localhost` or `127.0.0.1`
- **Windows to WSL**: Use WSL's IP (get via `hostname -I`)
- **Bridge network**: Use `--network host` in Docker

### File Permissions
- WSL files are Linux-based
- Windows files may have permission issues
- Store code in WSL filesystem for best results

### Virtual Network
- WSL2 has its own network namespace
- Port forwarding works automatically for localhost
- Access services from Windows using `localhost:PORT`

---

## Next Steps

1. **Complete Setup**: Follow steps above (15-30 minutes)
2. **Run Tests**: `cargo test --all` (5-10 minutes)
3. **Start Development**: See [QUICKSTART.md](QUICKSTART.md)
4. **Explore**: Try connecting connectors, run syncs
5. **Deploy to Server**: When ready, use [Server Deployment Guide](DEVELOPMENT.md#server-deployment)

---

## Resources

- [Microsoft WSL2 Docs](https://learn.microsoft.com/en-us/windows/wsl/)
- [Docker Desktop WSL2 Integration](https://docs.docker.com/desktop/wsl/)
- [Rust on Windows WSL](https://rustup.rs/)
- [PyReverseETL Docs](DEVELOPMENT.md)

---

**Status**: ✅ Production Ready on WSL2  
**Last Updated**: 2026-07-18  
**Support**: github.com/Mullassery/PyReverseETL/issues
