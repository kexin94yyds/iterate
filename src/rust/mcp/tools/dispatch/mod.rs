//! 子代理派发工具模块
//!
//! 生成子代理提示词，供用户复制到新聊天窗口执行批量任务

pub mod mcp;

// 重新导出主要类型和功能
pub use mcp::DispatchTool;
