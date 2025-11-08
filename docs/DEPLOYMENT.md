# Deployment Guide

## Prerequisites

### Software Requirements
- Rust 1.75+
- Solana CLI 1.17+
- Anchor CLI 0.29+
- Node.js 16+
- MongoDB 4.4+
- Docker (optional)

### Accounts & Access
- Solana wallet with SOL for deployment
- MongoDB instance (local or Atlas)
- USDT token mint address

---

## Local Development Setup

### 1. Install Dependencies

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install Solana CLI
sh -c "$(curl -sSfL https://release.solana.com/stable/install)"

# Install Anchor
cargo install --git https://github.com/coral-xyz/anchor avm --locked --force
avm install 0.29.0
avm use 0.29.0

# Install Node dependencies
npm install
```

### 2. Start Local Validator

```bash
# Terminal 1: Start Solana test validator
solana-test-validator
```

### 3. Start MongoDB

```bash
# Option 1: Docker
docker run -d -p 27017:27017 --name mongodb mongo:latest

# Option 2: Local installation
mongod --dbpath /path/to/data
```

### 4. Configure Environment

```bash
# Copy example env file
cp .env.example .env

# Edit .env with your values
# Especially USDT_MINT and MONGODB_URI
```

### 5. Build and Deploy Anchor Program

```bash
# Build the program
anchor build

# Get the program ID
solana address -k target/deploy/vault_manager-keypair.json

# Update Anchor.toml and lib.rs with the program ID

# Deploy
anchor deploy
```

### 6. Run Backend Service

```bash
# Build
cargo build --release

# Run
cargo run --release
```

### 7. Test

```bash
# Run Anchor tests
anchor test

# Run Rust unit tests
cargo test

# Run backend integration tests
cargo test --test integration
```

---

## Devnet Deployment

### 1. Configure Solana CLI

```bash
# Set to devnet
solana config set --url https://api.devnet.solana.com

# Create/set keypair
solana-keygen new -o ~/.config/solana/id.json

# Airdrop SOL
solana airdrop 2
```

### 2. Update Configuration

```bash
# Update .env
SOLANA_RPC_URL=https://api.devnet.solana.com
SOLANA_WS_URL=wss://api.devnet.solana.com
```

### 3. Deploy Program

```bash
# Build
anchor build

# Deploy to devnet
anchor deploy --provider.cluster devnet

# Verify deployment
solana program show <PROGRAM_ID>
```

### 4. Initialize Authority

```bash
# Use Anchor client or write a script
# See tests/vault-manager.ts for example
```

### 5. Deploy Backend

```bash
# Build release binary
cargo build --release

# Run on server
./target/release/vault-manager-service
```

---

## Production Deployment

### 1. Infrastructure Setup

#### Option A: VPS/Dedicated Server

```bash
# Install Docker and Docker Compose
curl -fsSL https://get.docker.com -o get-docker.sh
sh get-docker.sh

# Install docker-compose
sudo curl -L "https://github.com/docker/compose/releases/download/v2.20.0/docker-compose-$(uname -s)-$(uname -m)" -o /usr/local/bin/docker-compose
sudo chmod +x /usr/local/bin/docker-compose
```

#### Option B: Kubernetes

See `kubernetes/` directory for manifests (to be created).

### 2. MongoDB Setup

#### MongoDB Atlas (Recommended)

1. Create account at https://www.mongodb.com/cloud/atlas
2. Create a cluster
3. Create database user
4. Whitelist IP addresses
5. Get connection string
6. Update MONGODB_URI in .env

#### Self-Hosted MongoDB

```bash
# Docker Compose for MongoDB with replica set
docker-compose up -d mongodb
```

### 3. Solana Mainnet Configuration

```bash
# Set to mainnet
solana config set --url https://api.mainnet-beta.solana.com

# Or use a private RPC (recommended)
# Examples: QuickNode, Helius, Triton

# Update .env
SOLANA_RPC_URL=https://your-rpc-provider.com
```

### 4. Security Hardening

#### SSL/TLS Setup

```bash
# Install certbot
sudo apt-get install certbot

# Get certificate
sudo certbot certonly --standalone -d api.yourdomain.com

# Update Nginx/reverse proxy config
```

#### Environment Variables

```bash
# Use secrets management
# - AWS Secrets Manager
# - HashiCorp Vault
# - Kubernetes Secrets

# Never commit .env to git
echo ".env" >> .gitignore
```

#### Firewall Rules

```bash
# Allow only necessary ports
sudo ufw allow 22    # SSH
sudo ufw allow 80    # HTTP
sudo ufw allow 443   # HTTPS
sudo ufw enable
```

### 5. Deploy Backend Service

#### Systemd Service

Create `/etc/systemd/system/vault-manager.service`:

```ini
[Unit]
Description=Vault Manager Service
After=network.target

