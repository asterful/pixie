use Pixie::history::change::{ChangeEvent, ResizeAnchor};
use Pixie::history::{History, HistoryChunk, HistoryLookahead};
use Pixie::world::canvas::Canvas;
use Pixie::world::color::Color;
use Pixie::world::World;
use std::path::PathBuf;
use std::sync::Once;
use std::time::{Duration, Instant, SystemTime};

static INIT: Once = Once::new();

fn setup_large_test_world(test_name: &str, total_events: usize) -> (World, PathBuf) {
    INIT.call_once(|| {
        let _ = Pixie::env::init();
    });

    let unique_id = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let db_path = std::env::temp_dir().join(format!("{}_{}.db", test_name, unique_id));

    let history = History::open(&db_path, 5).expect("Failed to open history db");
    let mut world = World::from(history);

    let red = Color::from_hex("#FF0000").unwrap();
    let blue = Color::from_hex("#0000FF").unwrap();
    let green = Color::from_hex("#00FF00").unwrap();

    for i in 1..=total_events {
        let change = if i == 1 {
            ChangeEvent::Resize {
                anchor: ResizeAnchor::TopLeft,
                width: 16,
                height: 16,
            }
        } else if i % 3 == 0 {
            ChangeEvent::Paint {
                x: i % 16,
                y: (i / 2) % 16,
                color: red.clone(),
            }
        } else if i % 3 == 1 {
            ChangeEvent::Paint {
                x: i % 16,
                y: (i / 2) % 16,
                color: blue.clone(),
            }
        } else {
            ChangeEvent::Paint {
                x: i % 16,
                y: (i / 2) % 16,
                color: green.clone(),
            }
        };
        world.apply_event(change);
    }

    let expected_rows = (total_events + 1) as i64;
    let start = Instant::now();
    loop {
        if let Ok(conn) = rusqlite::Connection::open(&db_path) {
            let _ = conn.busy_timeout(Duration::from_millis(500));
            let count: i64 = conn
                .query_row("SELECT COUNT(*) FROM events", [], |r| r.get(0))
                .unwrap_or(0);

            if count >= expected_rows {
                break;
            }
        }

        if start.elapsed() > Duration::from_secs(5) {
            panic!(
                "Timeout waiting for writer thread to commit {} events",
                expected_rows
            );
        }
        std::thread::sleep(Duration::from_millis(10));
    }

    (world, db_path)
}

fn print_client_chunk(
    step_name: &str,
    target_id: u64,
    lookahead: HistoryLookahead,
    chunk: &HistoryChunk,
) {
    let fmt_snap = |snap: &Option<(i64, Canvas)>| match snap {
        Some((id, c)) => format!("Snapshot @ Event {:2} ({}x{})", id, c.width(), c.height()),
        None => "None".to_string(),
    };

    let mode_str = match lookahead {
        HistoryLookahead::Full => "Full",
        HistoryLookahead::Forward => "Forward",
        HistoryLookahead::Backward => "Backward",
    };

    println!("\n========================================================");
    println!(" [CLIENT STEP: {}]", step_name);
    println!(" Target Event : {}", target_id);
    println!(" Mode         : {}", mode_str);
    println!("--------------------------------------------------------");
    println!(" Prev Snapshot      : {}", fmt_snap(&chunk.prev_snapshot));
    println!(" Current Snapshot   : {}", fmt_snap(&chunk.current_snapshot));
    println!(" Next Snapshot      : {}", fmt_snap(&chunk.next_snapshot));
    println!(" Next Next Snapshot : {}", fmt_snap(&chunk.next_next_snapshot));
    println!("--------------------------------------------------------");

    let event_ids: Vec<i64> = chunk.events.iter().map(|(id, _)| *id).collect();
    let min_event = event_ids.first().copied().unwrap_or(0);
    let max_event = event_ids.last().copied().unwrap_or(0);

    println!(
        " Fetched Events Count : {} (ID Range: {}..={})",
        chunk.events.len(),
        min_event,
        max_event
    );
    println!(" Events Stream:");

    let prev_snap_id = chunk.prev_snapshot.as_ref().map(|(id, _)| *id);
    let curr_snap_id = chunk.current_snapshot.as_ref().map(|(id, _)| *id);
    let next_snap_id = chunk.next_snapshot.as_ref().map(|(id, _)| *id);
    let next_next_snap_id = chunk.next_next_snapshot.as_ref().map(|(id, _)| *id);

    for (event_id, change) in &chunk.events {
        let mut tags = Vec::new();

        if Some(*event_id) == prev_snap_id {
            tags.push("SNAPSHOT: Prev");
        }
        if Some(*event_id) == curr_snap_id {
            tags.push("SNAPSHOT: Current");
        }
        if Some(*event_id) == next_snap_id {
            tags.push("SNAPSHOT: Next");
        }
        if Some(*event_id) == next_next_snap_id {
            tags.push("SNAPSHOT: NextNext");
        }
        if *event_id == target_id as i64 {
            tags.push("CLIENT REPLAY TARGET");
        }

        let marker = if tags.is_empty() {
            String::new()
        } else {
            format!(" <--- [{}]", tags.join(" | "))
        };

        println!("    {:2}: {:?}{}", event_id, change, marker);
    }
    println!("========================================================\n");
}

#[test]
fn test_client_scrubbing_sequence() {
    let (world, db_path) = setup_large_test_world("scrubbing_sim", 50);

    // 1. Initial jump to middle using FULL lookahead (target = 24)
    let chunk_mid = world
        .history
        .get_history_chunk(24, HistoryLookahead::Full)
        .expect("Failed to fetch mid chunk");
    print_client_chunk(
        "1. Middle Scrub (Full Window)",
        24,
        HistoryLookahead::Full,
        &chunk_mid,
    );

    // 2. Scrub backward into Q1 using BACKWARD lookahead (target = 19)
    let chunk_q1 = world
        .history
        .get_history_chunk(19, HistoryLookahead::Backward)
        .expect("Failed to fetch Q1 chunk");
    print_client_chunk(
        "2. Backward Scrub Q1 (Backward Window)",
        19,
        HistoryLookahead::Backward,
        &chunk_q1,
    );

    // 3. Scrub forward into Q2 using FORWARD lookahead (target = 26)
    let chunk_q2 = world
        .history
        .get_history_chunk(26, HistoryLookahead::Forward)
        .expect("Failed to fetch Q2 chunk");
    print_client_chunk(
        "3. Forward Scrub Q2 (Forward Window)",
        26,
        HistoryLookahead::Forward,
        &chunk_q2,
    );

    let _ = std::fs::remove_file(&db_path);
}