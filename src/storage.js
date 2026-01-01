const fs = require('fs').promises;
const path = require('path');
const Board = require('./board');
const { SESSION_NAME, STORAGE_PATH, BOARD_WIDTH, BOARD_HEIGHT, DEFAULT_COLOR, AUTOSAVE_INTERVAL_MS } = require('./constants');

const HISTORY_FILE = path.join(STORAGE_PATH, `history-${SESSION_NAME}.json`);

async function loadHistory() {
  try {
    const data = await fs.readFile(HISTORY_FILE, 'utf-8');
    const parsed = JSON.parse(data);
    console.log(`[Storage] Loaded history from ${HISTORY_FILE}`);
    console.log(`[Storage] Segments: ${parsed.segments.length}, Total changes: ${parsed.totalChanges}`);
    return parsed;
  } catch (error) {
    if (error.code === 'ENOENT') {
      console.log(`[Storage] No existing history file found at ${HISTORY_FILE}`);
      return null;
    }
    console.error(`[Storage] Error loading history:`, error);
    return null;
  }
}

async function saveHistory(segments, totalChanges) {
  try {
    // Ensure storage directory exists
    await fs.mkdir(STORAGE_PATH, { recursive: true });
    
    const data = {
      sessionName: SESSION_NAME,
      savedAt: Date.now(),
      totalChanges,
      segments
    };
    
    await fs.writeFile(HISTORY_FILE, JSON.stringify(data), 'utf-8');
    console.log(`[Storage] Saved history to ${HISTORY_FILE} (${segments.length} segments, ${totalChanges} changes)`);
  } catch (error) {
    console.error(`[Storage] Error saving history:`, error);
  }
}

async function initializeBoard() {
  const historyData = await loadHistory();
  
  if (historyData) {
    console.log(`[Init] Loading session: ${SESSION_NAME}`);
    return Board.fromHistory(historyData, DEFAULT_COLOR);
  } else {
    console.log(`[Init] Starting new session: ${SESSION_NAME}`);
    return new Board(BOARD_WIDTH, BOARD_HEIGHT, DEFAULT_COLOR);
  }
}

function startAutosave(board) {
  setInterval(() => {
    saveHistory(board.segments, board.totalChanges);
  }, AUTOSAVE_INTERVAL_MS);
  
  console.log(`[Autosave] Enabled - saving every ${AUTOSAVE_INTERVAL_MS / 1000}s`);
}

module.exports = { loadHistory, saveHistory, initializeBoard, startAutosave, HISTORY_FILE };
