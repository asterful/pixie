const WebSocket = require('ws');
const { parseMessage, validatePaintMessage } = require('./utils');
const { PORT, MESSAGE_TYPES, SESSION_NAME } = require('./constants');
const { initializeBoard, startAutosave, saveHistory } = require('./storage');

// Global state
let board;
const clients = new Set();

// ============================================================================
// WebSocket Message Handlers
// ============================================================================

function broadcast(message) {
  const data = JSON.stringify(message);
  clients.forEach(client => {
    if (client.readyState === WebSocket.OPEN) {
      client.send(data);
    }
  });
}

function sendToClient(client, message) {
  if (client.readyState === WebSocket.OPEN) {
    client.send(JSON.stringify(message));
  }
}

function sendInitialState(ws) {
  sendToClient(ws, {
    type: MESSAGE_TYPES.INIT,
    width: board.width,
    height: board.height,
    board: board.getState()
  });
}

function handlePing(ws) {
  sendToClient(ws, {
    type: MESSAGE_TYPES.PONG,
    clients: clients.size
  });
}

function handlePaint(message) {
  const validation = validatePaintMessage(message);
  if (!validation.valid) return;

  const { x, y, color } = message;
  const normalizedColor = board.setPixel(x, y, color);

  broadcast({
    type: MESSAGE_TYPES.UPDATE,
    x,
    y,
    color: normalizedColor
  });
}

function handleHistory(ws) {
  sendToClient(ws, {
    type: MESSAGE_TYPES.HISTORY_RESPONSE,
    segments: board.segments,
    stats: board.getHistoryStats()
  });
}

// ============================================================================
// WebSocket Server Setup
// ============================================================================

const wss = new WebSocket.Server({ port: PORT });

wss.on('connection', (ws) => {
  clients.add(ws);
  sendInitialState(ws);

  ws.on('message', (raw) => {
    const message = parseMessage(raw);
    if (!message) return;

    switch (message.type) {
      case MESSAGE_TYPES.PING:
        handlePing(ws);
        break;

      case MESSAGE_TYPES.PAINT:
        handlePaint(message);
        break;

      case MESSAGE_TYPES.HISTORY:
        handleHistory(ws);
        break;

      default:
        break;
    }
  });

  ws.on('close', () => {
    clients.delete(ws);
  });

  ws.on('error', () => {});
});

// ============================================================================
// Server Lifecycle
// ============================================================================

async function shutdown() {
  console.log('[Shutdown] Saving final state...');
  await saveHistory(board.segments, board.totalChanges);
  process.exit(0);
}

async function start() {
  board = await initializeBoard();
  startAutosave(board);
  console.log(`Place WebSocket Server\nSession: ${SESSION_NAME}\nPort: ${PORT} | Board: ${board.width}x${board.height}`);
}

// Start server
start().catch(err => {
  console.error('[Error] Failed to start server:', err);
  process.exit(1);
});

// Graceful shutdown
process.on('SIGTERM', shutdown);
process.on('SIGINT', shutdown);