[Service]
Type=simple
User=vault-service
WorkingDirectory=/opt/vault-manager
ExecStart=/opt/vault-manager/target/release/vault-manager-service
Restart=always
RestartSec=10
Environment="RUST_LOG=info"
EnvironmentFile=/opt/vault-manager/.env

[Install]
WantedBy=multi-user.target
```

Enable and start:

```bash
sudo systemctl daemon-reload
sudo systemctl enable vault-manager
sudo systemctl start vault-manager
sudo systemctl status vault-manager
```

#### Docker Deployment

```dockerfile
# Dockerfile
FROM rust:1.75 as builder

WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y libssl-dev ca-certificates
COPY --from=builder /app/target/release/vault-manager-service /usr/local/bin/
COPY .env .env

CMD ["vault-manager-service"]
```

Build and run:

```bash
docker build -t vault-manager:latest .
docker run -d --name vault-manager -p 8080:8080 --env-file .env vault-manager:latest
```

### 6. Reverse Proxy (Nginx)

```nginx
server {
    listen 80;
    server_name api.yourdomain.com;
    
    location / {
        proxy_pass http://localhost:8080;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
    }
}
```

### 7. Monitoring Setup

#### Logging

```bash
# Use journalctl for systemd services
journalctl -u vault-manager -f

# Or use log aggregation
# - ELK Stack
# - Datadog
# - CloudWatch
```

#### Metrics

```bash
# Install Prometheus exporter (future enhancement)
# Set up Grafana dashboards
```

#### Alerts

```bash
# Configure alerts for:
# - Service downtime
# - High error rates
# - Balance discrepancies
# - Unusual transaction activity
```

---

## Post-Deployment Checklist

### Program Verification

- [ ] Program deployed to correct network
- [ ] Program ID updated in all configs
- [ ] Authority initialized
- [ ] Test vault created successfully

### Backend Service

- [ ] Service running and healthy
- [ ] Database connection working
- [ ] Solana RPC connection working
- [ ] WebSocket connections functional
- [ ] API endpoints responding

### Security

- [ ] SSL/TLS enabled
- [ ] Firewall configured
- [ ] Secrets properly managed
- [ ] Rate limiting enabled
- [ ] CORS configured
- [ ] Audit logging enabled

### Monitoring

- [ ] Logs being collected
- [ ] Metrics being tracked
- [ ] Alerts configured
- [ ] Backup strategy in place

### Testing

- [ ] Initialize vault test
- [ ] Deposit test
- [ ] Withdrawal test
- [ ] Lock/unlock test
- [ ] WebSocket test
- [ ] Load testing completed

---

## Maintenance

### Database Backups

```bash
# MongoDB backup
mongodump --uri="mongodb://localhost:27017/vault_manager" --out=/backup/$(date +%Y%m%d)

# Automated backups with cron
0 2 * * * /scripts/backup-mongodb.sh
```

### Updates

```bash
# Pull latest code
git pull origin main

# Rebuild and deploy
anchor build
anchor deploy

# Restart backend
sudo systemctl restart vault-manager
```

### Monitoring

```bash
# Check service status
sudo systemctl status vault-manager

# View logs
journalctl -u vault-manager -n 100

# Check MongoDB
mongo --eval "db.adminCommand('serverStatus')"
```

---

## Troubleshooting

### Common Issues

#### Service Won't Start

```bash
# Check logs
journalctl -u vault-manager -n 50

# Verify configuration
cat .env

# Test MongoDB connection
mongosh $MONGODB_URI
```

#### Program Deployment Fails

```bash
# Check balance
solana balance

# Verify program size
ls -lh target/deploy/*.so

# Check program authority
solana program show <PROGRAM_ID>
```

#### Database Connection Issues

```bash
# Test connection
mongosh $MONGODB_URI

# Check firewall
telnet mongodb-host 27017

# Verify credentials
```

---

## Rollback Procedure

### Program Rollback

```bash
# Redeploy previous version
anchor deploy --program-keypair backup/vault_manager-keypair.json
```

### Backend Rollback

```bash
# Systemd
sudo systemctl stop vault-manager
sudo cp backup/vault-manager-service /opt/vault-manager/target/release/
sudo systemctl start vault-manager

# Docker
docker stop vault-manager
docker run -d --name vault-manager vault-manager:previous
```

---

## Support

For deployment issues:
1. Check logs first
2. Review configuration
3. Verify network connectivity
4. Consult documentation

---

**Remember**: Always test on devnet before mainnet deployment!
