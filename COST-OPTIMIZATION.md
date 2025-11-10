# Cost Optimization Guide for Cloudflare Durable Objects

This document explains how ISO is configured to minimize Cloudflare costs while maintaining real-time gameplay.

## Current Configuration (Development)

### Client-Side Settings
- **Input Rate**: 30 Hz (33ms interval)
  - Location: `lobby_worker/src/lib.rs` line ~111
  - Reduced from 60 Hz to halve request costs
- **Input Coalescing**: Enabled
  - Only sends inputs when state changes
  - Further reduces requests by ~50-70% during idle gameplay
- **WebSocket Error Handling**: Proper cleanup to prevent reconnection loops

### Server-Side Settings (Durable Object)
- **Tick Rate**: 5 Hz (200ms interval) 
  - Location: `server_do/src/lib.rs` line ~229
  - Production recommendation: 20 Hz (50ms)
- **Snapshot Rate**: 5 Hz (every tick, no throttle)
  - Location: `server_do/src/lib.rs` line 74 (`snapshot_throttle: 1`)
  - Can be increased to 2-3 to reduce broadcasts
- **Alarm Behavior**: 
  - Stops when no clients connected (saves duration costs)
  - Starts on first player join
- **Client Cleanup**: Automatic on disconnect/error

## Cloudflare Billing Model for DOs

### Request Costs (Paid Plan: $5/mo base)
- **Included**: 1,000,000 requests/month
- **Overage**: $0.15/million requests
- **WebSocket Discount**: 20:1 (100 incoming messages = 5 billed requests)
- **Outgoing Messages**: FREE (server→client broadcasts)

### Duration Costs
- **Included**: 400,000 GB-seconds/month
- **Overage**: $12.50/million GB-seconds
- **Rate**: 0.128 GB-s per active second (128 MB standard)
- **Optimization**: Use WebSocket Hibernation (not yet implemented)

## Cost Calculations

### Single 10-Minute Match (6 players)
**Requests (with current settings):**
- Input messages: 6 players × 30 Hz × 600s = 108,000 messages
- After 20:1 discount: **5,400 billed requests**
- Outgoing snapshots: 5 Hz × 600s × 6 players = 18,000 messages → **FREE**

**Duration:**
- Active time: 600 seconds
- GB-seconds: 600 × 0.128 = **76.8 GB-s**

**Total cost for one match: ~$0.001 (virtually free)**

### Monthly Estimates (Paid Plan: $5/mo)

#### Light Usage (10 matches/day, 6 players avg)
- **Requests/month**: 5,400 × 10 × 30 = 1,620,000 → Overage: $0.09
- **Duration/month**: 76.8 × 10 × 30 = 23,040 GB-s → Included
- **Total**: $5.09/month

#### Medium Usage (50 matches/day, 6 players avg)
- **Requests/month**: 5,400 × 50 × 30 = 8,100,000 → Overage: $1.07
- **Duration/month**: 76.8 × 50 × 30 = 115,200 GB-s → Included
- **Total**: $6.07/month

#### Heavy Usage (200 matches/day, 6 players avg)
- **Requests/month**: 5,400 × 200 × 30 = 32,400,000 → Overage: $4.71
- **Duration/month**: 76.8 × 200 × 30 = 460,800 GB-s → Overage: $0.76
- **Total**: $10.47/month

## Free Tier Limits (Development)

### Daily Limits
- **Requests**: 100,000/day (unbilled)
- **Duration**: No explicit daily limit (but limited by request quota)

### Free Tier Capacity
With current settings (30 Hz input, 5 Hz snapshots):
- **Matches/day**: ~18 full matches (6 players, 10 min each)
- **Player-minutes/day**: ~1,080 player-minutes

## Optimization Recommendations

### To Stay on Free Tier (Development)
1. ✅ **Input rate**: 30 Hz (current)
2. ✅ **Input coalescing**: Enabled (current)
3. ✅ **Alarm stops when idle**: Enabled (current)
4. ⚠️ **Reduce tick rate**: Consider 3 Hz (333ms) for testing
5. ⚠️ **Increase snapshot throttle**: Set to 2-3 for testing

### To Minimize Paid Tier Costs (Production)
1. ✅ Keep input rate at 30 Hz
2. ✅ Keep input coalescing enabled
3. ⚠️ **Increase tick rate**: 20 Hz (50ms) for responsive gameplay
4. ⚠️ **Implement WebSocket Hibernation**: Save duration costs on idle lobbies
5. ⚠️ **Add idle match timeout**: Close DOs after 5 minutes of inactivity
6. ⚠️ **Optimize snapshot size**: Use delta encoding (future)

## Configuration Changes

### To Reduce Free Tier Usage (Testing)
Edit `server_do/src/lib.rs`:

```rust
// Line ~229: Increase alarm interval
let tick_interval_ms = 333; // 3 Hz instead of 5 Hz

// Line 74: Increase snapshot throttle
snapshot_throttle: 2, // Send every other tick (2.5 Hz snapshots)
```

Edit `lobby_worker/src/lib.rs`:

```javascript
// Line ~111: Reduce input rate
}, 50); // 20 Hz instead of 30 Hz
```

### For Production (Responsive Gameplay)
Edit `server_do/src/lib.rs`:

```rust
// Line ~229: Production tick rate
let tick_interval_ms = 50; // 20 Hz for smooth gameplay

// Line 74: No snapshot throttle
snapshot_throttle: 1, // Send every tick
```

## Monitoring Costs

### Check Cloudflare Dashboard
1. Go to: https://dash.cloudflare.com
2. Navigate to: Workers & Pages → iso → Metrics
3. Monitor:
   - Requests (with WebSocket discount applied)
   - Duration (GB-seconds)
   - Durable Objects count

### Check Logs for Issues
```bash
npm run logs:check
```

Look for:
- Repeated error patterns (could indicate retry loops)
- High alarm frequency (should be ~5/sec when active, 0 when idle)
- WebSocket connection/disconnection patterns

## Troubleshooting High Costs

### If You Hit Free Tier Limit
1. Check logs for error loops: `npm run logs:check`
2. Verify alarm stops when no clients: Look for "No clients, stopping alarm loop"
3. Verify clients disconnect properly: Look for "WebSocket close event"
4. Reduce tick/input rates as shown above

### Common Issues That Cause High Costs
- ❌ **Error retry loops**: Fixed with proper error handling
- ❌ **Alarms running when no clients**: Fixed with client count check
- ❌ **High input rate**: Fixed at 30 Hz with coalescing
- ❌ **Failed WebSocket cleanup**: Fixed with proper close handlers
- ❌ **Development testing loops**: Be mindful of automated testing

## Future Optimizations

### Not Yet Implemented
1. **WebSocket Hibernation**: Reduce duration costs on idle lobbies
2. **Delta Encoding**: Reduce snapshot sizes (less bandwidth, faster)
3. **Idle Match Timeout**: Auto-close DOs after N minutes inactive
4. **Rate Limiting**: Prevent abuse (client-side input limiting)
5. **Match Expiry**: Auto-delete old match DOs to reduce storage

### When to Consider Alternative Hosting
- **>$50/month on Cloudflare**: Consider hybrid approach
- **Hundreds of concurrent matches 24/7**: Consider VPS/Fly.io
- **Need predictable costs**: Consider flat-rate VPS ($5-20/month)

---

**Last Updated**: 2025-11-10
**Configuration Version**: M2 (Development)

