module.exports = {
  PORT: process.env.PORT || 3000,
  BOARD_WIDTH: 256,
  BOARD_HEIGHT: 256,
  DEFAULT_COLOR: '#FFFFFF',
  
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
