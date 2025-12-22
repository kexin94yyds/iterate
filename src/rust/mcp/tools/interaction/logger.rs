//! 对话日志记录模块
//!
//! 自动记录 zhi 工具的 AI 提问和用户回答
//! 支持 5 分钟防抖自动同步到 GitHub

use chrono::Local;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Mutex;
use std::process::Command;

/// 全局状态：是否有待同步的对话
static PENDING_SYNC: AtomicBool = AtomicBool::new(false);

/// 上次写入时间戳（Unix 秒）
static LAST_WRITE_TIMESTAMP: AtomicU64 = AtomicU64::new(0);

/// 知识库路径缓存
static KNOWLEDGE_DIR_CACHE: Mutex<Option<PathBuf>> = Mutex::new(None);

/// 防抖间隔：5 分钟
const SYNC_DEBOUNCE_SECS: u64 = 300;

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
    
    // 标记有待同步，并启动防抖同步
    mark_pending_sync(&knowledge_dir);
    
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

/// 截取消息，避免日志过长（安全处理 UTF-8 边界）
fn truncate_message(msg: &str, max_chars: usize) -> String {
    let char_count = msg.chars().count();
    if char_count <= max_chars {
        msg.to_string()
    } else {
        let truncated: String = msg.chars().take(max_chars).collect();
        format!("{}...\n\n*(已截断)*", truncated)
    }
}

/// 标记有待同步，并启动防抖同步任务
fn mark_pending_sync(knowledge_dir: &PathBuf) {
    PENDING_SYNC.store(true, Ordering::SeqCst);
    
    // 更新最后写入时间戳
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    LAST_WRITE_TIMESTAMP.store(now, Ordering::SeqCst);
    
    // 缓存知识库路径
    if let Ok(mut cache) = KNOWLEDGE_DIR_CACHE.lock() {
        *cache = Some(knowledge_dir.clone());
    }
    
    // 启动后台同步任务（防抖）
    std::thread::spawn(move || {
        // 等待防抖间隔
        std::thread::sleep(std::time::Duration::from_secs(SYNC_DEBOUNCE_SECS));
        
        // 检查是否在等待期间有新写入（防抖）
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let last_write = LAST_WRITE_TIMESTAMP.load(Ordering::SeqCst);
        let should_sync = current_time >= last_write + SYNC_DEBOUNCE_SECS;
        
        if should_sync && PENDING_SYNC.load(Ordering::SeqCst) {
            // 从缓存获取知识库路径
            if let Ok(cache) = KNOWLEDGE_DIR_CACHE.lock() {
                if let Some(ref dir) = *cache {
                    sync_conversations(dir);
                }
            }
        }
    });
}

/// 同步对话记录到 GitHub
fn sync_conversations(knowledge_dir: &PathBuf) {
    // 重置待同步标记
    PENDING_SYNC.store(false, Ordering::SeqCst);
    
    let conversations_dir = knowledge_dir.join("conversations");
    if !conversations_dir.exists() {
        return;
    }
    
    // git add conversations/
    let add_result = Command::new("git")
        .args(["add", "conversations/"])
        .current_dir(knowledge_dir)
        .output();
    
    if let Err(e) = add_result {
        eprintln!("[cunzhi] git add 失败: {}", e);
        return;
    }
    
    // git commit
    let today = Local::now().format("%Y-%m-%d").to_string();
    let commit_msg = format!("sync: 对话记录 {}", today);
    let commit_result = Command::new("git")
        .args(["commit", "-m", &commit_msg])
        .current_dir(knowledge_dir)
        .output();
    
    if let Ok(output) = commit_result {
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            // 如果是 "nothing to commit" 则忽略
            if !stderr.contains("nothing to commit") {
                eprintln!("[cunzhi] git commit 失败: {}", stderr);
            }
            return;
        }
    }
    
    // git push
    let push_result = Command::new("git")
        .args(["push"])
        .current_dir(knowledge_dir)
        .output();
    
    if let Err(e) = push_result {
        eprintln!("[cunzhi] git push 失败: {}", e);
    } else {
        eprintln!("[cunzhi] 对话记录已同步到 GitHub");
    }
}

/// 强制立即同步（用于应用退出时）
pub fn force_sync_conversations() {
    if !PENDING_SYNC.load(Ordering::SeqCst) {
        return;
    }
    
    // 尝试查找知识库目录
    if let Ok(knowledge_dir) = find_knowledge_dir(None) {
        sync_conversations(&knowledge_dir);
    }
}
