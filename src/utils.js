const { BOARD_WIDTH, BOARD_HEIGHT } = require('./constants');

function parseMessage(raw) {
  try {
    return JSON.parse(raw);
  } catch {
    return null;
  }
}

function isValidColor(color) {
  return typeof color === 'string' && /^#[0-9A-Fa-f]{6}$/.test(color);
}

function isValidCoordinates(x, y) {
  return (
    Number.isInteger(x) &&
    Number.isInteger(y) &&
    x >= 0 &&
    x < BOARD_WIDTH &&
    y >= 0 &&
    y < BOARD_HEIGHT
  );
}

function validatePaintMessage(message) {
  if (!message || message.type !== 'paint') {
    return { valid: false, error: 'Invalid message type' };
  }

  const { x, y, color } = message;

  if (!isValidCoordinates(x, y)) {
    return { valid: false, error: 'Invalid coordinates' };
  }

  if (!isValidColor(color)) {
    return { valid: false, error: 'Invalid color format' };
  }

  return { valid: true };
}

module.exports = {
  parseMessage,
  isValidColor,
  isValidCoordinates,
  validatePaintMessage
};
