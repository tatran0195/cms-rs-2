# CMS Deployment Guide

## Windows Deployment (Single Machine)

This guide covers deploying CMS to a single AWS Windows machine as specified in the architecture requirements.

---

## 📋 Prerequisites

### 1. System Requirements

- Windows Server 2019 or later (recommended: Windows Server 2022)
- Minimum 4GB RAM (recommended: 8GB+ for production)
- Minimum 2 vCPUs (recommended: 4+ vCPUs)
- Minimum 50GB disk space (SSD recommended)
- Static IP address or Elastic IP

### 2. Software Requirements

- [Rust](https://www.rust-lang.org/tools/install) 1.70+ (MSVC toolchain)
- [Node.js](https://nodejs.org/) 18+ (for frontend build)
- [PostgreSQL](https://www.postgresql.org/) 14+ (or managed PostgreSQL)
- [NSSM](https://nssm.cc/download) (Non-Sucking Service Manager)
- [Git](https://git-scm.com/) (optional, for source control)

---

## 🚀 Deployment Steps

### Step 1: Prepare the Server

1. **Launch EC2 Instance**
   - AMI: Windows Server 2022 Base
   - Instance Type: t3.medium or larger
   - Storage: 50GB+ SSD (gp3 recommended)
   - Security Group: Open ports 80, 443, 22 (SSH optional)

2. **Connect to Instance**

   ```powershell
   # Use RDP or Session Manager
   mstsc /v:your-ec2-public-ip
   ```

3. **Install Prerequisites**

   ```powershell
   # Install Rust (MSVC toolchain)
   winget install --id Rustlang.Rust

   # Or use rustup
   rustup-init -y --default-toolchain stable -t x86_64-pc-windows-msvc

   # Install Node.js
   winget install --id OpenJS.NodeJS

   # Install PostgreSQL (or use RDS)
   winget install --id PostgreSQL.PostgreSQL

   # Install NSSM
   choco install nssm
   # Or download from: https://nssm.cc/download
   ```

---

### Step 2: Clone and Build

1. **Clone Repository**

   ```powershell
   cd C:\apps
   git clone https://github.com/your-org/cms.git
   cd cms
   ```

2. **Configure Environment**
   - Copy `config/dev.env` to `config/deploy.env`
   - Edit `config/deploy.env` with production values:
     ```env
     CMS_ENV=deploy
     CMS_SERVER__PORT=80
     CMS_DATABASE__URL=postgres://user:password@localhost:5432/cms
     CMS_AUTH__JWT_SECRET=your-jwt-secret-here
     CMS_AUTH__SESSION_SECRET=your-session-secret-here
     CMS_STORAGE__BACKEND=local
     CMS_STORAGE__LOCAL_ROOT=C:\\apps\\cms\\storage
     ```

3. **Build Backend**

   ```powershell
   cargo build --release
   ```

4. **Build Frontend**
   ```powershell
   cd frontend
   npm install
   npm run build
   cd ..
   ```

---

### Step 3: Configure PostgreSQL

1. **Create Database**

   ```sql
   CREATE DATABASE cms;
   CREATE USER cms_user WITH PASSWORD 'secure-password';
   GRANT ALL PRIVILEGES ON DATABASE cms TO cms_user;
   ```

2. **Run Migrations**
   ```powershell
   # Migrations will run automatically on first startup
   # Or manually:
   sqlx migrate run --database-url postgres://cms_user:secure-password@localhost:5432/cms
   ```

---

### Step 4: Install as Windows Service

1. **Copy Files**

   ```powershell
   # Create deployment directory
   mkdir C:\\apps\\cms\\deploy

   # Copy binary
   copy target\\release\\cms-server.exe C:\\apps\\cms\\deploy\\

   # Copy config
   mkdir C:\\apps\\cms\\config
   copy config\\deploy.env C:\\apps\\cms\\config\\

   # Copy frontend
   xcopy /E /Y dist\\frontend C:\\apps\\cms\\deploy\\frontend\\
   ```

2. **Install Service**

   ```powershell
   cd deploy
   nssm-install.bat
   ```

   Or manually:

   ```powershell
   nssm install CMSServer "C:\apps\cms\deploy\cms-server.exe"
   nssm set CMSServer AppDirectory "C:\apps\cms\deploy"
   nssm set CMSServer AppEnvironmentExtra CMS_ENV=deploy
   nssm set CMSServer AppEnvironmentExtra CMS_CONFIG_PATH="C:\apps\cms\config\deploy.env"
   nssm set CMSServer Start SERVICE_AUTO_START
   ```

3. **Start Service**

   ```powershell
   nssm start CMSServer
   ```

4. **Verify Service**
   ```powershell
   nssm status CMSServer
   nssm logs CMSServer
   ```

---

### Step 5: Configure Reverse Proxy (Optional)

For production, use IIS or Nginx as a reverse proxy:

#### IIS Configuration

```powershell
# Install IIS
Install-WindowsFeature -Name Web-Server -IncludeManagementTools

# Install URL Rewrite Module
Install-WindowsFeature -Name Web-URL-Rewrite

# Configure site in IIS Manager:
# - Bindings: http:80, https:443
# - Reverse Proxy to localhost:3000 (or your configured port)
```

#### Nginx Configuration (if using Windows Nginx)

```nginx
server {
    listen 80;
    server_name your-domain.com;

    location / {
        proxy_pass http://localhost:3000;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }

    # Static files
    location /assets/ {
        alias C:/apps/cms/deploy/frontend/assets/;
        expires 1y;
        add_header Cache-Control "public, max-age=31536000, immutable";
    }
}
```

---

## 📊 Configuration Reference

### Environment Variables

| Variable                              | Description                | Required | Default   |
| ------------------------------------- | -------------------------- | -------- | --------- |
| `CMS_ENV`                             | Environment (dev/deploy)   | Yes      | dev       |
| `CMS_SERVER__PORT`                    | Server port                | Yes      | 3000      |
| `CMS_SERVER__HOST`                    | Server host                | No       | 0.0.0.0   |
| `CMS_DATABASE__URL`                   | PostgreSQL connection URL  | Yes      | -         |
| `CMS_AUTH__JWT_SECRET`                | JWT secret key             | Yes      | -         |
| `CMS_AUTH__SESSION_SECRET`            | Session secret key         | Yes      | -         |
| `CMS_STORAGE__BACKEND`                | Storage backend (local/s3) | No       | local     |
| `CMS_STORAGE__LOCAL_ROOT`             | Local storage path         | No       | ./storage |
| `CMS_RATE_LIMIT__ENABLED`             | Enable rate limiting       | No       | true      |
| `CMS_RATE_LIMIT__REQUESTS_PER_SECOND` | Rate limit RPS             | No       | 100       |
| `CMS_RATE_LIMIT__BURST_SIZE`          | Rate limit burst           | No       | 200       |

### Storage Configuration

For S3 storage:

```env
CMS_STORAGE__BACKEND=s3
CMS_STORAGE__S3_ENDPOINT=https://s3.amazonaws.com
CMS_STORAGE__S3_REGION=us-east-1
CMS_STORAGE__S3_BUCKET=your-bucket
CMS_STORAGE__S3_ACCESS_KEY=your-access-key
CMS_STORAGE__S3_SECRET_KEY=your-secret-key
```

---

## 🔧 Service Management

### Start Service

```powershell
nssm start CMSServer
```

### Stop Service

```powershell
nssm stop CMSServer
```

### Restart Service

```powershell
nssm restart CMSServer
```

### Check Status

```powershell
nssm status CMSServer
```

### View Logs

```powershell
nssm logs CMSServer
```

### Uninstall Service

```powershell
nssm remove CMSServer confirm
```

---

## 📝 Monitoring

### Health Check

```powershell
curl http://localhost:3000/health
# Expected response: OK
```

### Metrics (if Prometheus enabled)

```powershell
# Add prometheus feature to cms-middleware
# Then access: http://localhost:9090/metrics
```

### Log Files

- Application logs: Use `nssm logs CMSServer`
- Windows Event Log: Check Application log in Event Viewer

---

## 🔄 Updates

### Update Backend

```powershell
cd C:\apps\cms
git pull
cargo build --release
nssm stop CMSServer
copy target\release\cms-server.exe C:\apps\cms\deploy\\
nssm start CMSServer
```

### Update Frontend

```powershell
cd C:\apps\cms\frontend
npm install
npm run build
xcopy /E /Y dist\frontend C:\apps\cms\deploy\frontend\\
```

---

## 🚨 Troubleshooting

### Service Won't Start

1. Check service logs: `nssm logs CMSServer`
2. Check Windows Event Log
3. Try running manually: `cms-server.exe`
4. Verify config file exists and is valid

### Database Connection Failed

1. Verify PostgreSQL is running
2. Check connection string in config
3. Test connection: `psql -h localhost -U cms_user -d cms`
4. Verify firewall allows connections

### Port Already in Use

1. Check for running service: `nssm status CMSServer`
2. Stop existing service: `nssm stop CMSServer`
3. Check for other processes: `netstat -ano | findstr :3000`

### Frontend Not Loading

1. Verify frontend files exist in deploy directory
2. Check browser console for errors
3. Verify static file paths in config

---

## 📚 Architecture Notes

### Deployment Model

- **Single AWS Windows machine** (no Docker)
- **In-process auth** (no GoTrue)
- **Process-local rate limiting** (appropriate for single machine)
- **Pluggable backends** (local/S3, pgvector/Qdrant, memory/Redis)

### Scaling

This deployment is designed for a single machine. For horizontal scaling:

- Use Redis backend for job queue
- Use Redis or distributed rate limiter
- Use load balancer with multiple instances
- Configure shared storage (S3)

---

## 📞 Support

For issues with deployment:

1. Check the [CODING_PROGRESS.md](../CODING_PROGRESS.md) for implementation details
2. Review the [WORKSPACE_AUDIT.md](../WORKSPACE_AUDIT.md) for architecture decisions
3. Check the [architecture documents](../../uploads/) for design decisions
