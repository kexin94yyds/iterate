//! 对话日志记录模块
//!
//! 自动记录 zhi 工具的 AI 提问和用户回答

use chrono::Local;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;

/// 对话日志条目
pub struct ConversationEntry {
    pub ai_message: String,
    pub user_response: String,
    pub project_path: Option<String>,
    pub image_count: usize,
    pub selected_options: Vec<String>,
}

/// 追加对话日志到 .cunzhi-knowledge/conversations/
pub fn append_conversation_log(entry: &ConversationEntry) {
    if let Err(e) = append_conversation_log_inner(entry) {
        // 静默失败，不影响主流程
        eprintln!("[cunzhi] 对话日志记录失败: {}", e);
    }
}

fn append_conversation_log_inner(entry: &ConversationEntry) -> std::io::Result<()> {
    // 查找 .cunzhi-knowledge 目录
    let knowledge_dir = find_knowledge_dir(entry.project_path.as_deref())?;
    let conversations_dir = knowledge_dir.join("conversations");
    
    // 确保目录存在
    fs::create_dir_all(&conversations_dir)?;
    
    // 按日期分文件
    let today = Local::now().format("%Y-%m-%d").to_string();
    let log_file = conversations_dir.join(format!("{}.md", today));
    
    // 生成日志条目
    let timestamp = Local::now().format("%H:%M:%S").to_string();
    let log_content = format_log_entry(entry, &timestamp);
    
    // 追加到文件
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_file)?;
    
    file.write_all(log_content.as_bytes())?;
    
    Ok(())
}

/// 查找 .cunzhi-knowledge 目录
fn find_knowledge_dir(project_path: Option<&str>) -> std::io::Result<PathBuf> {
    // 优先从项目路径查找
    if let Some(path) = project_path {
        let project_knowledge = PathBuf::from(path).join(".cunzhi-knowledge");
        if project_knowledge.exists() {
            return Ok(project_knowledge);
        }
    }
    
    // 从 HOME 目录查找
    if let Some(home) = dirs::home_dir() {
        // 检查常见位置
        let candidates = [
            home.join("cunzhi/.cunzhi-knowledge"),
            home.join(".cunzhi-knowledge"),
        ];
        
        for candidate in candidates {
            if candidate.exists() {
                return Ok(candidate);
            }
        }
    }
    
    Err(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        "找不到 .cunzhi-knowledge 目录",
    ))
}

/// 格式化日志条目
fn format_log_entry(entry: &ConversationEntry, timestamp: &str) -> String {
    let mut content = String::new();
    
    // 标题行：时间戳 + 项目（如果有）
    let project_info = entry.project_path
        .as_ref()
        .and_then(|p| PathBuf::from(p).file_name().map(|n| n.to_string_lossy().to_string()))
        .map(|name| format!(" @ {}", name))
        .unwrap_or_default();
    
    content.push_str(&format!("## {} {}\n\n", timestamp, project_info));
    
    // AI 提问（截取前 500 字符，避免日志过长）
    content.push_str("### 🤖 AI\n");
    let ai_msg = truncate_message(&entry.ai_message, 500);
    content.push_str(&ai_msg);
    content.push_str("\n\n");
    
    // 用户回答
    content.push_str("### 👤 用户\n");
    
    // 选择的选项
    if !entry.selected_options.is_empty() {
        content.push_str(&format!("**选择**: {}\n\n", entry.selected_options.join(", ")));
    }
    
    // 用户输入文本
    if !entry.user_response.is_empty() {
        content.push_str(&entry.user_response);
        content.push('\n');
    }
    
    // 图片标记
    if entry.image_count > 0 {
        content.push_str(&format!("\n📷 *附图 {} 张*\n", entry.image_count));
    }
    
    content.push_str("\n---\n\n");
    
    content
}

/// 截取消息，避免日志过长
fn truncate_message(msg: &str, max_len: usize) -> String {
    if msg.len() <= max_len {
        msg.to_string()
    } else {
        format!("{}...\n\n*(已截断)*", &msg[..max_len])
    }
}
