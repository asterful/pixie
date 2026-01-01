class Board {
  constructor(width, height, defaultColor) {
    this.width = width;
    this.height = height;
    this.defaultColor = defaultColor;
    this.grid = Array(height)
      .fill(null)
      .map(() => Array(width).fill(defaultColor));
    this.history = [];
  }

  setPixel(x, y, color) {
    const normalizedColor = color.toUpperCase();
    this.grid[y][x] = normalizedColor;
    this.history.push({
      x,
      y,
      color: normalizedColor,
      timestamp: Date.now()
    });
    return normalizedColor;
  }

  getPixel(x, y) {
    return this.grid[y][x];
  }

  getState() {
    return this.grid;
  }

  getHistory(limit) {
    return limit ? this.history.slice(-limit) : this.history;
  }
}

module.exports = Board;
