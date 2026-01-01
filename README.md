# Place WebSocket Server

Real-time collaborative pixel canvas server inspired by r/Place.

## Architecture

### Core Components

1. **Board State** (`src/board.js`)
   - Authoritative 256x256 grid
   - Each cell stores a hex color (#RRGGBB)
   - In-memory only (no persistence)
   - Snapshot-based history tracking (configurable interval, default: 200 changes)

2. **WebSocket Server** (`src/index.js`)
   - Handles client connections
   - Broadcasts updates to all connected clients
   - Instant broadcast (no batching needed at ~50 clients)
   - Supports history requests for timeline replay

3. **Utilities** (`src/utils.js`)
   - Validates paint messages
   - Ensures hex color format
   - Silently drops invalid messages

4. **Constants** (`src/constants.js`)
   - Board dimensions and defaults
   - Message type definitions

### Message Protocol

See [CLIENT_PROTOCOL.md](CLIENT_PROTOCOL.md) for complete protocol documentation.

#### Client → Server (Paint Action)
```json
{
  "type": "paint",
  "x": 64,
  "y": 64,
  "color": "#FF5733"
}
```

#### Client → Server (History Request)
```json
{
  "type": "history"
}
```

#### Server → Client (Initial State)
```json
{
  "type": "init",
  "width": 256,
  "height": 256,
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

#### Server → Client (History Response)
```json
{
  "type": "history_response",
  "segments": [...],
  "stats": {
    "totalChanges": 42,
    "segments": 3,
    "snapshotInterval": 20
  }
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

Board dimensions and defaults are configured in `src/constants.js`.

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

### Why Snapshot-Based History?
- Efficient timeline reconstruction: maximum N changes to replay for any point in time (where N = `snapshotInterval`)
- Trade-off: snapshots use more memory than flat history, but enable fast time-travel
- Perfect for day-long sessions with replay/timeline features
- Interval is configurable (default: 200 changes per segment)

### Why No Persistence?
- Simplicity: No database, no disk I/O
- Day-long sessions: Server runs for a day then resets
- Can add later: Export history to JSON at end of session if needed

### Why JSON Instead of Binary?
- Simplicity and debuggability
- At 50 clients, bandwidth is not a bottleneck
- Easy to test with standard tools

### Why Last-Write-Wins?
- No conflict resolution needed
- Node.js single-threaded model naturally serializes updates
- Matches r/Place behavior (chaotic is part of the fun)

### Why No Authentication?
- Out of scope for this project
- Can add later: JWT tokens, rate limiting by IP, etc.

## Limitations

- **Stateless:** Board resets on server restart
- **No persistence:** History lost on shutdown (export feature can be added)
- **No authentication:** Anyone can paint
- **Memory grows:** History unbounded within session (acceptable for day-long sessions)
- **Single instance:** No horizontal scaling (use sticky sessions if needed)

## Performance

- **Board:** 256×256 = 65,536 cells
- **Memory:** ~1 MB for board state + history (grows with usage)
- **Max clients:** Tested for ~50, can handle hundreds
- **Broadcast:** Instant (no batching needed)
- **History replay:** Maximum `snapshotInterval` changes to reconstruct any point in time (configurable, default: 200)

## Client Files

- **client.html:** Main collaborative canvas interface
- **timeline.html:** Timeline viewer with history slider for replay

## Future Enhancements (Not Implemented)

- Persistence (periodic snapshots)
- Export history to JSON file
- Rate limiting per IP
- Admin endpoints (clear board, stats)
- Multiple boards/rooms
