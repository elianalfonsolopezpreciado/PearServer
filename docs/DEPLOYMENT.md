# Deployment Guide

Production deployment guide for Pear Server.

## Table of Contents

1. [Prerequisites](#prerequisites)
2. [systemd Service](#systemd-service-linux)
3. [Docker Deployment](#docker-deployment)
4. [Cloud Platforms](#cloud-platforms)
5. [SSL/TLS Setup](#ssltls-setup)
6. [Monitoring](#monitoring)
7. [Scaling](#scaling)

## Prerequisites

### System Requirements

**Minimum:**
- CPU: 2 cores
- RAM: 4GB
- Disk: 10GB
- OS: Linux (Ubuntu 20.04+, Debian 11+, RHEL 8+)

**Recommended:**
- CPU: 4+ cores
- RAM: 8GB+
- Disk: 20GB+ SSD
- OS: Linux with kernel 5.10+

### Dependencies

```bash
# Ubuntu/Debian
sudo apt update
sudo apt install build-essential pkg-config libssl-dev

# RHEL/CentOS
sudo yum groupinstall "Development Tools"
sudo yum install openssl-devel

# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

## systemd Service (Linux)

### 1. Build Release Binary

```bash
cargo build --release
sudo cp target/release/pear /usr/local/bin/
sudo chmod +x /usr/local/bin/pear
```

### 2. Create System User

```bash
sudo useradd --system --no-create-home --shell /bin/false pear
```

### 3. Create Configuration Directory

```bash
sudo mkdir -p /etc/pear
sudo cp pear.toml.example /etc/pear/pear.toml
sudo chown -R pear:pear /etc/pear
sudo chmod 600 /etc/pear/pear.toml
```

### 4. Create systemd Service File

Create `/etc/systemd/system/pear.service`:

```ini
[Unit]
Description=Pear Server - WebAssembly-Powered Web Server
After=network.target
Wants=network-online.target

[Service]
Type=simple
User=pear
Group=pear
WorkingDirectory=/var/lib/pear
ExecStart=/usr/local/bin/pear start --config /etc/pear/pear.toml --foreground
Restart=on-failure
RestartSec=5s
StandardOutput=journal
StandardError=journal

# Security hardening
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/var/lib/pear
CapabilityBoundingSet=CAP_NET_BIND_SERVICE

# Resource limits
LimitNOFILE=1048576
LimitNPROC=512

[Install]
WantedBy=multi-user.target
```

### 5. Enable and Start Service

```bash
sudo systemctl daemon-reload
sudo systemctl enable pear
sudo systemctl start pear
sudo systemctl status pear
```

### 6. View Logs

```bash
# Follow logs
sudo journalctl -u pear -f

# View recent logs
sudo journalctl -u pear -n 100
```

## Docker Deployment

### 1. Create Dockerfile

```dockerfile
FROM rust:1.75-slim AS builder

WORKDIR /app
COPY . .

RUN cargo build --release

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/pear /usr/local/bin/pear
COPY --from=builder /app/pear.toml.example /etc/pear/pear.toml
COPY --from=builder /app/static /opt/pear/static

WORKDIR /opt/pear

EXPOSE 8080 8443 9000

CMD ["pear", "start", "--config", "/etc/pear/pear.toml", "--foreground"]
```

### 2. Build Image

```bash
docker build -t pear-server:0.3.0 .
```

### 3. Run Container

```bash
docker run -d \
  --name pear-server \
  -p 80:8080 \
  -p 443:8443 \
  -p 9000:9000 \
  -v $(pwd)/pear.toml:/etc/pear/pear.toml:ro \
  --restart unless-stopped \
  pear-server:0.3.0
```

### 4. Docker Compose

Create `docker-compose.yml`:

```yaml
version: '3.8'

services:
  pear:
    image: pear-server:0.3.0
    build: .
    ports:
      - "80:8080"
      - "443:8443"
      -"9000:9000"
    volumes:
      - ./pear.toml:/etc/pear/pear.toml:ro
      - pear-data:/var/lib/pear
    restart: unless-stopped
    environment:
      - RUST_LOG=info

volumes:
  pear-data:
```

Run with:
```bash
docker-compose up -d
```

## Cloud Platforms

### AWS EC2

1. Launch EC2 instance (t3.medium or larger)
2. Configure Security Group:
   - Port 80 (HTTP)
   - Port 443 (HTTPS)
   - Port 9000 (Dashboard, restrict to your IP)
3. Use systemd deployment method
4. Configure Elastic IP for static address

### Google Cloud Platform

1. Create Compute Engine VM
2. Configure firewall rules
3. Use systemd deployment
4. Set up Cloud Logging for log aggregation

### DigitalOcean

1. Create Droplet (2GB+ RAM)
2. Use one-click Docker installation
3. Deploy via Docker
4. Configure Floating IP

### Kubernetes

Example deployment:

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: pear-server
spec:
  replicas: 3
  selector:
    matchLabels:
      app: pear-server
  template:
    metadata:
      labels:
        app: pear-server
    spec:
      containers:
      - name: pear
        image: pear-server:0.3.0
        ports:
        - containerPort: 8080
        - containerPort: 8443
        - containerPort: 9000
        env:
        - name: RUST_LOG
          value: "info"
        volumeMounts:
        - name: config
          mountPath: /etc/pear
      volumes:
      - name: config
        configMap:
          name: pear-config
---
apiVersion: v1
kind: Service
metadata:
  name: pear-service
spec:
  type: LoadBalancer
  ports:
  - port: 80
    targetPort: 8080
    name: http
  - port: 443
    targetPort: 8443
    name: https
  selector:
    app: pear-server
```

## SSL/TLS Setup

### Automatic (Let's Encrypt)

Edit `pear.toml`:

```toml
[ssl]
auto_cert = true
email = "admin@example.com"
domains = ["example.com", "www.example.com"]
```

Ensure ports 80 and 443 are accessible for ACME challenge.

### Manual Certificates

1. Obtain certificates (e.g., from Let's Encrypt manually)
2. Place in `/etc/pear/certs/`
3. Configure paths in `pear.toml`

## Monitoring

### Prometheus Metrics

Pear Server exposes metrics on `/metrics` endpoint (planned for future release).

### Health Checks

```bash
# HTTP health check
curl http://localhost:8080/health

# Status check
pear status --format json
```

### Logging

```bash
# JSON structured logs
RUST_LOG=info pear start

# Export to file
pear start 2>&1 | tee /var/log/pear/server.log
```

## Scaling

### Vertical Scaling

Increase server resources:
- More CPU cores
- More RAM
- Faster storage (SSD/NVMe)

### Horizontal Scaling

Deploy multiple Pear Server instances behind a load balancer:

```
        [Load Balancer]
             |
      ┌──────┴──────┐
      │      │      │
  [Pear 1][Pear 2][Pear 3]
```

Use:
- HAProxy
- nginx
- AWS ALB
- Google Cloud Load Balancer

### Configuration Tuning

In `pear.toml`:

```toml
[cages]
# Increase replicas for high availability
default_replicas = 5

# Adjust memory limits based on workload
memory_limit_mb = 256
```

## Best Practices

1. **Always use HTTPS in production**
2. **Restrict dashboard access** (firewall or VPN)
3. **Enable verbose logging initially** to catch issues
4. **Monitor resource usage** and scale proactively
5. **Regular backups** of configuration
6. **Use configuration management** (Ansible, Terraform)
7. **Implement CI/CD** for deployments
8. **Test in staging** before production

## Troubleshooting

### High Memory Usage

```bash
# Check Cage pool stats
pear status

# Reduce replicas
pear config set cages.default_replicas 2

# Reduce memory limit
pear config set cages.memory_limit_mb 64
```

### Connection Refused

```bash
# Check if server is running
systemctl status pear

# Check firewall
sudo ufw status
sudo firewall-cmd --list-all

# Check port binding
netstat -tulpn | grep pear
```

### SSL Certificate Errors

```bash
# Check Let's Encrypt logs
sudo journalctl -u pear | grep -i acme

# Verify DNS points to server
dig +short example.com

# Test certificate renewal
pear config validate
```

---

**For more deployment options and support, see the [README](../README.md).**
