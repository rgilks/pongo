# Development Workflow

This document outlines the standard development workflow for ISO, following the project's principles of small, tested changes.

## Quick Reference

```bash
# Full workflow (recommended)
npm run test:all          # Format, lint, test
npm run build             # Build client + server
npm run dev               # Start local dev server
npm run deploy:test       # Deploy + test + check logs
```

## Standard Development Cycle

### 1. Make Changes

- Keep changes small and focused
- Test each step as you go
- Follow coding principles (functions < 20 lines, files < 200 lines)

### 2. Fix Linting Errors and Run Tests

**Automated (recommended):**
```bash
npm run test:all
```

This runs:
- `cargo fmt --check` - Format check
- `cargo clippy --workspace -- -D warnings` - Linting
- `cargo test --workspace` - All tests

**Manual alternative:**
```bash
cargo fmt                 # Format code
cargo clippy --workspace -- -D warnings  # Lint
cargo test --workspace    # Test
```

**Fix any errors before proceeding.**

### 3. Local Testing

**Build the project:**
```bash
npm run build
```

**Start local dev server:**
```bash
npm run dev
# Server runs at http://localhost:8787
```

**Follow TEST-PLAN.md:**
- Test relevant endpoints locally
- Test WebSocket connections
- Verify rendering and game functionality
- Test in browser at `http://localhost:8787`

**Benefits of local testing:**
- No rate limits
- Faster iteration
- Better debugging (terminal logs)
- Isolated from production

### 4. Deploy, Test, and Check Logs

**Automated (recommended):**
```bash
npm run deploy:test
```

This script:
- Deploys to Cloudflare Workers
- Tests endpoints (`/`, `/create`, `/join/:code`)
- Checks logs for errors (10 seconds)

**Manual alternative:**
```bash
# Deploy
npx wrangler deploy

# Test endpoints
curl https://iso.rob-gilks.workers.dev/
curl https://iso.rob-gilks.workers.dev/create
curl https://iso.rob-gilks.workers.dev/join/CODE

# Check logs
npm run logs:check
```

**Verify:**
- All endpoints respond correctly
- WebSocket connections work
- No errors in logs

### 5. Documentation Updates

Update relevant documentation:

- **README.md** - User-facing features, setup, quick start
- **TEST-PLAN.md** - Test procedures, verification steps
- **SPEC.md** - Architecture changes, new features
- **COMPLETION-PLAN.md** - Progress tracking, milestone status

**When to update:**
- New features added
- Workflow changes
- Bug fixes that affect behavior
- Status changes (milestone completion)

### 6. Commit and Push

**Only commit after all steps succeed:**
- ✅ All tests pass
- ✅ Local testing successful
- ✅ Deployment successful
- ✅ No errors in logs
- ✅ Documentation updated

**Commit message format:**
```
Short summary (50 chars or less)

Detailed explanation of what changed and why.
- Bullet points for multiple changes
- Reference issues if applicable
```

**Example:**
```bash
git add -A
git commit -m "Optimize request volume and fix WebSocket connection

- Reduce alarm frequency from 50ms to 200ms (75% reduction)
- Stop alarms when no clients connected
- Fix RefCell borrow across await point
- Improve error handling with rate limit detection

Request reduction: ~72k/hour → ~18k/hour"
git push
```

## Pre-commit Hook

The project includes a pre-commit hook (`.githooks/pre-commit`) that automatically runs:

1. `cargo fmt --check` - Format verification
2. `cargo clippy --workspace -- -D warnings` - Linting
3. `cargo test --workspace` - Tests

**Setup (one-time):**
```bash
git config core.hooksPath .githooks
```

The hook prevents commits if any check fails, ensuring code quality.

## Common Workflows

### Adding a New Feature

1. Create feature branch (optional)
2. Make small, incremental changes
3. Run `npm run test:all` frequently
4. Test locally with `npm run dev`
5. Update documentation
6. Deploy and test: `npm run deploy:test`
7. Commit and push

### Fixing a Bug

1. Reproduce the bug locally
2. Write a test that fails (if applicable)
3. Fix the bug
4. Verify test passes
5. Run full test suite: `npm run test:all`
6. Test locally
7. Deploy and verify fix: `npm run deploy:test`
8. Update documentation if needed
9. Commit and push

### Refactoring

1. Ensure tests pass before refactoring
2. Make small, incremental changes
3. Run tests after each change
4. Verify behavior unchanged
5. Update documentation if structure changes
6. Deploy and test
7. Commit and push

## Troubleshooting

### Tests Fail

- Check error messages carefully
- Run specific test: `cargo test --package <package> --test <test_name>`
- Check for clippy warnings: `cargo clippy --workspace -- -D warnings`

### Local Dev Server Issues

- Ensure `npm run build` completed successfully
- Check port 8787 is not in use
- Delete `.wrangler/state/` to reset local state
- Check terminal for error messages

### Deployment Issues

- Verify you're logged in: `npx wrangler whoami`
- Check `wrangler.toml` configuration
- Review deployment logs
- Check Cloudflare dashboard for errors

### Rate Limits

- Use local development to avoid rate limits
- Optimize request frequency (see `server_do/src/lib.rs`)
- Consider upgrading to paid plan for production

## Best Practices

1. **Small Changes**: Make incremental progress, test frequently
2. **Test First**: Write tests when possible, especially for game logic
3. **Document As You Go**: Update docs with each significant change
4. **Review Before Commit**: Ensure all checks pass before committing
5. **Meaningful Commits**: One logical change per commit
6. **Clear Messages**: Describe what and why, not just what

## Project Structure

```
iso/
├── game_core/      # ECS, systems, components (shared client/server)
├── proto/          # Network protocol (C2S/S2C messages)
├── client_wasm/    # WebGPU renderer, input, prediction
├── server_do/      # Durable Object (game state, WebSocket hub)
├── lobby_worker/   # HTTP endpoints, static serving
├── scripts/        # Deployment and testing scripts
└── worker/         # Built WASM packages, static assets
```

## Resources

- **Specification**: `SPEC.md` - Full game design and architecture
- **Test Plan**: `TEST-PLAN.md` - Testing procedures
- **Completion Plan**: `COMPLETION-PLAN.md` - Milestone tracking
- **README**: `README.md` - Quick start and overview

