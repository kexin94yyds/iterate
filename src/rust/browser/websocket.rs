use anyhow::Result;
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc, RwLock};
use tokio::net::{TcpListener, TcpStream};
use futures::{StreamExt, SinkExt};
use tokio_tungstenite::{accept_async, tungstenite::Message};

use super::detector::AiCompletionEvent;

const WS_PORT: u16 = 9333;

/// 发送到浏览器的消息
#[derive(Debug, Clone)]
pub struct BrowserMessage {
    pub message_type: String,
    pub message: String,
    pub tab_id: Option<u32>,
}

/// 全局消息发送通道
static BROWSER_TX: once_cell::sync::Lazy<Arc<RwLock<Option<mpsc::Sender<BrowserMessage>>>>> =
    once_cell::sync::Lazy::new(|| Arc::new(RwLock::new(None)));

/// 发送消息到浏览器
pub async fn send_to_browser(message: String) -> Result<()> {
    log::info!("尝试发送消息到浏览器: {}", message);
    
    // 先尝试通过 channel 发送（主应用进程）
    {
        let tx = BROWSER_TX.read().await;
        if let Some(ref sender) = *tx {
            let msg = BrowserMessage {
                message_type: "send_message".to_string(),
                message: message.clone(),
                tab_id: None,
            };
            log::info!("发送消息通过 channel...");
            match sender.send(msg).await {
                Ok(_) => {
                log::info!("消息已发送到 channel");
                return Ok(());
                }
                Err(e) => {
                    log::warn!("Channel 发送失败: {}，连接可能已断开", e);
                    // 清理失效的发送器
                    drop(tx);
                    let mut tx_write = BROWSER_TX.write().await;
                    *tx_write = None;
                }
            }
        }
    }
    
    // 如果 channel 不可用，作为客户端直接连接发送（弹窗进程）
    log::info!("Channel 不可用，尝试作为客户端发送...");
    send_as_client(message).await
}

