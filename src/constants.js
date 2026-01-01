module.exports = {
  PORT: process.env.PORT || 3000,
  BOARD_WIDTH: 256,
  BOARD_HEIGHT: 256,
  DEFAULT_COLOR: '#FFFFFF',
  
  // Storage configuration
  SESSION_NAME: 'newyears-2025',
  STORAGE_PATH: '/data',
  AUTOSAVE_INTERVAL_MS: 2 * 60 * 1000, // 2 minutes
  
  MESSAGE_TYPES: {
    PING: 'ping',
    PONG: 'pong',
    PAINT: 'paint',
    INIT: 'init',
    UPDATE: 'update',
    HISTORY: 'history',
    HISTORY_RESPONSE: 'history_response'
  }
};
