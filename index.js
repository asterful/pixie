const WebSocket = require('ws');

const PORT = process.env.PORT || 3000;
const BOARD_WIDTH = 128;
const BOARD_HEIGHT = 128;
const DEFAULT_COLOR = '#FFFFFF';

const board = Array(BOARD_HEIGHT)
  .fill(null)
  .map(() => Array(BOARD_WIDTH).fill(DEFAULT_COLOR));

const clients = new Set();

function isValidPaint(message) {
  if (!message || message.type !== 'paint') return false;
  const { x, y, color } = message;
  if (!Number.isInteger(x) || !Number.isInteger(y)) return false;
  if (x < 0 || x >= BOARD_WIDTH || y < 0 || y >= BOARD_HEIGHT) return false;
  if (typeof color !== 'string' || !/^#[0-9A-Fa-f]{6}$/.test(color)) return false;
  return true;
}

function broadcast(message) {
  const data = JSON.stringify(message);
  clients.forEach(client => {
    if (client.readyState === WebSocket.OPEN) {
      client.send(data);
    }
  });
}

const wss = new WebSocket.Server({ port: PORT });

wss.on('connection', (ws) => {
  clients.add(ws);

  ws.send(JSON.stringify({
    type: 'init',
    width: BOARD_WIDTH,
    height: BOARD_HEIGHT,
    board: board
  }));

  ws.on('message', (raw) => {
    let message;
    try {
      message = JSON.parse(raw);
    } catch {
      return;
    }
    
    if (message.type === 'ping') {
      ws.send(JSON.stringify({
        type: 'pong',
        clients: clients.size
      }));
      return;
    }
    
    if (!isValidPaint(message)) return;

    const { x, y, color } = message;
    const normalizedColor = color.toUpperCase();
    
    board[y][x] = normalizedColor;
    broadcast({ type: 'update', x, y, color: normalizedColor });
  });

  ws.on('close', () => clients.delete(ws));
  ws.on('error', () => {});
});

console.log(`Place WebSocket Server\nPort: ${PORT} | Board: ${BOARD_WIDTH}x${BOARD_HEIGHT}`);
process.on('SIGTERM', () => process.exit(0));
process.on('SIGINT', () => process.exit(0));
