// ============================================================================
// Bot Detection Heuristics
// ============================================================================

// Configuration
const MIN_REQUESTS_FOR_CHECK = 10;  // Need enough data to detect patterns
const VARIANCE_THRESHOLD_MS = 30;  // Standard deviation below this = suspicious

/**
 * Calculate standard deviation of intervals between timestamps
 * @param {number[]} timestamps - Array of timestamp values in milliseconds
 * @returns {number} Standard deviation in milliseconds
 */
function calculateTimingVariance(timestamps) {
  if (timestamps.length < 2) return Infinity;

  // Calculate intervals between consecutive requests
  const intervals = [];
  for (let i = 1; i < timestamps.length; i++) {
    intervals.push(timestamps[i] - timestamps[i - 1]);
  }

  // Calculate mean interval
  const mean = intervals.reduce((sum, val) => sum + val, 0) / intervals.length;

  // Calculate variance
  const squaredDiffs = intervals.map(val => Math.pow(val - mean, 2));
  const variance = squaredDiffs.reduce((sum, val) => sum + val, 0) / intervals.length;

  // Return standard deviation
  return Math.sqrt(variance);
}

/**
 * Analyzes request history to detect bot-like behavior
 * @param {Array<{timestamp: number, x: number, y: number, color: string}>} requests
 * @returns {{isBot: boolean, reason: string | null}}
 */
function checkBotBehavior(requests) {
  // Not enough data to make a determination
  if (requests.length < MIN_REQUESTS_FOR_CHECK) {
    return { isBot: false, reason: null };
  }

  // Extract timestamps
  const timestamps = requests.map(req => req.timestamp);

  // Check timing variance
  const stdDev = calculateTimingVariance(timestamps);

  if (stdDev < VARIANCE_THRESHOLD_MS) {
    return {
      isBot: true,
      reason: `Timing too consistent (stddev: ${stdDev.toFixed(2)}ms)`
    };
  }

  return { isBot: false, reason: null };
}

module.exports = {
  checkBotBehavior,
  calculateTimingVariance,
  MIN_REQUESTS_FOR_CHECK,
  VARIANCE_THRESHOLD_MS
};
