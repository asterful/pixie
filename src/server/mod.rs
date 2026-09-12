mod messages;
mod rate_limit;

use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::tungstenite::Message;
use futures_util::{StreamExt, SinkExt};
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use tokio::sync::RwLock;
use std::collections::HashMap;
use messages::{ClientMessage, ServerMessage};
use crate::world::{World, color::Color};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Role {
    Admin,
    Player,
}

struct ClientInfo {
    sender: tokio::sync::mpsc::UnboundedSender<Message>,
    role: Role,
}

type Clients = Arc<RwLock<HashMap<SocketAddr, ClientInfo>>>;

pub struct Server {
    addr: String,
    clients: Clients,
    world: Arc<RwLock<World>>,
}

impl Server {
    pub fn new(addr: impl Into<String>) -> Self {
        println!("DEBUG: Entering Server::new()");

        println!("DEBUG: Opening history database...");
        
        let history = crate::history::History::open(
            crate::env::db_path(),
            crate::env::default_snapshot_interval()
        ).expect("Failed to open history database");
        
        println!("DEBUG: Reconstructing world from history...");
        let world = World::from(history);
        
        println!("DEBUG: Server struct built successfully.");
        Self {
            addr: addr.into(),
            clients: Arc::new(RwLock::new(HashMap::new())),
            world: Arc::new(RwLock::new(world)),
        }
    }

    pub async fn run(&self) -> Result<(), Box<dyn std::error::Error>> {
        let listener = TcpListener::bind(&self.addr).await?;
        println!("Server listening on {}", self.addr);
        

        while let Ok((stream, addr)) = listener.accept().await {
            println!("New connection from {}", addr);
            tokio::spawn(Self::handle_connection(
                stream, 
                addr, 
                self.clients.clone(), 
                self.world.clone()
            ));
        }

        Ok(())
    }

    fn build_init_message(world: &World) -> ServerMessage {
        let width = world.canvas.width();
        let height = world.canvas.height();
        
        // Get palette and pixel indices
        let pixels = world.canvas.pixels();
        let palette = world.canvas.palette();
        let palette_colors = palette.colors().to_vec();
        
        ServerMessage::Init {
            width,
            height,
            palette: palette_colors,
            board: pixels.to_vec(),
            cooldown: 0,
        }
    }

