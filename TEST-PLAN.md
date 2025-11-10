# PONG Test Plan

This document outlines manual test procedures for the Pong game. Tests are performed using browser automation tools.

---

## Test Environment Setup

### Local Testing

```bash
# Build and start local server
npm run build
npm run dev
```

Server runs at `http://localhost:8787`

### Production Testing

```bash
# Deploy to Cloudflare
npm run deploy
```

Production URL: `https://iso.rob-gilks.workers.dev`

---

## Test Categories

### 1. Connection & Lobby Tests

#### TC-001: Create Match

**Objective**: Verify match creation endpoint

**Steps**:

1. Navigate to `http://localhost:8787`
2. Click "Create Match" button
3. Verify 5-character code is displayed
4. Verify "Waiting for opponent..." message

**Expected**:

- Code is exactly 5 characters
- Code uses valid Base32 characters (no vowels/ambiguous)
- Page shows shareable link

#### TC-002: Join Match

**Objective**: Verify two players can join the same match

**Steps**:

1. Player 1 creates a match, gets code
2. Player 2 opens new browser window
3. Player 2 navigates to join URL with code
4. Verify both players see "Game starting..."

**Expected**:

- Player 2 successfully joins
- Both players receive player ID (0 or 1)
- Game initializes when both players connected

#### TC-003: Invalid Code

**Objective**: Verify error handling for invalid codes

**Steps**:

1. Try to join with invalid code "AAAAA"
2. Verify error message

**Expected**:

- Clear error message "Match not found"

---

### 2. Paddle Movement Tests

#### TC-004: Left Paddle Movement

**Objective**: Verify left player can move paddle up and down

**Steps**:

1. Create and join match as left player
2. Press Up arrow key
3. Verify paddle moves up
4. Press Down arrow key
5. Verify paddle moves down

**Expected**:

- Paddle moves smoothly at 8 units/second
- Paddle stays within arena bounds (top/bottom)
- Movement is responsive

#### TC-005: Right Paddle Movement

**Objective**: Verify right player can move paddle

**Steps**:

1. Join match as right player (second joiner)
2. Press Up/Down keys
3. Verify paddle movement

**Expected**:

- Right paddle moves correctly
- Independent from left paddle

#### TC-006: Paddle Bounds

**Objective**: Verify paddles can't leave the arena

**Steps**:

1. Hold Up arrow until paddle reaches top
2. Continue holding, verify paddle stops at boundary
3. Hold Down arrow until paddle reaches bottom
4. Verify paddle stops at boundary

**Expected**:

- Paddle Y position clamped to arena height
- No glitches or errors at boundaries

---

### 3. Ball Physics Tests

#### TC-007: Ball Initial State

**Objective**: Verify ball starts correctly

**Steps**:

1. Start new game with 2 players
2. Observe ball position and velocity

**Expected**:

- Ball starts at center (16, 12)
- Ball has random initial direction
- Ball moves at 8 units/second

#### TC-008: Wall Bounce

**Objective**: Verify ball bounces off top and bottom walls

**Steps**:

1. Let ball travel to top wall
2. Verify ball bounces (velocity.y reverses)
3. Let ball travel to bottom wall
4. Verify ball bounces

**Expected**:

- Ball Y velocity reverses on wall hit
- Ball X velocity unchanged
- No ball stuck in wall

#### TC-009: Paddle Bounce

**Objective**: Verify ball bounces off paddles

**Steps**:

1. Position paddle to intercept ball
2. Let ball hit paddle
3. Verify ball bounces back

**Expected**:

- Ball X velocity reverses
- Ball speed increases slightly (1.05x)
- Ball influenced by paddle velocity

#### TC-010: Ball Speed Cap

**Objective**: Verify ball doesn't exceed max speed

**Steps**:

1. Play long rally (10+ paddle hits)
2. Observe ball speed

**Expected**:

- Ball speed caps at 16 units/second
- No infinite acceleration

---

### 4. Scoring Tests

#### TC-011: Left Player Scores

**Objective**: Verify scoring when ball exits right edge

**Steps**:

1. Right player misses ball
2. Ball exits right edge (x > 32)
3. Verify left player score increases

**Expected**:

- Left score increments by 1
- Score display updates
- Ball resets to center

#### TC-012: Right Player Scores

**Objective**: Verify scoring when ball exits left edge

**Steps**:

1. Left player misses ball
2. Ball exits left edge (x < 0)
3. Verify right player score increases

**Expected**:

- Right score increments by 1
- Score display updates
- Ball resets to center

#### TC-013: Ball Reset After Score

**Objective**: Verify ball state after scoring

**Steps**:

1. Score a point
2. Observe ball position and velocity

**Expected**:

