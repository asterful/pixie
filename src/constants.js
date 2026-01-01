module.exports = {
  PORT: process.env.PORT || 3000,
  BOARD_WIDTH: 256,
  BOARD_HEIGHT: 256,
  DEFAULT_COLOR: '#FFFFFF',
  
  // Storage configuration
  SESSION_NAME: '1-1-2026',
  STORAGE_PATH: '/data',
  AUTOSAVE_INTERVAL_MS: 2 * 60 * 1000, // 2 minutes
  PAINT_COOLDOWN_MS: 200, // Rate limit for paint commands
  
  MESSAGE_TYPES: {
    PING: 'ping',
    PONG: 'pong',
    PAINT: 'paint',
    INIT: 'init',
    UPDATE: 'update',
    HISTORY: 'history',
    HISTORY_RESPONSE: 'history_response',
    RATE_LIMIT: 'rate_limit'
  }
};
