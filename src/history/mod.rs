pub mod change;

use std::sync::Mutex;
use std::sync::Arc;
use serde::{Deserialize, Serialize};

use crate::history::change::ChangeEvent;
use crate::world::canvas::Canvas;
use crate::history::change::Change;
use tokio::sync::mpsc::UnboundedSender;


#[derive(Debug)]
pub enum RollbackError {
    #[allow(dead_code)]
    IndexOutOfBounds {
        target: usize,
        max: usize,
    },
    #[allow(dead_code)]
    Database(rusqlite::Error),
}

impl From<rusqlite::Error> for RollbackError {
    fn from(err: rusqlite::Error) -> Self {
        RollbackError::Database(err)
    }
}

pub struct History {
    tx: UnboundedSender<(Change, Option<Canvas>)>,
    conn: Arc<Mutex<rusqlite::Connection>>,
    snapshot_interval: usize,
    event_count: std::sync::atomic::AtomicUsize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryChunk {
    pub prev_snapshot: Option<(i64, Canvas)>,
    pub current_snapshot: Option<(i64, Canvas)>,
    pub next_snapshot: Option<(i64, Canvas)>,
    pub next_next_snapshot: Option<(i64, Canvas)>,
    pub events: Vec<(i64, Change)>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[repr(u8)]
pub enum HistoryLookahead {
    Full,
    Forward,
    Backward,
}

#[allow(dead_code)]
impl History {

    /// Returns the true count of events directly from SQLite (disk).
    pub fn db_event_count(&self) -> Result<usize, rusqlite::Error> {
        let conn = self.conn.lock().unwrap();
        let max_id: i64 = conn
            .query_row("SELECT COALESCE(MAX(id), 0) FROM events", [], |row| row.get(0))
            .unwrap_or(0);
        Ok(max_id as usize)
    }

    pub fn get_history_chunk(
        &self,
        target_id: u64,
        lookahead: HistoryLookahead,
    ) -> Result<HistoryChunk, rusqlite::Error> {
        let conn = self.conn.lock().unwrap();
        let target_id = target_id as i64;

        // 1. Base snapshot <= target
        let current_snapshot = Self::latest_snapshot_before_conn(&conn, target_id)?;
        let current_id = current_snapshot.as_ref().map(|(id, _)| *id).unwrap_or(0);

        // 2. 1 snapshot down (backward)
        let prev_snapshot = match lookahead {
            HistoryLookahead::Forward => None,
            _ => Self::snapshot_before_conn(&conn, current_id)?,
        };

        // 3. 1 snapshot up (forward)
        let next_snapshot = match lookahead {
            HistoryLookahead::Backward => None,
            _ => Self::snapshot_after_conn(&conn, current_id)?,
        };

        // 4. 2 snapshots up (forward)
        let next_id = next_snapshot.as_ref().map(|(id, _)| *id).unwrap_or(i64::MAX);
        let next_next_snapshot = match lookahead {
            HistoryLookahead::Backward => None,
            _ => {
                if next_id == i64::MAX {
                    None
                } else {
                    Self::snapshot_after_conn(&conn, next_id)?
                }
            }
        };

        let prev_id = prev_snapshot.as_ref().map(|(id, _)| *id).unwrap_or(0);
        let upper_bound_id = next_next_snapshot
            .as_ref()
            .map(|(id, _)| *id)
            .unwrap_or(next_id);

        // Bind event range across snapshot intervals (non-overlapping deltas)
        let (min_id, max_id) = match lookahead {
            HistoryLookahead::Forward => (next_id, upper_bound_id),
            HistoryLookahead::Backward => (prev_id, current_id),
            HistoryLookahead::Full => (prev_id, upper_bound_id),
        };

        let events = Self::fetch_events_conn(&conn, min_id, max_id)?;

        // Filter out redundant snapshots for delta modes to prevent sending duplicate canvases
        let (res_prev, res_curr, res_next, res_next_next) = match lookahead {
            HistoryLookahead::Full => (prev_snapshot, current_snapshot, next_snapshot, next_next_snapshot),
            HistoryLookahead::Forward => (None, None, None, next_next_snapshot),
            HistoryLookahead::Backward => (prev_snapshot, None, None, None),
        };

        Ok(HistoryChunk {
            prev_snapshot: res_prev,
            current_snapshot: res_curr,
            next_snapshot: res_next,
            next_next_snapshot: res_next_next,
            events,
        })
    }

    fn snapshot_before_conn(conn: &rusqlite::Connection, current_id: i64) -> Result<Option<(i64, Canvas)>, rusqlite::Error> {
        let mut stmt = conn.prepare(
            "SELECT last_event_id, canvas_blob FROM snapshots WHERE last_event_id < ?1 ORDER BY last_event_id DESC LIMIT 1"
        )?;
        
        let mut rows = stmt.query(rusqlite::params![current_id])?;
        if let Some(row) = rows.next()? {
            let last_event_id: i64 = row.get(0)?;
            let blob: Vec<u8> = row.get(1)?;
            let canvas: Canvas = bincode::deserialize(&blob).map_err(|_| {
                rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Blob, Box::new(std::io::Error::new(std::io::ErrorKind::Other, "Failed to deserialize canvas blob")))
            })?;
            Ok(Some((last_event_id, canvas)))
        } else {
            Ok(None)
        }
    }

    fn snapshot_after_conn(conn: &rusqlite::Connection, current_id: i64) -> Result<Option<(i64, Canvas)>, rusqlite::Error> {
        let mut stmt = conn.prepare(
            "SELECT last_event_id, canvas_blob FROM snapshots WHERE last_event_id > ?1 ORDER BY last_event_id ASC LIMIT 1"
        )?;
        
        let mut rows = stmt.query(rusqlite::params![current_id])?;
        if let Some(row) = rows.next()? {
            let last_event_id: i64 = row.get(0)?;
            let blob: Vec<u8> = row.get(1)?;
            let canvas: Canvas = bincode::deserialize(&blob).map_err(|_| {
                rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Blob, Box::new(std::io::Error::new(std::io::ErrorKind::Other, "Failed to deserialize canvas blob")))
            })?;
            Ok(Some((last_event_id, canvas)))
        } else {
            Ok(None)
        }
    }

