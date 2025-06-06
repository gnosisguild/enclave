# SigNoz OpenTelemetry Collector Setup

## Setup Instructions

### 1. Get SigNoz Cloud Credentials
- Sign up at https://signoz.io/teams/
- Get ingestion key from Settings > Ingestion Settings


### 2. Configure Environment
`cd signoz/deploy/docker`

`cp .env.example .env`

Edit .env with your actual credentials

### 3. Run Collector
`docker-compose -f docker-compose-cloud-only.yaml up -d`

----
### Self-Hosted Setup
1. run `install.sh` at `signoz/deploy/`
2. open SigNoz UI (Main Dashboard)
   URL: http://localhost:8080