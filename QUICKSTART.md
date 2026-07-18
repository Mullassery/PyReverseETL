# PyReverseETL: Quick Start Guide

**Mac Studio Development → Server Deployment**

---

## 🍎 Mac Studio: 5-Minute Setup

### 1. Install Prerequisites
```bash
# Homebrew
/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"

# Core tools
brew install rust rustup docker git

# Verify
rustc --version  # 1.70+
docker --version # 24.0+
```

### 2. Clone & Build
```bash
git clone https://github.com/Mullassery/PyReverseETL.git
cd PyReverseETL

# Start test infrastructure
docker-compose -f docker-compose.local.yml up -d

# Build optimized binary (2-3 min on Mac Studio)
cargo build --release

# Wait for "Finished" message
```

### 3. Start Server
```bash
# Terminal 1: Run API server
./target/release/pyreverseetl server --port 8080

# Should print: "Server listening on 0.0.0.0:8080"
```

### 4. Test It Works
```bash
# Terminal 2: Test API
curl http://localhost:8080/health

# Expected: {"status":"ok"}

# List test databases
curl http://localhost:8080/connectors | jq .

# View Grafana dashboards (optional)
open http://localhost:3001
```

### 5. Run Tests
```bash
# Full test suite
cargo test --all

# Just connector tests
cargo test testing::harness

# With logging
RUST_LOG=debug cargo test --all -- --nocapture
```

✅ **That's it!** You're running PyReverseETL on Mac Studio.

---

## 🖥️ Server: Deploy in 5 Minutes

### 1. Prepare Server (SSH)
```bash
# SSH into server
ssh ubuntu@your-server.com

# Create directory
mkdir -p /opt/pyreverseetl/{config,backups}
cd /opt/pyreverseetl

# Create secrets file
cat > .env << 'EOF'
DB_USER=pyreverseetl
DB_PASSWORD=$(openssl rand -base64 32)
JWT_SECRET=$(openssl rand -base64 32)
ENCRYPTION_KEY=$(openssl rand -base64 32)
GRAFANA_USER=admin
GRAFANA_PASSWORD=$(openssl rand -base64 16)
EOF

chmod 600 .env
```

### 2. Copy Configuration
```bash
# From your Mac, copy files to server
scp -r PyReverseETL/docker-compose.server.yml ubuntu@your-server.com:/opt/pyreverseetl/
scp -r PyReverseETL/config/* ubuntu@your-server.com:/opt/pyreverseetl/config/

# Back on server
cd /opt/pyreverseetl
```

### 3. Start Services
```bash
# Pull latest images
docker-compose -f docker-compose.server.yml pull

# Start everything
docker-compose -f docker-compose.server.yml up -d

# Watch logs
docker-compose logs -f pyreverseetl

# Wait for "Listening on 0.0.0.0:8080"
```

### 4. Verify Deployment
```bash
# Health check
curl http://localhost:8080/health

# Prometheus
curl http://localhost:9090/api/v1/query?query=up

# Grafana dashboard
open http://your-server.com:3000
```

### 5. Enable HTTPS (Optional)
```bash
# Install certbot
sudo apt-get install certbot python3-certbot-nginx

# Get certificate (replace domain)
sudo certbot certonly --standalone -d yourdomain.com

# Update nginx config
sudo cp /etc/letsencrypt/live/yourdomain.com/privkey.pem /opt/pyreverseetl/certs/
sudo cp /etc/letsencrypt/live/yourdomain.com/fullchain.pem /opt/pyreverseetl/certs/
```

✅ **Server is live!** Access at `http://your-server.com:8080`

---

## 📊 Development Workflow

### Daily Development on Mac

```bash
# Terminal 1: Infrastructure (one-time)
docker-compose -f docker-compose.local.yml up

# Terminal 2: Watch for code changes and rebuild
cargo watch -x "build --release"

# Terminal 3: Run server
./target/release/pyreverseetl server

# Terminal 4: Run tests as you code
cargo watch -x "test --lib"

# Terminal 5: Interact with API
curl http://localhost:8080/connectors
```

### Adding a New Connector

```bash
# 1. Add connector implementation in core/src/connectors/
# 2. Add tests in core/src/testing/
# 3. Add documentation in docs/connectors/[CONNECTOR].md
# 4. Run tests
cargo test --all

# 5. Commit and push
git add -A
git commit -m "feat: Add [Connector] connector"
git push
```

### Before Pushing to Server

```bash
# Run full test suite
cargo test --all --release

# Check formatting
cargo fmt --all -- --check

# Lint
cargo clippy --all -- -D warnings

# Build release binary
cargo build --release

# Push to repo
git push origin main
```

---

