use serde::{Deserialize, Serialize};
use crate::history::{HistoryChunk, HistoryLookahead, change::ResizeAnchor};

/// Messages sent from client to server
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ClientMessage {
    #[serde(rename = "paint")]
    Paint { x: usize, y: usize, color: String },
    
    #[serde(rename = "ping")]
    Ping,
    
    #[serde(rename = "resize")]
    Resize { width: usize, height: usize, anchor: ResizeAnchor },
    
    #[serde(rename = "rollback")]
    Rollback { target_index: usize },

    #[serde(rename = "get_event_count")]
    GetEventCount,

    #[serde(rename = "get_history")]
    GetHistory { 
        target_index: usize, 
        lookahead: HistoryLookahead,
    },
}

/// Messages sent from server to client
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ServerMessage {
    #[serde(rename = "init")]
    Init {
        width: usize,
        height: usize,
        palette: Vec<String>,
        board: Vec<u32>,
        cooldown: u64,
    },
    
    #[serde(rename = "update")]
    Update {
        x: usize,
        y: usize,
        color: String,
    },
    
    #[serde(rename = "pong")]
    Pong {
        clients: usize,
    },

    #[serde(rename = "event_count")]
    EventCount {
        total: usize,
    },

    #[serde(rename = "history_chunk")]
    HistoryChunk {
        chunk: HistoryChunk,
    },
}
