const WebSocket = require('ws');
const Board = require('./board');
const { parseMessage, validatePaintMessage } = require('./utils');
const { PORT, BOARD_WIDTH, BOARD_HEIGHT, DEFAULT_COLOR, MESSAGE_TYPES } = require('./constants');

// Initialize board and clients
const board = new Board(BOARD_WIDTH, BOARD_HEIGHT, DEFAULT_COLOR);
const clients = new Set();

// Helper functions
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
    width: BOARD_WIDTH,
    height: BOARD_HEIGHT,
    board: board.getState()
  });
}

// Message handlers
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
  const segments = board.segments;
  sendToClient(ws, {
    type: MESSAGE_TYPES.HISTORY_RESPONSE,
    segments: segments,
    stats: board.getHistoryStats()
  });
}

// WebSocket server setup
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

// Server startup and shutdown
console.log(`Place WebSocket Server\nPort: ${PORT} | Board: ${BOARD_WIDTH}x${BOARD_HEIGHT}`);
process.on('SIGTERM', () => process.exit(0));
process.on('SIGINT', () => process.exit(0));