## 🚀 Production Workflows

### Deploy Latest Code to Server

```bash
# On server
cd /opt/pyreverseetl

# Pull latest (if using git)
git pull

# Or manually copy
scp PyReverseETL/docker-compose.server.yml ubuntu@server:/opt/pyreverseetl/

# Restart with zero downtime
docker-compose -f docker-compose.server.yml up -d --scale pyreverseetl=2
docker-compose -f docker-compose.server.yml ps

# Verify health
curl http://localhost:8080/health
```

### View Live Logs

```bash
# On server
docker-compose logs -f pyreverseetl

# Specific service
docker-compose logs -f postgres
docker-compose logs -f prometheus
```

### Backup Database

```bash
# Manual backup
docker-compose exec postgres pg_dump -U pyreverseetl pyreverseetl > backup-$(date +%Y%m%d-%H%M%S).sql

# Restore from backup
cat backup-20240718.sql | docker-compose exec -T postgres psql -U pyreverseetl pyreverseetl
```

### Scale Up

```bash
# Edit docker-compose.server.yml
# Change: replicas: 2 → replicas: 4

# Apply
docker-compose -f docker-compose.server.yml up -d --scale pyreverseetl=4

# Verify
docker-compose ps
```

---

## 🔧 Common Tasks

### View API Metrics
```bash
curl http://localhost:9090/api/v1/query?query=pyreverseetl_sync_duration_seconds
```

### Access Grafana Dashboards
```bash
# Mac: http://localhost:3001
# Server: http://your-server.com:3000
# Login: admin / [GRAFANA_PASSWORD from .env]
```

### View Trace Logs (Jaeger)
```bash
# Mac: http://localhost:16686
# Server: http://your-server.com:16686
# Search by service: pyreverseetl
```

### Test Specific Connector
```bash
# Run PostgreSQL tests
cargo test postgres --all

# Run with logging
RUST_LOG=debug cargo test postgres --all -- --nocapture
```

### Rebuild Container Image
```bash
# Local (Mac)
docker build -t pyreverseetl:latest .

# Server
docker-compose -f docker-compose.server.yml build --no-cache
docker-compose -f docker-compose.server.yml up -d
```

---

## 📈 Performance Tips

### Mac Studio Development
- Use `cargo build --release` for accurate performance testing
- Profile with `cargo flamegraph`
- Benchmarks: `cargo bench --all`

### Server Production
- Set `DB_POOL_SIZE` to `CPU_CORES * 2`
- Adjust `MAX_BATCH_SIZE` based on memory
- Monitor with `docker stats`
- Scale replicas based on load

---

## ❓ Troubleshooting

### "Connection refused"
```bash
# Check if services are running
docker-compose ps

# Restart services
docker-compose -f docker-compose.local.yml restart

# Check port 8080 is available
lsof -i :8080
```

### "Database connection failed"
```bash
# Check PostgreSQL is healthy
docker-compose exec postgres pg_isready

# Check credentials
echo $DB_PASSWORD  # Should be set

# View logs
docker-compose logs postgres
```

### "Build takes too long"
```bash
# Clean build cache
cargo clean

# Use incremental builds
cargo build (without --release)

# Or just fetch prebuilt binary
wget https://releases.github.com/Mullassery/PyReverseETL/v2.0.1/pyreverseetl-arm64
chmod +x pyreverseetl-arm64
./pyreverseetl-arm64 server --port 8080
```

### "Server memory usage high"
```bash
# Check resource usage
docker stats

# Reduce cache size
docker-compose exec pyreverseetl pyreverseetl config --cache-size-mb 512

# Or reduce replicas
docker-compose -f docker-compose.server.yml up -d --scale pyreverseetl=1
```

---

## 📚 Learn More

- **Full Development Guide**: [DEVELOPMENT.md](DEVELOPMENT.md)
- **Implementation Plan**: [docs/V2.1_IMPLEMENTATION_PLAN.md](docs/V2.1_IMPLEMENTATION_PLAN.md)
- **Connector Docs**: [docs/connectors/](docs/connectors/)
- **Architecture**: [docs/ADVANCED_ARCHITECTURE_PATTERNS.md](docs/ADVANCED_ARCHITECTURE_PATTERNS.md)
- **GitHub**: [github.com/Mullassery/PyReverseETL](https://github.com/Mullassery/PyReverseETL)

---

## 🆘 Need Help?

- **Issues**: github.com/Mullassery/PyReverseETL/issues
- **Discussions**: github.com/Mullassery/PyReverseETL/discussions
- **Email**: mullassery@gmail.com

---

**Ready to go!** 🚀

Start with **Mac Studio** for development, deploy to **Server** for production.
