const WebSocket = require('ws');
const http = require('http');
const { parseMessage, validatePaintMessage } = require('./utils');
const { PORT, MESSAGE_TYPES, SESSION_NAME, PAINT_COOLDOWN_MS } = require('./constants');
const { initializeBoard, startAutosave, saveHistory } = require('./storage');
const { checkBotBehavior } = require('./heuristics');

// Global state
let board;
let wss; // Defined globally, initialized later
let httpServer; // HTTP server for history endpoint
const clients = new Set();
const clientLastPaint = new WeakMap(); // Track last paint time per client
const clientRequestHistory = new WeakMap(); // Track last 20 requests per client

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
  // This is now safe because we don't accept connections until board is defined
  sendToClient(ws, {
    type: MESSAGE_TYPES.INIT,
    width: board.width,
    height: board.height,
    board: board.getState(),
    cooldown: PAINT_COOLDOWN_MS
  });
}

function handlePing(ws) {
  sendToClient(ws, {
    type: MESSAGE_TYPES.PONG,
    clients: clients.size
  });
}

function handlePaint(ws, message) {
  const validation = validatePaintMessage(message);
  if (!validation.valid) return;

  const now = Date.now();
  const { x, y, color } = message;

  // Record ALL paint attempts in history (before any checks)
  const requestHistory = clientRequestHistory.get(ws) || [];
  requestHistory.push({ timestamp: now, x, y, color });
  
  // Keep only last 20 requests
  if (requestHistory.length > 20) {
    requestHistory.shift();
  }
  clientRequestHistory.set(ws, requestHistory);

  // Bot detection check
  const botCheck = checkBotBehavior(requestHistory);
  if (botCheck.isBot) {
    // Silently reject bot requests
    return;
  }

  // Rate limiting check
  const lastPaint = clientLastPaint.get(ws) || 0;
  const timeSinceLastPaint = now - lastPaint;

  if (timeSinceLastPaint < PAINT_COOLDOWN_MS) {
    const waitTime = PAINT_COOLDOWN_MS - timeSinceLastPaint;
    sendToClient(ws, {
      type: MESSAGE_TYPES.RATE_LIMIT,
      message: 'Please wait before placing another pixel',
      waitTime
    });
    return;
  }

  const normalizedColor = board.setPixel(x, y, color);

  // Update last paint time for this client
  clientLastPaint.set(ws, now);

  broadcast({
    type: MESSAGE_TYPES.UPDATE,
    x,
    y,
    color: normalizedColor
  });
}

// ============================================================================
// HTTP and WebSocket Server Setup
// ============================================================================

// HTTP server with WebSocket upgrade support
function startServer() {
  // Create HTTP server
  httpServer = http.createServer((req, res) => {
    // Enable CORS
    res.setHeader('Access-Control-Allow-Origin', '*');
    res.setHeader('Access-Control-Allow-Methods', 'GET, OPTIONS');
    res.setHeader('Access-Control-Allow-Headers', 'Content-Type');

    if (req.method === 'OPTIONS') {
      res.writeHead(200);
      res.end();
      return;
    }

    // Handle GET /history endpoint
    if (req.method === 'GET' && req.url === '/history') {
      res.writeHead(200, { 'Content-Type': 'application/json' });
      res.end(JSON.stringify({
        width: board.width,
        height: board.height,
        board: board.getState(),
        segments: board.segments,
        stats: board.getHistoryStats()
      }));
      return;
    }

    // Default 404 for other routes
    res.writeHead(404, { 'Content-Type': 'text/plain' });
    res.end('Not Found');
  });

  // Create WebSocket server attached to HTTP server
  const server = new WebSocket.Server({ server: httpServer });

  server.on('connection', (ws) => {
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
          handlePaint(ws, message);
          break;

        default:
          break;
      }
    });

    ws.on('close', () => {
      clients.delete(ws);
      clientLastPaint.delete(ws);
      clientRequestHistory.delete(ws);
    });

    ws.on('error', () => {});
  });

  // Start listening
  httpServer.listen(PORT);

  return server;
}

// ============================================================================
// Server Lifecycle
// ============================================================================

async function shutdown() {
  console.log('[Shutdown] Saving final state...');
  if (board) {
      await saveHistory(board.segments, board.totalChanges);
  }
  process.exit(0);
}

async function start() {
  try {
    console.log('Loading board data...');
    // 1. Initialize Board FIRST (Waits here until done)
    board = await initializeBoard();
    
    // 2. Start Autosave
    startAutosave(board);

    // 3. Start Server SECOND (Only runs after board is ready)
    wss = startServer();
    
    console.log(`Place WebSocket Server\nSession: ${SESSION_NAME}\nPort: ${PORT} | Board: ${board.width}x${board.height}`);
  } catch (err) {
    console.error('[Error] Failed during startup:', err);
    process.exit(1);
  }
}

// Start server
start();

// Graceful shutdown
process.on('SIGTERM', shutdown);
process.on('SIGINT', shutdown);