/// 作为 WebSocket 客户端发送消息（用于弹窗进程）
async fn send_as_client(message: String) -> Result<()> {
    use tokio_tungstenite::connect_async;
    
    let url = format!("ws://127.0.0.1:{}", WS_PORT);
    log::info!("连接到 WebSocket 服务器: {}", url);
    
    let (ws_stream, _) = connect_async(&url).await
        .map_err(|e| anyhow::anyhow!("连接 WebSocket 失败: {}", e))?;
    
    let (mut write, _read) = ws_stream.split();
    
    let json = serde_json::json!({
        "type": "send_message",
        "message": message,
    });
    
    log::info!("发送消息: {}", json);
    write.send(Message::Text(json.to_string())).await
        .map_err(|e| anyhow::anyhow!("发送消息失败: {}", e))?;
    
    log::info!("消息已发送");
    Ok(())
}

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

        // 创建消息通道
        let (browser_tx, browser_rx) = mpsc::channel::<BrowserMessage>(100);
        
        // 存储发送器到全局变量
        {
            let mut tx = BROWSER_TX.write().await;
            *tx = Some(browser_tx);
        }

        tokio::spawn(async move {
            while *running.read().await {
                match listener.accept().await {
                    Ok((stream, addr)) => {
                        log::info!("新的 WebSocket 连接: {}", addr);
                        let event_tx = event_tx.clone();
                        // 为每个连接创建消息通道
                        let (new_tx, new_rx) = mpsc::channel::<BrowserMessage>(100);
                        // 传递发送器，连接处理器会在确认是浏览器扩展后更新 BROWSER_TX
                        tokio::spawn(handle_connection(stream, event_tx, new_rx, new_tx));
                    }
                    Err(e) => {
                        log::error!("接受连接失败: {}", e);
                    }
                }
            }
        });
        
        // 消耗初始的 receiver（不使用）
        drop(browser_rx);

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
    mut browser_rx: mpsc::Receiver<BrowserMessage>,
    browser_tx: mpsc::Sender<BrowserMessage>,
) {
    let mut is_browser_extension = false;
    let ws_stream = match accept_async(stream).await {
        Ok(ws) => ws,
        Err(e) => {
            log::error!("WebSocket 握手失败: {}", e);
            return;
        }
    };

    let (mut write, mut read) = ws_stream.split();

    loop {
        tokio::select! {
            // 接收浏览器扩展的消息
            msg = read.next() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        log::debug!("收到消息: {}", text);
                        
                        // 解析收到的消息
                        if let Ok(data) = serde_json::from_str::<serde_json::Value>(&text) {
                            let msg_type = data.get("type").and_then(|t| t.as_str()).unwrap_or("");
                            
                            match msg_type {
                                "ai_completed" => {
                                    // 浏览器扩展发来的 AI 完成事件
                                    // 确认这是浏览器扩展连接，更新 BROWSER_TX
                                    if !is_browser_extension {
                                        is_browser_extension = true;
                                        log::info!("确认为浏览器扩展连接，更新 BROWSER_TX");
                                        let mut tx = BROWSER_TX.write().await;
                                        *tx = Some(browser_tx.clone());
                                    }
                                    
                                    // 获取 AI 回复内容
                                    log::info!("收到的完整数据: {:?}", data);
                                    let ai_response = data.get("aiResponse").and_then(|v| v.as_str()).unwrap_or("").to_string();
                                    log::info!("AI 回复内容长度: {}, 内容前100字符: {}", ai_response.len(), ai_response.chars().take(100).collect::<String>());

                                    let event = AiCompletionEvent {
                                        url: data.get("url").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                                        title: data.get("title").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                                        site_name: data.get("siteName").and_then(|v| v.as_str()).unwrap_or("Unknown").to_string(),
                                        message_preview: ai_response, // 存储 AI 回复内容
                                        timestamp: chrono::Utc::now(),
                                        run_time: data.get("runTime").and_then(|v| v.as_u64()).map(|v| v as u32),
                                        think_time: data.get("thinkTime").and_then(|v| v.as_u64()).map(|v| v as u32),
                                        image_generated: data.get("imageGenerated").and_then(|v| v.as_bool()).unwrap_or(false),
                                        new_images: data.get("newImages").and_then(|v| v.as_u64()).map(|v| v as u32),
                                    };

                                    log::info!("AI 完成事件: {} - {}", event.site_name, event.url);
                                    let _ = event_tx.send(event);
                                }
                                "send_message" => {
                                    // 弹窗进程发来的消息，需要转发给浏览器扩展
                                    if let Some(message) = data.get("message").and_then(|v| v.as_str()) {
                                        log::info!("收到 send_message 请求，转发给浏览器扩展: {}", message);
                                        let msg = BrowserMessage {
                                            message_type: "send_message".to_string(),
                                            message: message.to_string(),
                                            tab_id: None,
                                        };
                                        // 通过 channel 转发给浏览器扩展连接
                                        let tx = BROWSER_TX.read().await;
                                        if let Some(ref sender) = *tx {
                                            if let Err(e) = sender.send(msg).await {
                                                log::error!("转发消息失败: {}", e);
                                            } else {
                                                log::info!("消息已转发到浏览器扩展");
                                            }
                                        } else {
                                            log::warn!("没有浏览器扩展连接，无法转发消息");
                                        }
                                    }
                                }
                                "ping" => {
                                    // 心跳消息，忽略
                                }
                                _ => {
                                    log::debug!("未知消息类型: {}", msg_type);
                                }
                            }
                        }
                        
                        // 回复确认
                        if let Err(e) = write.send(Message::Text(r#"{"status":"ok"}"#.to_string())).await {
                            log::error!("发送回复失败: {}", e);
                            break;
                        }
                    }
                    Some(Ok(Message::Close(_))) => {
                        log::info!("WebSocket 连接关闭");
                        break;
                    }
                    Some(Ok(Message::Ping(data))) => {
                        let _ = write.send(Message::Pong(data)).await;
                    }
                    Some(Err(e)) => {
                        log::error!("WebSocket 错误: {}", e);
                        break;
                    }
                    None => break,
                    _ => {}
                }
            }
            // 发送消息到浏览器扩展
            browser_msg = browser_rx.recv() => {
                if let Some(msg) = browser_msg {
                    let json = serde_json::json!({
                        "type": msg.message_type,
                        "message": msg.message,
                        "tabId": msg.tab_id,
                    });
                    log::info!("发送消息到浏览器: {}", json);
                    if let Err(e) = write.send(Message::Text(json.to_string())).await {
                        log::error!("发送消息到浏览器失败: {}", e);
                        break;
                    }
                }
            }
        }
    }

    // 连接关闭时，如果是浏览器扩展连接，清理 BROWSER_TX
    if is_browser_extension {
        let mut tx = BROWSER_TX.write().await;
        *tx = None;
        log::info!("浏览器扩展连接关闭，已清理 BROWSER_TX");
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
