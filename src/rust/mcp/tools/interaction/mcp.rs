use anyhow::Result;
use rmcp::{Error as McpError, model::*};

use crate::mcp::{ZhiRequest, PopupRequest, McpResponse};
use crate::mcp::handlers::{create_tauri_popup, parse_mcp_response};
use crate::mcp::utils::{generate_request_id, popup_error};
use super::logger::{append_conversation_log, ConversationEntry};

/// 智能代码审查交互工具
///
/// 支持预定义选项、自由文本输入和图片上传
#[derive(Clone)]
pub struct InteractionTool;

impl InteractionTool {
    pub async fn zhi(
        request: ZhiRequest,
    ) -> Result<CallToolResult, McpError> {
        let ai_message = request.message.clone();
        let project_path = request.project_path.clone();
        
        let popup_request = PopupRequest {
            id: generate_request_id(),
            message: request.message,
            predefined_options: if request.predefined_options.is_empty() {
                None
            } else {
                Some(request.predefined_options)
            },
            is_markdown: request.is_markdown,
            project_path: request.project_path,
        };

        match create_tauri_popup(&popup_request) {
            Ok(response) => {
                // 记录对话日志
                log_conversation(&ai_message, &response, project_path);
                
                // 解析响应内容，支持文本和图片
                let content = parse_mcp_response(&response)?;
                Ok(CallToolResult::success(content))
            }
            Err(e) => {
                Err(popup_error(e.to_string()).into())
            }
        }
    }
}

/// 记录对话到日志
fn log_conversation(ai_message: &str, response: &str, project_path: Option<String>) {
    // 跳过取消操作
    if response.trim() == "CANCELLED" || response.trim() == "用户取消了操作" {
        return;
    }
    
    // 解析响应获取用户输入详情
    let (user_text, selected_options, image_count) = parse_response_for_log(response);
    
    let entry = ConversationEntry {
        ai_message: ai_message.to_string(),
        user_response: user_text,
        project_path,
        image_count,
        selected_options,
    };
    
    append_conversation_log(&entry);
}

/// 解析响应用于日志记录
fn parse_response_for_log(response: &str) -> (String, Vec<String>, usize) {
    // 尝试解析结构化格式
    if let Ok(structured) = serde_json::from_str::<McpResponse>(response) {
        let user_text = structured.user_input.unwrap_or_default();
        let selected_options = structured.selected_options;
        let image_count = structured.images.len();
        return (user_text, selected_options, image_count);
    }
    
    // 回退：直接作为文本
    (response.to_string(), vec![], 0)
}
