# Place WebSocket Server

Real-time collaborative pixel canvas server inspired by r/Place.

## Architecture

### Core Components

1. **Board State** (`src/board.js`)
   - Authoritative 128x128 grid
   - Each cell stores a hex color (#RRGGBB)
   - In-memory only (no persistence)

2. **WebSocket Server** (`src/server.js`)
   - Handles client connections
   - Broadcasts updates to all connected clients
   - Instant broadcast (no batching needed at ~50 clients)

3. **Validator** (`src/validator.js`)
   - Validates paint messages
   - Ensures hex color format
   - Silently drops invalid messages

4. **Config** (`src/config.js`)
   - Environment variable management
   - Sensible defaults

### Message Protocol

#### Client → Server (Paint Action)
```json
{
  "type": "paint",
  "x": 64,
  "y": 64,
  "color": "#FF5733"
}
```

#### Server → Client (Initial State)
```json
{
  "type": "init",
  "width": 128,
  "height": 128,
  "board": [
    ["#FFFFFF", "#FFFFFF", ...],
    ...
  ]
}
```

#### Server → Client (Broadcast Update)
```json
{
  "type": "update",
  "x": 64,
  "y": 64,
  "color": "#FF5733"
}
```

## Installation

```bash
npm install
```

## Running Locally

```bash
npm start
# or
npm run dev
```

Server runs on port 3000 by default.

## Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `PORT` | 3000 | Server port (Railway sets this automatically) |
| `BOARD_WIDTH` | 128 | Board width in pixels |
| `BOARD_HEIGHT` | 128 | Board height in pixels |
| `DEFAULT_COLOR` | #FFFFFF | Initial color for all cells |
| `RATE_LIMIT_ENABLED` | false | Enable per-client rate limiting |
| `RATE_LIMIT_SECONDS` | 5 | Cooldown between paints (if enabled) |

## Railway Deployment

1. Push to GitHub
2. Connect Railway to your repo
3. Railway will auto-detect Node.js and use `npm start`
4. No additional configuration needed

## Testing with websocat

Install [websocat](https://github.com/vi/websocat):

```bash
# Connect
websocat ws://localhost:3000

# You'll receive the initial board state immediately

# Send a paint command
{"type":"paint","x":10,"y":10,"color":"#FF0000"}

# You should see the update broadcast back
```

## Design Decisions

### Why No Persistence?
- Simplicity: No database, no disk I/O
- Railway restarts: State loss is acceptable for this use case
- Can add later: Periodic snapshots to disk or Redis if needed

### Why JSON Instead of Binary?
- Simplicity and debuggability
- At 50 clients, bandwidth is not a bottleneck
- Easy to test with standard tools

### Why Last-Write-Wins?
- No conflict resolution needed
- Node.js single-threaded model naturally serializes updates
- Matches r/Place behavior (chaotic is part of the fun)

### Why No Authentication?
- Out of scope for MVP
- Can add later: JWT tokens, rate limiting by IP, etc.

## Limitations

- **Stateless:** Board resets on server restart
- **No history:** No undo, no replay
- **No authentication:** Anyone can paint
- **Memory only:** ~0.23 MB for 128x128 board
- **Single instance:** No horizontal scaling (use sticky sessions if needed)

## Performance

- **Board:** 128×128 = 16,384 cells
- **Memory:** ~0.23 MB for board state
- **Max clients:** Tested for ~50, can handle hundreds
- **Broadcast:** Instant (no batching needed)

## Monitoring

Server logs include:
- Client connections/disconnections
- Paint actions with coordinates and color
- Validation failures (dropped messages)
- Broadcast errors

## Future Enhancements (Not Implemented)

- Persistence (periodic snapshots)
- Rate limiting per IP (not just per connection)
- Admin endpoints (clear board, stats)
- Historical replay
- Multiple boards/rooms