    async fn handle_connection(stream: TcpStream, addr: SocketAddr, clients: Clients, world: Arc<RwLock<World>>) {
        use tokio_tungstenite::tungstenite::handshake::server::{Request, Response};
        
        // Extract query parameters from the WebSocket handshake
        let query_params = Arc::new(Mutex::new(HashMap::new()));
        let params_for_callback = query_params.clone();
        
        let ws_stream = match tokio_tungstenite::accept_hdr_async(stream, |req: &Request, resp: Response| {
            // Extract query string from the request URI
            if let Some(query_string) = req.uri().query() {
                let mut params = HashMap::new();
                for param in query_string.split('&') {
                    if let Some((key, value)) = param.split_once('=') {
                        params.insert(
                            key.to_string(),
                            urlencoding::decode(value).unwrap_or_default().to_string()
                        );
                    }
                }
                if let Ok(mut p) = params_for_callback.lock() {
                    *p = params;
                }
            }
            Ok(resp)
        }).await {
            Ok(stream) => stream,
            Err(e) => {
                eprintln!("WebSocket handshake failed with {}: {}", addr, e);
                return;
            }
        };
        
        let query_params = query_params.lock().unwrap().clone();
        println!("Query parameters for {}: {:?}", addr, query_params);

        // Determine role based on auth parameter
        let role = match query_params.get("auth") {
            Some(token) if token == crate::env::admin_token() => Role::Admin,
            _ => Role::Player,
        };
        println!("Client {} connected as {:?}", addr, role);

        let (mut ws_sender, mut ws_receiver) = ws_stream.split();
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();

        // Store the client with their role
        clients.write().await.insert(addr, ClientInfo {
            sender: tx.clone(),
            role,
        });

        // Send init message immediately
        let init_msg = {
            let world_lock = world.read().await;
            Self::build_init_message(&world_lock)
        };
        
        if let Ok(json) = serde_json::to_string(&init_msg) {
            if tx.send(Message::Text(json)).is_err() {
                eprintln!("Failed to send init message to {}", addr);
                clients.write().await.remove(&addr);
                return;
            }
        }

        // Spawn task to handle outgoing messages
        let send_task = tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                if ws_sender.send(msg).await.is_err() {
                    break;
                }
            }
        });


        let mut rate_limiter = rate_limit::RateLimiter::new(
            crate::env::rate_limit_tokens(),
            crate::env::rate_limit_refill_rate_ms()
        );

        // Handle incoming messages
        while let Some(msg) = ws_receiver.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    Self::handle_json_message(&clients, &world, &text, addr, &mut rate_limiter).await;
                }
                Ok(Message::Binary(bin)) => {
                    Self::broadcast_message(&clients, Message::Binary(bin), addr).await;
                }
                Ok(Message::Close(_)) => {
                    println!("Client {} closed connection", addr);
                    break;
                }
                Ok(Message::Ping(data)) => {
                    if let Some(client_info) = clients.read().await.get(&addr) {
                        client_info.sender.send(Message::Pong(data)).ok();
                    }
                }
                Ok(Message::Pong(_)) => {}
                Ok(Message::Frame(_)) => {}
                Err(e) => {
                    eprintln!("WebSocket error for {}: {}", addr, e);
                    break;
                }
            }
        }

        // Clean up
        println!("Client {} disconnected", addr);
        send_task.abort();
        clients.write().await.remove(&addr);
    }

    async fn handle_json_message(clients: &Clients, world: &Arc<RwLock<World>>, text: &str, sender: SocketAddr, rate_limiter: &mut rate_limit::RateLimiter) {
        let client_msg: ClientMessage = match serde_json::from_str(text) {
            Ok(msg) => msg,
            Err(e) => {
                eprintln!("Failed to parse JSON from {}: {} ({})", sender, e, text);
                return;
            }
        };

        match client_msg {
            ClientMessage::Paint { x, y, color } => {
                let is_admin = {
                    let clients_lock = clients.read().await;
                    clients_lock.get(&sender).map(|info| info.role == Role::Admin).unwrap_or(false)
                };

                // Enforce rate limit for non-admin clients
                if !is_admin && !rate_limiter.take() {
                    eprintln!("Rate limit exceeded for client {}", sender);
                    return;
                }

                // Validate color format
                if !color.starts_with('#') || color.len() != 7 {
                    eprintln!("Invalid color format from {}: {}", sender, color);
                    return;
                }
                
                // Parse color
                let parsed_color = match Color::from_hex(&color) {
                    Ok(c) => c,
                    Err(e) => {
                        eprintln!("Invalid color from {}: {} ({})", sender, color, e);
                        return;
                    }
                };
                
                // Apply the paint operation
                let color_for_broadcast = parsed_color.clone();
                let result = {
                    let mut world_lock = world.write().await;
                    let paint_event = crate::history::change::ChangeEvent::Paint {
                        x,
                        y,
                        color: parsed_color,
                    };
                    world_lock.apply_event(paint_event)
                };
                
                match result {
                    Ok(_) => {
                        // Broadcast update to all clients (including sender)
                        let update_msg = ServerMessage::Update {
                            x,
                            y,
                            color: color_for_broadcast.to_hex().to_string(),
                        };
                        
                        if let Ok(json) = serde_json::to_string(&update_msg) {
                            Self::broadcast_to_all(clients, Message::Text(json)).await;
                        }
                    }
                    Err(e) => {
                        eprintln!("Failed to paint pixel from {}: {:?}", sender, e);
                    }
                }
            }
            ClientMessage::Ping => {
                // Respond with current client count
                let client_count = clients.read().await.len();
                let pong_msg = ServerMessage::Pong {
                    clients: client_count,
                };
                
                if let Ok(json) = serde_json::to_string(&pong_msg) {
                    if let Some(client_info) = clients.read().await.get(&sender) {
                        client_info.sender.send(Message::Text(json)).ok();
                    }
                }
            }
            ClientMessage::Resize { width, height, anchor } => {
                // Check if sender is admin
                let is_admin = {
                    let clients_lock = clients.read().await;
                    clients_lock.get(&sender)
                        .map(|info| info.role == Role::Admin)
                        .unwrap_or(false)
                };
                
                if !is_admin {
                    eprintln!("Non-admin client {} attempted to resize canvas", sender);
                    return;
                }
                
                // Validate dimensions
                if width == 0 || height == 0 {
                    eprintln!("Invalid resize dimensions from {}: {}x{}", sender, width, height);
                    return;
                }
                
                // Apply the resize operation
                let result = {
                    let mut world_lock = world.write().await;
                    let resize_event = crate::history::change::ChangeEvent::Resize {
                        anchor,
                        width,
                        height,
                    };
                    world_lock.apply_event(resize_event)
                };
                
                match result {
                    Ok(_) => {
                        // Send new Init message to all clients with updated board state
                        let init_msg = {
                            let world_lock = world.read().await;
                            Self::build_init_message(&world_lock)
                        };
                        
                        if let Ok(json) = serde_json::to_string(&init_msg) {
                            Self::broadcast_to_all(clients, Message::Text(json)).await;
                        }
                        
                        println!("Canvas resized to {}x{} with anchor {:?} by admin {}", width, height, anchor, sender);
                    }
                    Err(e) => {
                        eprintln!("Failed to resize canvas from {}: {:?}", sender, e);
                    }
                }
            }
            ClientMessage::Rollback { target_index } => {
                // Check if sender is admin
                let is_admin = {
                    let clients_lock = clients.read().await;
                    clients_lock.get(&sender)
                        .map(|info| info.role == Role::Admin)
                        .unwrap_or(false)
                };
                
                if !is_admin {
                    eprintln!("Non-admin client {} attempted to rollback canvas", sender);
                    return;
                }
                
                // Apply the rollback operation
                let result = {
                    let mut world_lock = world.write().await;
                    world_lock.rollback_to_index(target_index)
                };
                
                match result {
                    Ok(_) => {
                        // Send new Init message to all clients with rolled-back board state
                        let init_msg = {
                            let world_lock = world.read().await;
                            Self::build_init_message(&world_lock)
                        };
                        
                        if let Ok(json) = serde_json::to_string(&init_msg) {
                            Self::broadcast_to_all(clients, Message::Text(json)).await;
                        }
                        
                        println!("Canvas rolled back to index {} by admin {}", target_index, sender);
                    }
                    Err(e) => {
                        eprintln!("Failed to rollback canvas from {}: {:?}", sender, e);
                    }
                }
            }
            ClientMessage::GetEventCount => {
                let count_result = {
                    let world_lock = world.read().await;
                    world_lock.history.db_event_count()
                };

                match count_result {
                    Ok(total) => {
                        let msg = ServerMessage::EventCount { total };
                        if let Ok(json) = serde_json::to_string(&msg) {
                            if let Some(client_info) = clients.read().await.get(&sender) {
                                client_info.sender.send(Message::Text(json)).ok();
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Failed to fetch event count for {}: {:?}", sender, e);
                    }
                }
            }
            ClientMessage::GetHistory { target_index, lookahead } => {
                let chunk_result = {
                    let world_lock = world.read().await;
                    world_lock.history.get_history_chunk(target_index as u64, lookahead)
                };

                match chunk_result {
                    Ok(chunk) => {
                        let msg = ServerMessage::HistoryChunk { chunk };
                        if let Ok(json) = serde_json::to_string(&msg) {
                            if let Some(client_info) = clients.read().await.get(&sender) {
                                client_info.sender.send(Message::Text(json)).ok();
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Failed to fetch history chunk for {}: {:?}", sender, e);
                    }
                }
            }
        }
    }

    async fn broadcast_message(clients: &Clients, msg: Message, sender: SocketAddr) {
        let clients = clients.read().await;
        for (addr, client_info) in clients.iter() {
            if *addr != sender {
                client_info.sender.send(msg.clone()).ok();
            }
        }
    }
    
    async fn broadcast_to_all(clients: &Clients, msg: Message) {
        let clients = clients.read().await;
        for client_info in clients.values() {
            client_info.sender.send(msg.clone()).ok();
        }
    }
}
