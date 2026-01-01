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

  // Load board from saved history
  static fromHistory(historyData, defaultColor = '#FFFFFF', snapshotInterval = 200) {
    if (!historyData || !historyData.segments || historyData.segments.length === 0) {
      throw new Error('Invalid history data: missing segments');
    }
    
    // Get dimensions from the saved snapshot
    const firstSnapshot = historyData.segments[0].snapshot;
    const height = firstSnapshot.length;
    const width = firstSnapshot[0].length;
    
    console.log(`[Board] Loading from history: ${width}x${height} board`);
    
    const board = new Board(width, height, defaultColor, snapshotInterval);
    board.segments = historyData.segments;
    board.totalChanges = historyData.totalChanges || 0;
    
    // Reconstruct current grid state from last segment
    const lastSegment = board.segments[board.segments.length - 1];
    board.grid = lastSegment.snapshot.map(row => [...row]);
    
    // Apply any remaining changes in the last segment
    for (const change of lastSegment.changes) {
      board.grid[change.y][change.x] = change.color;
    }
    
    console.log(`[Board] Restored from history: ${board.totalChanges} changes, ${board.segments.length} segments`);
    
    return board;
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
