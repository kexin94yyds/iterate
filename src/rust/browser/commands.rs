use tauri::{AppHandle, Emitter};
use serde::{Deserialize, Serialize};

use super::detector::AiCompletionEvent;
use super::websocket::{start_ws_server, stop_ws_server};
use crate::mcp::handlers::create_tauri_popup;
use crate::mcp::types::PopupRequest;
use crate::mcp::utils::generate_request_id;

#[derive(Debug, Serialize, Deserialize)]
pub struct BrowserMonitorStatus {
    pub connected: bool,
    pub monitoring: bool,
}

/// 启动浏览器监控（WebSocket 模式）
#[tauri::command]
pub async fn start_browser_monitoring(
    app: AppHandle,
    _port: Option<u16>,
) -> Result<String, String> {
    match start_ws_server().await {
        Ok(event_tx) => {
            // 启动事件转发到前端
            let app_handle = app.clone();
            let mut receiver = event_tx.subscribe();
            tokio::spawn(async move {
                loop {
                    match receiver.recv().await {
                        Ok(event) => {
                            log::info!("AI 完成事件: {} - {}", event.site_name, event.url);
                            // 发送事件到前端显示在列表中
                            let _ = app_handle.emit("browser-ai-completed", &event);
                            
                            // 创建弹窗通知（像 cunzhi 那样）- 在独立线程中运行避免阻塞
                            let popup_request = PopupRequest {
                                id: generate_request_id(),
                                message: format!(
                                    "## {} AI 完成\n\n**标题**: {}",
                                    event.site_name, event.title
                                ),
                                predefined_options: Some(vec![
                                    "打开聊天页面".to_string(),
                                    "忽略".to_string(),
                                ]),
                                is_markdown: true,
                                project_path: None,
                                link_url: Some(event.url.clone()),
                                link_title: Some(event.title.clone()),
                            };
                            
                            let url = event.url.clone();
                            // 使用独立线程处理弹窗，不阻塞事件循环
                            std::thread::spawn(move || {
                                if let Ok(response) = create_tauri_popup(&popup_request) {
                                    if response.contains("打开") {
                                        #[cfg(target_os = "macos")]
                                        {
                                            let _ = std::process::Command::new("open").arg(&url).spawn();
                                        }
                                    }
                                }
                            });
                        }
                        Err(e) => {
                            log::warn!("事件接收错误: {}, 重新订阅", e);
                            break;
                        }
                    }
                }
            });
            
            Ok("WebSocket 服务器已启动 (端口 9333)".to_string())
        }
        Err(e) => Err(e.to_string())
    }
}

/// 停止浏览器监控
#[tauri::command]
pub async fn stop_browser_monitoring() -> Result<String, String> {
    stop_ws_server().await;
    Ok("浏览器监控已停止".to_string())
}

/// 获取浏览器监控状态
#[tauri::command]
pub async fn get_browser_monitor_status() -> Result<BrowserMonitorStatus, String> {
    // WebSocket 模式下简化状态返回
    Ok(BrowserMonitorStatus {
        connected: true,
        monitoring: true,
    })
}

/// 打开浏览器页面（通过 URL）
#[tauri::command]
pub async fn open_browser_url(url: String) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(&url)
            .spawn()
            .map_err(|e| format!("打开 URL 失败: {}", e))?;
    }
    
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("cmd")
            .args(["/C", "start", &url])
            .spawn()
            .map_err(|e| format!("打开 URL 失败: {}", e))?;
    }
    
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(&url)
            .spawn()
            .map_err(|e| format!("打开 URL 失败: {}", e))?;
    }
    
    Ok(())
}

/// 显示 AI 完成通知弹窗
#[tauri::command]
pub async fn show_ai_completion_popup(
    app: AppHandle,
    event: AiCompletionEvent,
) -> Result<(), String> {
    // 发送到前端显示弹窗
    app.emit("show-ai-completion-popup", &event)
        .map_err(|e| format!("发送弹窗事件失败: {}", e))
}
