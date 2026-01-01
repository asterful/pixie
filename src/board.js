class Board {
  constructor(width, height, defaultColor, snapshotInterval = 200) {
    this.width = width;
    this.height = height;
    this.defaultColor = defaultColor;
    this.snapshotInterval = snapshotInterval;
    this.grid = Array(height)
      .fill(null)
      .map(() => Array(width).fill(defaultColor));
    
    // Snapshot-based history
    this.segments = [{
      index: 0,
      timestamp: Date.now(),
      snapshot: this.cloneGrid(),
      changes: []
    }];
    this.totalChanges = 0;
  }

  cloneGrid() {
    return this.grid.map(row => [...row]);
  }

  setPixel(x, y, color) {
    const normalizedColor = color.toUpperCase();
    this.grid[y][x] = normalizedColor;
    
    const currentSegment = this.segments[this.segments.length - 1];
    const change = {
      x,
      y,
      color: normalizedColor,
      timestamp: Date.now()
    };
    
    currentSegment.changes.push(change);
    this.totalChanges++;
    
    // Create new snapshot when reaching interval
    if (currentSegment.changes.length >= this.snapshotInterval) {
      this.segments.push({
        index: this.totalChanges,
        timestamp: Date.now(),
        snapshot: this.cloneGrid(),
        changes: []
      });
    }
    
    return normalizedColor;
  }

  getPixel(x, y) {
    return this.grid[y][x];
  }

  getState() {
    return this.grid;
  }

  getHistoryStats() {
    return {
      totalChanges: this.totalChanges,
      segments: this.segments.length,
      snapshotInterval: this.snapshotInterval
    };
  }
}

module.exports = Board;
