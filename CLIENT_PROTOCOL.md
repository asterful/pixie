# Place WebSocket Protocol

Communication protocol for the Place collaborative canvas server.

## Transport

- **Protocol:** WebSocket (RFC 6455)
- **Format:** JSON text messages
- **Encoding:** UTF-8
- **Development:** `ws://localhost:3000`
- **Production:** `wss://pixie-production-677d.up.railway.app`

## Connection Lifecycle

1. Client establishes WebSocket connection
2. Server immediately sends `init` message with full board state
3. Client sends `paint` messages to update pixels
4. Server broadcasts `update` messages to all connected clients
5. Either side may close the connection at any time

---

## Message Types

### 1. Initial State (Server → Client)

Sent immediately upon connection establishment.

**Message:**
```json
{
  "type": "init",
  "width": 128,
  "height": 128,
  "board": [
    ["#FFFFFF", "#FFFFFF", ...],
    ["#FFFFFF", "#FFFFFF", ...],
    ...
  ],
  "cooldown": 500
}
```

**Fields:**
| Field | Type | Description |
|-------|------|-------------|
| `type` | string | Always `"init"` |
| `width` | integer | Board width in pixels |
| `height` | integer | Board height in pixels |
| `board` | array | 2D array `[height][width]` of hex color strings |
| `cooldown` | integer | Minimum milliseconds between paint requests per client |

**Notes:**
- Board array is indexed as `board[y][x]` (row, then column)
- All cells initially contain `#FFFFFF` (white)
- This is the only time the full board state is sent

---

### 2. Paint Action (Client → Server)

Request to paint a pixel on the board.

**Message:**
```json
{
  "type": "paint",
  "x": 64,
  "y": 64,
  "color": "#FF5733"
}
```

**Fields:**
| Field | Type | Constraints |
|-------|------|-------------|
| `type` | string | Must be `"paint"` |
| `x` | integer | `0 ≤ x < width` |
| `y` | integer | `0 ≤ y < height` |
| `color` | string | Hex format `#RRGGBB` (case insensitive) |

**Validation:**
- Color must match regex: `^#[0-9A-Fa-f]{6}$`
- Coordinates must be integers within board bounds
- Invalid messages are silently dropped (no error response)

**Rate Limiting:**
- Clients must wait at least `cooldown` ms between paint requests
- Requests sent too quickly will be rejected with a `rate_limit` message
- The cooldown value is provided in the `init` message (typically 700ms)

**Valid Examples:**
```json
{"type":"paint","x":0,"y":0,"color":"#FF0000"}
{"type":"paint","x":127,"y":127,"color":"#00ff00"}
{"type":"paint","x":50,"y":50,"color":"#AbCdEf"}
```

**Invalid Examples:**
```json
{"type":"paint","x":-1,"y":0,"color":"#FF0000"}      // x out of bounds
{"type":"paint","x":0,"y":128,"color":"#FF0000"}     // y out of bounds  
{"type":"paint","x":0,"y":0,"color":"FF0000"}        // missing # prefix
{"type":"paint","x":0,"y":0,"color":"#FF00"}         // wrong length
{"type":"paint","x":0.5,"y":0,"color":"#FF0000"}     // x not integer
{"type":"paint","x":"0","y":0,"color":"#FF0000"}     // x is string
```

---

### 3. Update Broadcast (Server → Client)

Sent to all connected clients when any client paints a pixel.

**Message:**
```json
{
  "type": "update",
  "x": 64,
  "y": 64,
  "color": "#FF5733"
}
```

**Fields:**
| Field | Type | Description |
|-------|------|-------------|
| `type` | string | Always `"update"` |
| `x` | integer | X coordinate that changed |
| `y` | integer | Y coordinate that changed |
| `color` | string | New color (uppercase hex) |

**Notes:**
- Color is normalized to uppercase by server
- Clients receive broadcasts for their own paint actions
- All connected clients receive the same update simultaneously

---

### 4. Client Count Request (Client → Server)

Request the current number of connected clients.

**Message:**
```json
{
  "type": "ping"
}
```

**Fields:**
| Field | Type | Description |
|-------|------|-------------|
| `type` | string | Must be `"ping"` |

**Notes:**
- No other fields required
- Can be sent at any time after connection
- Recommended polling interval: 3-5 seconds

---

### 5. Client Count Response (Server → Client)

Response with the current number of connected clients.

**Message:**
```json
{
  "type": "pong",
  "clients": 42
}
```

**Fields:**
| Field | Type | Description |
|-------|------|-------------|
| `type` | string | Always `"pong"` |
| `clients` | integer | Number of currently connected WebSocket clients |

**Notes:**
- Sent only in response to a `ping` message
- Count includes the requesting client
- Count may change between requests

---