    fn fetch_events_conn(conn: &rusqlite::Connection, min_id: i64, max_id: i64) -> Result<Vec<(i64, Change)>, rusqlite::Error> {
        let mut stmt = conn.prepare(
            r#"
            SELECT 
                e.id, e.event_type_id, e.created_at,
                p.x, p.y, p.color_hex,
                r.anchor_type, r.width, r.height,
                rb.target_event_id
            FROM events e
            LEFT JOIN paint_event p ON e.id = p.event_id
            LEFT JOIN resize_event r ON e.id = r.event_id
            LEFT JOIN rollback_event rb ON e.id = rb.event_id
            WHERE e.id > ?1 AND e.id <= ?2
            ORDER BY e.id ASC
            "#
        )?;

        let mut rows = stmt.query(rusqlite::params![min_id, max_id])?;
        let mut events = Vec::new();

        while let Some(row) = rows.next()? {
            let event_id: i64 = row.get(0)?;
            let event_type_id: i64 = row.get(1)?;
            let timestamp: u64 = row.get::<_, i64>(2)? as u64;

            let event = match event_type_id {
                0 => ChangeEvent::Paint {
                    x: row.get::<_, i64>(3)? as usize,
                    y: row.get::<_, i64>(4)? as usize,
                    color: crate::world::color::Color::from_hex(&row.get::<_, String>(5)?).expect("Failed to parse color hex"),
                },
                1 => ChangeEvent::Resize {
                    anchor: crate::history::change::ResizeAnchor::from_u8(row.get::<_, i64>(6)? as u8),
                    width: row.get::<_, i64>(7)? as usize,
                    height: row.get::<_, i64>(8)? as usize,
                },
                2 => ChangeEvent::Rollback {
                    target_event_id: row.get(9)?,
                },
                _ => ChangeEvent::Init {
                    width: crate::env::default_canvas_width(),
                    height: crate::env::default_canvas_height(),
                },
            };

            events.push((event_id, Change { event, timestamp }));
        }

        Ok(events)
    }

