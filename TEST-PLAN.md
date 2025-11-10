# PONG Test Plan

Manual test procedures for the Pong game, executed via browser automation.

---

## Test Environment

### Local Testing

```bash
npm run build
npm run dev
# Server at http://localhost:8787
```

### Production Testing

```bash
npm run deploy
# Production at https://iso.rob-gilks.workers.dev
```

---

## Core Test Cases

### TC-001: Match Creation & Join

**Objective**: Verify two players can create and join a match

**Steps**:

1. Navigate to `http://localhost:8787`
2. Click "Create Match"
3. Verify 5-character code displayed
4. Copy code
5. Open second browser window
6. Join with code
7. Verify both players see "Game starting..."

**Expected**:

- Code is exactly 5 characters (Base32, no vowels)
- Both players receive player ID (0 or 1)
- Game initializes when both connected

---

### TC-002: Paddle Movement

**Objective**: Verify paddles move correctly with input

**Steps**:

1. Join match as left player
2. Press Up arrow key
3. Verify paddle moves up
4. Press Down arrow key
5. Verify paddle moves down
6. Hold Up until paddle reaches top boundary
7. Verify paddle stops at boundary
8. Repeat for right player

**Expected**:

- Paddles move smoothly at 8 units/second
- Paddles stay within arena bounds (Y: 2 to 22)
- Movement is responsive (< 100ms latency)
- Independent control for each player

---

### TC-003: Ball Physics

**Objective**: Verify ball movement and collisions

**Steps**:

1. Start game with 2 players
2. Observe ball initial position (center: 16, 12)
3. Let ball hit top wall
4. Verify ball bounces (velocity.y reverses)
5. Position paddle to intercept ball
6. Let ball hit paddle
7. Verify ball bounces back
8. Verify ball speed increases slightly
9. Continue rally, observe ball speed caps at 16 u/s

**Expected**:

- Ball starts at center with 8 u/s velocity
- Random initial direction
- Wall bounce: Y velocity reverses
- Paddle bounce: X velocity reverses, speed increases 1.05x
- Speed capped at 16 units/second
- No ball stuck in walls or paddles

---

### TC-004: Scoring

**Objective**: Verify scoring logic works correctly

**Steps**:

1. Right player misses ball
2. Ball exits right edge (x > 32)
3. Verify left score increments
4. Verify ball resets to center
5. Left player misses ball
6. Ball exits left edge (x < 0)
7. Verify right score increments
8. Verify ball resets

**Expected**:

- Correct player score increments
- Score display updates immediately
- Ball returns to center (16, 12)
- Ball velocity resets to 8 u/s
- Random direction chosen for new ball

---

### TC-005: Win Condition

**Objective**: Verify game ends at 11 points

**Steps**:

1. Play until one player reaches 11 points
2. Verify game over screen appears
3. Verify correct winner displayed
4. Verify scores frozen

**Expected**:

- Game stops when score reaches 11
- Winner message: "Left Player Wins!" or "Right Player Wins!"
- Game state freezes (no more ball movement)

---

### TC-006: Network Synchronization

**Objective**: Verify both clients see same game state

**Steps**:

1. Open two browser windows side by side
2. Player 1 moves paddle up
3. Verify Player 2 sees paddle movement
4. Hit ball, verify both see same ball position
5. Score point, verify both see score update

**Expected**:

- Both clients show identical game state
- No desynchronization
- Latency < 100ms
- State updates at 60 Hz

---

### TC-007: Player Disconnect

**Objective**: Verify handling when player disconnects

**Steps**:

1. Start game with 2 players
2. Player 1 closes browser tab
3. Verify Player 2 sees disconnect indication
4. Verify game stops or waits for reconnect

**Expected**:

- Disconnection detected within 5 seconds
- Remaining player notified
- Server cleans up disconnected player
- No memory leaks or orphaned connections

---

### TC-008: Performance

**Objective**: Verify client and server performance

**Steps**:

1. Start game
2. Open browser performance tools
3. Play for 2 minutes
4. Check frame rate (should be 60 fps)
5. Check server logs for tick timing

**Expected**:

- Client: Consistent 60 fps, no frame drops
- Server: Ticks every 16.67ms (60 Hz)
- No memory leaks
- CPU usage reasonable

---

## Edge Cases

### TC-009: Corner Collision

**Objective**: Verify ball behavior at corner

**Steps**:

1. Let ball hit corner of paddle (top or bottom edge)
2. Observe bounce angle

**Expected**:

- Ball bounces at reasonable angle
- No getting stuck
- No unexpected physics glitches

---

### TC-010: Simultaneous Input

**Objective**: Verify both players can input simultaneously

**Steps**:

1. Both players press Up at same time
2. Verify both paddles move independently

**Expected**:

- No input conflicts
- Both paddles respond correctly
- Server handles concurrent inputs

---

### TC-011: Rapid Input Changes

**Objective**: Verify rapid key presses handled correctly

**Steps**:

1. Rapidly alternate Up/Down keys
2. Verify paddle responds smoothly

**Expected**:

- Paddle doesn't glitch or jump
- All inputs processed correctly
- Smooth movement despite rapid changes

---

## Automated Test Script (Pseudocode)

```javascript
// Browser automation sequence
1. navigate('http://localhost:8787')
2. click('#create-match')
3. waitFor('.match-code')
4. code = getText('.match-code')
5. openNewTab()
6. navigate(`http://localhost:8787/join/${code}`)
7. waitFor('.game-canvas')
8. pressKey('ArrowUp', duration: 500ms)
9. snapshot()
10. verifyPaddlePosition(y < initial_y)
11. waitForBall()
12. verifyBallMoving()
```

---

## Pre-Deployment Checklist

Before each commit/deploy:

- [ ] All unit tests pass (`npm run test`)
- [ ] Linting passes (`npm run clippy`)
- [ ] Formatting correct (`npm run fmt`)
- [ ] Match creation works
- [ ] Two players can join
- [ ] Paddle movement responsive
- [ ] Ball physics correct
- [ ] Scoring works
- [ ] Win condition triggers
- [ ] No console errors
- [ ] Performance acceptable (60 fps client, 60 Hz server)

---

## Known Issues

(Document any known issues here as they're discovered)

---

**Last Updated**: 2025-11-10  
**Status**: Core test plan for Pong
