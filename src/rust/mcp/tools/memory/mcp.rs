use anyhow::Result;
use rmcp::{Error as McpError, model::*};

use super::{MemoryManager, MemoryCategory};
use crate::mcp::{JiyiRequest, utils::{validate_project_path, project_path_error}};

/// 全局记忆管理工具
///
/// 用于存储和管理重要的开发规范、用户偏好和最佳实践
#[derive(Clone)]
pub struct MemoryTool;

impl MemoryTool {
    pub async fn jiyi(
        request: JiyiRequest,
    ) -> Result<CallToolResult, McpError> {
        // 使用增强的路径验证功能
        if let Err(e) = validate_project_path(&request.project_path) {
            return Err(project_path_error(format!(
                "路径验证失败: {}\n原始路径: {}\n请检查路径格式是否正确，特别是 Windows 路径应使用正确的盘符格式（如 C:\\path）",
                e,
                request.project_path
            )).into());
        }

        let manager = MemoryManager::new(&request.project_path)
            .map_err(|e| McpError::internal_error(format!("创建记忆管理器失败: {}", e), None))?;

        let result = match request.action.as_str() {
            "记忆" => {
                if request.content.trim().is_empty() {
                    return Err(McpError::invalid_params("缺少记忆内容".to_string(), None));
                }

                let category = match request.category.as_str() {
                    "rule" => MemoryCategory::Rule,
                    "preference" => MemoryCategory::Preference,
                    "note" => MemoryCategory::Note,
                    "context" => MemoryCategory::Context,
                    _ => MemoryCategory::Context,
                };

                let id = manager.add_memory(&request.content, category)
                    .map_err(|e| McpError::internal_error(format!("添加记忆失败: {}", e), None))?;

                format!("✅ 记忆已添加，ID: {}\n📝 内容: {}\n📂 分类: {:?}", id, request.content, category)
            }
            "回忆" => {
                let memory_info = manager.get_project_info()
                    .map_err(|e| McpError::internal_error(format!("获取项目记忆失败: {}", e), None))?;
                let knowledge_info = manager.read_knowledge()
                    .map_err(|e| McpError::internal_error(format!("获取知识库失败: {}", e), None))?;
                
                format!("{}\n{}", memory_info, knowledge_info)
            }
            "沉淀" => {
                if request.content.trim().is_empty() {
                    return Err(McpError::invalid_params("缺少沉淀内容".to_string(), None));
                }
                
                // 验证 category 是否为 knowledge 专用类型
                let category = match request.category.as_str() {
                    "patterns" | "problems" => request.category.as_str(),
                    _ => return Err(McpError::invalid_params(
                        format!("沉淀仅支持 patterns/problems 分类，收到: {}", request.category),
                        None
                    )),
                };
                
                // 验证 problems 格式必须包含 P-YYYY-NNN
                if category == "problems" {
                    let pattern = regex::Regex::new(r"P-\d{4}-\d{3}").unwrap();
                    if !pattern.is_match(&request.content) {
                        return Err(McpError::invalid_params(
                            "沉淀 problems 必须包含 P-YYYY-NNN 格式的编号（如 P-2024-001）".to_string(),
                            None
                        ));
                    }
                }
                
                manager.settle_to_knowledge(&request.content, category)
                    .map_err(|e| McpError::internal_error(format!("沉淀失败: {}", e), None))?
            }
            "摘要" => {
                if request.content.trim().is_empty() {
                    return Err(McpError::invalid_params("缺少摘要内容".to_string(), None));
                }
                
                manager.add_session_summary(&request.content)
                    .map_err(|e| McpError::internal_error(format!("添加摘要失败: {}", e), None))?
            }
            _ => {
                return Err(McpError::invalid_params(
                    format!("未知的操作类型: {}", request.action),
                    None
                ));
            }
        };

        Ok(CallToolResult::success(vec![Content::text(result)]))
    }
}