    pub fn open<P: AsRef<std::path::Path>>(db_path: P, snapshot_interval: usize) -> Result<Self, rusqlite::Error> {
        if let Some(parent) = db_path.as_ref().parent() {
            std::fs::create_dir_all(parent).map_err(|_| {
                rusqlite::Error::InvalidPath(db_path.as_ref().to_path_buf())
            })?;
        }

        let conn = rusqlite::Connection::open(db_path)?;

        // Enable WAL mode and foreign keys
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;

        // Executes schema.sql (tables are created only if they don't exist yet)
        const SCHEMA_SQL: &str = include_str!("sql/schema.sql");
        conn.execute_batch(SCHEMA_SQL)?;

        let conn_arc = Arc::new(Mutex::new(conn));

        let initial_count: usize = conn_arc
            .lock()
            .unwrap()
            .query_row("SELECT COALESCE(MAX(id), 0) FROM events", [], |row| row.get::<_, i64>(0))
            .map(|val| val as usize)
            .unwrap_or(0);

        // Seed initial event and snapshot if the database is completely empty
        if initial_count == 0 {
            let mut conn = conn_arc.lock().unwrap();
            let width = crate::env::default_canvas_width();
            let height = crate::env::default_canvas_height();
            
            let default_canvas = Canvas::new(width, height).expect("Failed to create default canvas");
            let init_change = Change {
                event: ChangeEvent::Init { width, height },
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_millis() as u64,
            };
            let _ = Self::write_change_to_db(&mut conn, &init_change, Some(&default_canvas));
        }

        let initial_count: usize = conn_arc
            .lock()
            .unwrap()
            .query_row("SELECT COALESCE(MAX(id), 0) FROM events", [], |row| row.get::<_, i64>(0))
            .map(|val| val as usize)
            .unwrap_or(0);

        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<(Change, Option<Canvas>)>();

        let writer_conn = Arc::clone(&conn_arc);
        std::thread::spawn(move || {
            while let Some((_change, _canvas)) = rx.blocking_recv() {
                if let Ok(mut conn) = writer_conn.lock() {
                    let _ = Self::write_change_to_db(&mut conn, &_change, _canvas.as_ref());
                }
            }
        });

        Ok(Self {
            tx,
            conn: conn_arc,
            snapshot_interval,
            event_count: std::sync::atomic::AtomicUsize::new(initial_count),
        })
    }

    fn write_change_to_db(
        conn: &mut rusqlite::Connection,
        change: &Change,
        canvas: Option<&Canvas>,
    ) -> Result<i64, rusqlite::Error> {
        let tx = conn.transaction()?;

        let event_type_id = match &change.event {
            ChangeEvent::Init { .. } => 3,
            ChangeEvent::Paint { .. } => 0,
            ChangeEvent::Resize { .. } => 1,
            ChangeEvent::Rollback { .. } => 2,
        };

        tx.execute(
            "INSERT INTO events (event_type_id, created_at) VALUES (?1, ?2)",
            rusqlite::params![event_type_id, change.timestamp as i64],
        )?;
        let event_id = tx.last_insert_rowid();

        match &change.event {
            ChangeEvent::Init { .. } => {}
            ChangeEvent::Paint { x, y, color } => {
                tx.execute(
                    "INSERT INTO paint_event (event_id, x, y, color_hex) VALUES (?1, ?2, ?3, ?4)",
                    rusqlite::params![event_id, *x as i64, *y as i64, color.to_hex().to_string()],
                )?;
            }
            ChangeEvent::Resize { anchor, width, height } => {
                tx.execute(
                    "INSERT INTO resize_event (event_id, width, height, anchor_type) VALUES (?1, ?2, ?3, ?4)",
                    rusqlite::params![event_id, *width as i64, *height as i64, *anchor as u8],
                )?;
            }
            ChangeEvent::Rollback { target_event_id } => {
                tx.execute(
                    "INSERT INTO rollback_event (event_id, target_event_id) VALUES (?1, ?2)",
                    rusqlite::params![event_id, target_event_id],
                )?;
            }
        }

        if let Some(canvas) = canvas {
            let canvas_bytes = bincode::serialize(canvas).unwrap_or_default();
            tx.execute(
                "INSERT INTO snapshots (last_event_id, width, height, canvas_blob) VALUES (?1, ?2, ?3, ?4)",
                rusqlite::params![
                    event_id,
                    canvas.width() as i64,
                    canvas.height() as i64,
                    canvas_bytes
                ],
            )?;
        }

        tx.commit()?;
        Ok(event_id)
    }


    /// Record a new change and send it to the background writer thread,
    /// forcing a snapshot for resize/rollback events or regular intervals.
    pub fn record_change(&mut self, change: Change, current_canvas: &Canvas) {
        let current_count = self.event_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;

        let force_snapshot = matches!(
            change.event,
            ChangeEvent::Resize { .. } | ChangeEvent::Rollback { .. }
        );

        let canvas_snapshot = if force_snapshot || (current_count % self.snapshot_interval == 0) {
            Some(current_canvas.clone())
        } else {
            None
        };

        let _ = self.tx.send((change, canvas_snapshot));
    }

    /// Get the latest snapshot before or at the given target event ID
    pub fn latest_snapshot_before(&self, target_event_id: i64) -> Result<Option<(i64, Canvas)>, rusqlite::Error> {
        let conn = self.conn.lock().unwrap();
        Self::latest_snapshot_before_conn(&conn, target_event_id)
    }
    
