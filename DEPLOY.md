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

- `GET /` - Should return "ISO Game Server"
- `GET /create` - Should return "Create endpoint - TODO"
- `GET /join/:code` - Should return "Join endpoint - code: {code}"

## Current Status

✅ Basic Worker structure
✅ Durable Object structure
⏳ WebSocket support (next)
⏳ Game simulation integration (next)

