# Deployment Guide

## Prerequisites

1. **Install Wrangler CLI**:
   ```bash
   npm install -g wrangler
   ```

2. **Login to Cloudflare**:
   ```bash
   wrangler login
   ```

## Local Development

Test locally before deploying:

```bash
wrangler dev
```

This will:
- Start a local development server
- Compile the Rust code to WebAssembly
- Make the worker available at `http://localhost:8787`

## Deploy to Cloudflare

Once local testing passes:

```bash
wrangler publish
```

This will:
- Build the worker
- Deploy to Cloudflare's global network
- Make it available at `https://iso.<your-subdomain>.workers.dev`

## Testing the Deployment

After deployment, test the endpoints:

- `GET /` - Returns "ISO Game Server"
- `GET /create` - Returns "Match created: {5-char code}"
- `GET /join/:code` - Returns "Match {code} found. Connect via WebSocket to join."

**Automated testing:**
```bash
npm run deploy:test
```

This script automatically:
- Deploys to Cloudflare
- Tests all endpoints
- Checks logs for errors

## Current Status

✅ Basic Worker structure
✅ Durable Object structure
✅ WebSocket support
✅ Game simulation integrated
✅ Network protocol (C2S/S2C)
✅ Player joining and snapshot broadcasting