- Ball returns to center (16, 12)
- Ball velocity resets to 8 u/s
- Random direction chosen

---

### 5. Win Condition Tests

#### TC-014: Game Ends at 11 Points

**Objective**: Verify game ends when player reaches 11

**Steps**:

1. Play until one player reaches 11 points
2. Verify game over screen

**Expected**:

- Game stops when score reaches 11
- Winner displayed
- "Game Over" message shown

#### TC-015: Winner Display

**Objective**: Verify correct winner is displayed

**Steps**:

1. Let left player win (11 points)
2. Verify "Left Player Wins!" message
3. Start new game
4. Let right player win
5. Verify "Right Player Wins!" message

**Expected**:

- Correct winner displayed
- Scores frozen
- Rematch option available

---

### 6. Network Synchronization Tests

#### TC-016: State Sync Between Clients

**Objective**: Verify both clients see same game state

**Steps**:

1. Open two browser windows side by side
2. Player 1 moves paddle up
3. Verify Player 2 sees paddle movement
4. Hit ball and verify both see same ball position

**Expected**:

- Both clients show identical game state
- No desynchronization
- Latency < 100ms

#### TC-017: Network Interruption

**Objective**: Verify handling of network issues

**Steps**:

1. Start game
2. Simulate network lag (browser dev tools)
3. Continue playing

**Expected**:

- Game continues with minor latency
- State catches up when connection restored
- No crashes or freezes

#### TC-018: Player Disconnect

**Objective**: Verify handling when player disconnects

**Steps**:

1. Start game with 2 players
2. Player 1 closes browser tab
3. Verify Player 2 sees disconnect message

**Expected**:

- Disconnection detected within 5 seconds
- Remaining player sees message
- Match ends or waits for reconnect

---

### 7. Performance Tests

#### TC-019: Frame Rate

**Objective**: Verify client runs at 60 fps

**Steps**:

1. Start game
2. Open browser performance tools
3. Play for 1 minute
4. Check frame rate

**Expected**:

- Consistent 60 fps
- No frame drops during normal play
- CPU usage reasonable

#### TC-020: Server Tick Rate

**Objective**: Verify server runs at 60 Hz

**Steps**:

1. Check server logs during game
2. Verify tick timing

**Expected**:

- Server ticks every 16.67ms
- No tick delays or skips
- State broadcasts at 60 Hz

---

### 8. Mobile Tests

#### TC-021: Touch Controls

**Objective**: Verify on-screen controls work on mobile

**Steps**:

1. Open game on mobile device
2. Tap "Up" button
3. Verify paddle moves up
4. Tap "Down" button
5. Verify paddle moves down

**Expected**:

- Touch buttons responsive
- Paddle movement smooth
- No input lag

#### TC-022: Mobile Performance

**Objective**: Verify game runs well on mobile

**Steps**:

1. Play full game on mobile device
2. Monitor performance

**Expected**:

- 60 fps maintained
- No overheating
- Battery usage reasonable

---

### 9. Edge Case Tests

#### TC-023: Ball Corner Hit

**Objective**: Verify ball behavior at corner

**Steps**:

1. Let ball hit corner of paddle
2. Observe bounce angle

**Expected**:

- Ball bounces at reasonable angle
- No getting stuck
- No unexpected physics

#### TC-024: Simultaneous Input

**Objective**: Verify both players can input simultaneously

**Steps**:

1. Both players press Up at same time
2. Verify both paddles move

**Expected**:

- No input conflicts
- Both paddles respond
- Server handles concurrent inputs

#### TC-025: Rapid Input Changes

**Objective**: Verify rapid key presses handled correctly

**Steps**:

1. Rapidly alternate Up/Down keys
2. Verify paddle responds correctly

**Expected**:

- Paddle doesn't glitch
- All inputs processed
- Smooth movement

---

## Automated Test Script

For browser automation, use the following sequence:

```javascript
// Example test script (pseudo-code)
1. navigate('http://localhost:8787')
2. click('#create-match')
3. waitFor('.match-code')
4. copyCode()
5. openNewTab()
6. navigate(`http://localhost:8787/join/${code}`)
7. waitFor('.game-canvas')
8. pressKey('ArrowUp', duration: 500ms)
9. snapshot()
10. verifyPaddlePosition(y < initial_y)
```

---

## Test Execution Checklist

Before each commit/deploy:

- [ ] All connection tests pass
- [ ] Paddle movement tests pass
- [ ] Ball physics tests pass
- [ ] Scoring tests pass
- [ ] Win condition tests pass
- [ ] Network sync tests pass
- [ ] Performance acceptable
- [ ] No console errors

---

## Known Issues / Limitations

(Document any known issues here as they're discovered)

---

**Last Updated**: 2025-11-10
**Status**: Initial test plan for Pong branch