    /// Get the current number of changes
    pub fn current_change_count(&self) -> usize {
        self.event_count.load(std::sync::atomic::Ordering::SeqCst)
    }

    // Internal helper that accepts an active connection borrow to prevent deadlocks
    fn latest_snapshot_before_conn(conn: &rusqlite::Connection, target_event_id: i64) -> Result<Option<(i64, Canvas)>, rusqlite::Error> {
        let mut stmt = conn.prepare(
            "SELECT last_event_id, canvas_blob FROM snapshots WHERE last_event_id <= ?1 ORDER BY last_event_id DESC LIMIT 1"
        )?;
        
        let mut rows = stmt.query(rusqlite::params![target_event_id])?;
        if let Some(row) = rows.next()? {
            let last_event_id: i64 = row.get(0)?;
            let blob: Vec<u8> = row.get(1)?;
            let canvas: Canvas = bincode::deserialize(&blob).map_err(|_| {
                rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Blob, Box::new(std::io::Error::new(std::io::ErrorKind::Other, "Failed to deserialize canvas blob")))
            })?;
            Ok(Some((last_event_id, canvas)))
        } else {
            Ok(None)
        }
    }

    /// Reconstruct a canvas from history by replaying all changes
    pub fn reconstruct_canvas(&self) -> Canvas {
        let conn = self.conn.lock().unwrap();

        // Always start from the latest snapshot in the database using the internal helper
        let (last_event_id, mut canvas) = match Self::latest_snapshot_before_conn(&conn, i64::MAX) {
            Ok(Some((id, cv))) => (id, cv),
            _ => panic!("History must have at least one snapshot"),
        };

        // Query all changes since that snapshot ID from the database
        let mut stmt = conn.prepare(
            r#"
            SELECT 
                e.event_type_id,
                p.x, p.y, p.color_hex,
                r.anchor_type, r.width, r.height
            FROM events e
            LEFT JOIN paint_event p ON e.id = p.event_id
            LEFT JOIN resize_event r ON e.id = r.event_id
            WHERE e.id > ?1
            ORDER BY e.id ASC
            "#
        ).expect("Failed to prepare canvas reconstruction query");

        let mut rows = stmt.query(rusqlite::params![last_event_id]).expect("Failed to execute reconstruction query");

        while let Some(row) = rows.next().expect("Failed to fetch event row") {
            let event_type_id: i64 = row.get(0).expect("Failed to get event_type_id");

            match event_type_id {
                0 => { // PAINT
                    let x: usize = row.get::<_, i64>(1).unwrap() as usize;
                    let y: usize = row.get::<_, i64>(2).unwrap() as usize;
                    let hex_str: String = row.get(3).unwrap();
                    let color = crate::world::color::Color::from_hex(&hex_str)
                        .expect("Failed to parse color hex from database");
                    let _ = canvas.set_pixel(x, y, color);
                }
                1 => { // RESIZE
                    let anchor_val: u8 = row.get::<_, i64>(4).unwrap() as u8;
                    let anchor = crate::history::change::ResizeAnchor::from_u8(anchor_val);
                    let width: usize = row.get::<_, i64>(5).unwrap() as usize;
                    let height: usize = row.get::<_, i64>(6).unwrap() as usize;
                    let _ = canvas.resize(width, height, anchor);
                }
                _ => {}
            }
        }

        canvas
    }

    /// Rollback to a specific change index (destructive)
    /// Index is 0-based. Truncates all changes after target_index.
    pub fn rollback_to_index(&mut self, target_index: usize) -> Result<(), RollbackError> {
        let target_id = target_index as i64;
        let mut conn = self.conn.lock().unwrap();

        let max_id: i64 = conn
            .query_row("SELECT COALESCE(MAX(id), 0) FROM events", [], |row| row.get(0))
            .unwrap_or(0);

        if target_id < 1 || target_id > max_id {
            return Err(RollbackError::IndexOutOfBounds {
                target: target_index,
                max: max_id.max(1) as usize - 1,
            });
        }

        let tx = conn.transaction()?;

        // Delete events after target_id. Cascades to paint_event, resize_event, rollback_event, and snapshots.
        tx.execute(
            "DELETE FROM events WHERE id > ?1",
            rusqlite::params![target_id],
        )?;

        tx.commit()?;

        let new_max: usize = conn
            .query_row("SELECT COALESCE(MAX(id), 0) FROM events", [], |row| row.get::<_, i64>(0))
            .map(|v| v as usize)
            .unwrap_or(0);
        self.event_count.store(new_max, std::sync::atomic::Ordering::SeqCst);

        Ok(())
    }
}
