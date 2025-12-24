//! 记忆管理工具模块
//!
//! 提供全局记忆管理功能，用于存储和管理重要的开发规范、用户偏好和最佳实践

pub mod manager;
pub mod types;
pub mod mcp;
pub mod commands;

// 重新导出主要类型和功能
pub use manager::MemoryManager;
pub use types::{MemoryEntry, MemoryCategory, MemoryMetadata};
pub use mcp::MemoryTool;
