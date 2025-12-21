# Pongo — Test Plan

## Setup

```bash
npm run build && npm run dev   # Local: http://localhost:8787
npm run deploy                 # Prod: https://pongo.rob-gilks.workers.dev
```

## Test Cases

### TC-01: Match Creation & Join
1. Click CREATE → verify 5-char code appears
2. Second browser: enter code, click JOIN
3. Both see game start, ball moving

### TC-02: Paddle Movement  
1. Press Up/Down arrows (or W/S)
2. Verify paddle moves smoothly, stops at boundaries
3. Test touch buttons on mobile

### TC-03: Ball Physics
1. Ball bounces off walls (Y velocity reverses)
2. Ball bounces off paddles (X velocity reverses)
3. Hit top of paddle → ball deflects up
4. Moving paddle adds spin to ball
5. Speed increases per hit, caps at 16 u/s

### TC-04: Scoring
1. Miss ball → opponent scores
2. Ball resets to center
3. Score display updates immediately

### TC-05: Win Condition
1. Play to 11 points
2. Winner message appears
3. Game freezes

### TC-06: Network Sync
1. Both clients show identical game state
2. Own paddle responds instantly (client prediction)
3. No desync or jitter

### TC-07: Performance
- Client: 60+ FPS (target 120)
- Server: 60 Hz ticks
- Ping displayed in UI

## Pre-Deploy Checklist

```bash
npm run test:all   # ✓ Format, lint, tests pass
npm run build      # ✓ WASM builds
npm run dev        # ✓ Local test passes
```

- [ ] Match create/join works
- [ ] Paddle movement responsive  
- [ ] Ball physics correct
- [ ] Scoring works
- [ ] No console errors