### 6. History Request (Client → Server) - DEPRECATED

**⚠️ DEPRECATED:** This WebSocket operation has been moved to HTTP due to performance concerns. Use `GET /history` HTTP endpoint instead (see HTTP API section below).

~~Request the complete history of all board changes via WebSocket.~~

**Note:** This message type is no longer supported. The server will silently ignore `history` message types. Use the HTTP endpoint instead.

---

### 7. Rate Limit Response (Server → Client)

Sent when a client attempts to paint too quickly.

**Message:**
```json
{
  "type": "rate_limit",
  "message": "Please wait before placing another pixel",
  "waitTime": 450
}
```

**Fields:**
| Field | Type | Description |
|-------|------|-------------|
| `type` | string | Always `"rate_limit"` |
| `message` | string | Human-readable error message |
| `waitTime` | integer | Milliseconds until next paint request is allowed |

**Notes:**
- Only sent when client violates rate limit
- The paint request is rejected and not processed
- Client should wait at least `waitTime` ms before retrying
- Recommended: disable paint UI during cooldown

---

## Error Handling

**Server behavior:**
- Invalid JSON: silently dropped
- Unknown message type: silently dropped  
- Failed validation: silently dropped
- Server logs rejection reason for debugging

**No error messages:**
- Server does not send error responses to clients
- Clients must validate locally before sending
- Check server logs if messages aren't being processed

---

## State Guarantees

**The server guarantees:**
- All clients see the same board state
- Updates broadcast in processing order
- Last-write-wins for concurrent updates

**The server does NOT guarantee:**
- Persistence across restarts
- Delivery confirmation
- Replay or undo

---

## HTTP API

The server provides HTTP endpoints for operations that are too expensive for WebSocket transport.

### GET /history

Retrieve the complete history of all board changes.

**Endpoint:** `GET /history`

**Request:**
- No parameters required
- No authentication required

**Response:**
```json
{
  "width": 128,
  "height": 128,
  "board": [
    ["#FFFFFF", "#FFFFFF", ...],
    ["#FFFFFF", "#FFFFFF", ...],
    ...
  ],
  "segments": [
    {
      "index": 0,
      "timestamp": 1704067200000,
      "snapshot": [
        ["#FFFFFF", "#FFFFFF", ...],
        ["#FFFFFF", "#FFFFFF", ...]
      ],
      "changes": [
        {"x": 10, "y": 10, "color": "#FF0000", "timestamp": 1704067205000},
        {"x": 11, "y": 10, "color": "#00FF00", "timestamp": 1704067210000}
      ]
    },
    {
      "index": 200,
      "timestamp": 1704067300000,
      "snapshot": [],
      "changes": []
    }
  ],
  "stats": {
    "totalChanges": 42,
    "segments": 3,
    "snapshotInterval": 200
  }
}
```

**Response Fields:**
| Field | Type | Description |
|-------|------|-------------|
| `width` | integer | Board width in pixels |
| `height` | integer | Board height in pixels |
| `board` | array | Current 2D board state `[height][width]` of hex color strings |
| `segments` | array | Array of history segments (see below) |
| `stats` | object | Statistics about the history |

**Segment Structure:**
| Field | Type | Description |
|-------|------|-------------|
| `index` | integer | Total number of changes before this segment |
| `timestamp` | integer | Unix timestamp (ms) when segment was created |
| `snapshot` | array | 2D array `[height][width]` of hex colors at segment start |
| `changes` | array | Array of changes in this segment (max = `snapshotInterval` from stats) |

**Change Structure:**
| Field | Type | Description |
|-------|------|-------------|
| `x` | integer | X coordinate of change |
| `y` | integer | Y coordinate of change |
| `color` | string | Color (uppercase hex) |
| `timestamp` | integer | Unix timestamp (ms) when change occurred |

**Stats Structure:**
| Field | Type | Description |
|-------|------|-------------|
| `totalChanges` | integer | Total number of pixel changes |
| `segments` | integer | Number of history segments |
| `snapshotInterval` | integer | Server-configured changes per segment (typically 200) |

**HTTP Status Codes:**
- `200 OK`: Success
- `404 Not Found`: Invalid endpoint

**Notes:**
- CORS is enabled (Access-Control-Allow-Origin: *)
- Response can be very large (contains full snapshots)
- Use for timeline reconstruction and replay features
- Segments created every `snapshotInterval` changes
- To reconstruct state at any time:
  1. Find the latest segment at or before target timestamp
  2. Start with that segment's snapshot
  3. Replay changes up to target timestamp
- Maximum `snapshotInterval` changes need to be replayed for any point in time

---

## Security
- No authentication
- No authorization  
- Rate limiting (WebSocket paint operations only)
- Use `wss://` and `https://` in production