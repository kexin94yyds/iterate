use anyhow::Result;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use tokio::net::{TcpListener, TcpStream};
use futures::{StreamExt, SinkExt};
use tokio_tungstenite::{accept_async, tungstenite::Message};

use super::detector::AiCompletionEvent;

const WS_PORT: u16 = 9333;

/// WebSocket 服务器状态
pub struct WsServer {
    event_tx: broadcast::Sender<AiCompletionEvent>,
    running: Arc<RwLock<bool>>,
}

impl WsServer {
    pub fn new() -> Self {
        let (event_tx, _) = broadcast::channel(100);
        Self {
            event_tx,
            running: Arc::new(RwLock::new(false)),
        }
    }

    /// 获取事件发送器
    pub fn get_event_sender(&self) -> broadcast::Sender<AiCompletionEvent> {
        self.event_tx.clone()
    }

    /// 启动 WebSocket 服务器
    pub async fn start(&self) -> Result<()> {
        let addr = format!("127.0.0.1:{}", WS_PORT);
        let listener = TcpListener::bind(&addr).await?;
        
        log::info!("WebSocket 服务器已启动: {}", addr);
        
        *self.running.write().await = true;
        let running = self.running.clone();
        let event_tx = self.event_tx.clone();

        tokio::spawn(async move {
            while *running.read().await {
                match listener.accept().await {
                    Ok((stream, addr)) => {
                        log::info!("新的 WebSocket 连接: {}", addr);
                        let event_tx = event_tx.clone();
                        tokio::spawn(handle_connection(stream, event_tx));
                    }
                    Err(e) => {
                        log::error!("接受连接失败: {}", e);
                    }
                }
            }
        });

        Ok(())
    }

    /// 停止服务器
    pub async fn stop(&self) {
        *self.running.write().await = false;
    }
}

/// 处理单个 WebSocket 连接
async fn handle_connection(
    stream: TcpStream,
    event_tx: broadcast::Sender<AiCompletionEvent>,
) {
    let ws_stream = match accept_async(stream).await {
        Ok(ws) => ws,
        Err(e) => {
            log::error!("WebSocket 握手失败: {}", e);
            return;
        }
    };

    let (mut write, mut read) = ws_stream.split();

    while let Some(msg) = read.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                log::debug!("收到消息: {}", text);
                
                // 解析浏览器扩展发来的消息
                if let Ok(data) = serde_json::from_str::<serde_json::Value>(&text) {
                    if data.get("type").and_then(|t| t.as_str()) == Some("ai_completed") {
                        let event = AiCompletionEvent {
                            url: data.get("url").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                            title: data.get("title").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                            site_name: data.get("siteName").and_then(|v| v.as_str()).unwrap_or("Unknown").to_string(),
                            message_preview: String::new(),
                            timestamp: chrono::Utc::now(),
                        };
                        
                        log::info!("AI 完成事件: {} - {}", event.site_name, event.url);
                        let _ = event_tx.send(event);
                    }
                }
                
                // 回复确认
                if let Err(e) = write.send(Message::Text(r#"{"status":"ok"}"#.to_string())).await {
                    log::error!("发送回复失败: {}", e);
                    break;
                }
            }
            Ok(Message::Close(_)) => {
                log::info!("WebSocket 连接关闭");
                break;
            }
            Ok(Message::Ping(data)) => {
                let _ = write.send(Message::Pong(data)).await;
            }
            Err(e) => {
                log::error!("WebSocket 错误: {}", e);
                break;
            }
            _ => {}
        }
    }
}

/// 全局 WebSocket 服务器实例
static WS_SERVER: once_cell::sync::Lazy<Arc<RwLock<Option<WsServer>>>> =
    once_cell::sync::Lazy::new(|| Arc::new(RwLock::new(None)));

/// 启动 WebSocket 服务器（如果已运行则返回现有的 sender）
pub async fn start_ws_server() -> Result<broadcast::Sender<AiCompletionEvent>> {
    let mut global = WS_SERVER.write().await;
    
    // 如果服务器已经在运行，返回现有的 sender
    if let Some(ref server) = *global {
        if *server.running.read().await {
            log::info!("WebSocket 服务器已在运行，返回现有 sender");
            return Ok(server.get_event_sender());
        }
    }
    
    // 创建新的服务器
    let server = WsServer::new();
    let event_tx = server.get_event_sender();
    server.start().await?;
    *global = Some(server);
    
    Ok(event_tx)
}

/// 停止 WebSocket 服务器
pub async fn stop_ws_server() {
    let mut global = WS_SERVER.write().await;
    if let Some(server) = global.take() {
        server.stop().await;
    }
}
