use anyhow::Result;
use rmcp::{Error as McpError, model::*};

use super::{MemoryManager, MemoryCategory};
// 从 mcp 模块导入：
// - JiyiRequest: 记忆操作的请求结构体
// - PopupRequest: 弹窗请求结构体
// - validate_project_path: 验证项目路径的工具函数
// - project_path_error: 生成路径错误的工具函数
// - generate_request_id: 生成唯一请求ID的工具函数
use crate::mcp::{JiyiRequest, PopupRequest, utils::{validate_project_path, project_path_error, generate_request_id}};
use crate::mcp::handlers::create_tauri_popup;

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
                
                // 弹窗确认
                let confirm_msg = format!(
                    "## 确认沉淀到 .cunzhi-knowledge/{}\n\n```\n{}\n```",
                    if category == "patterns" { "patterns.md" } else { "problems.md" },
                    &request.content
                );
                
                let popup_request = PopupRequest {
                    id: generate_request_id(),
                    message: confirm_msg,
                    predefined_options: Some(vec!["确认沉淀".to_string(), "取消".to_string()]),
                    is_markdown: true,
                    project_path: Some(request.project_path.clone()),
                };
                
                let response = create_tauri_popup(&popup_request)
                    .map_err(|e| McpError::internal_error(format!("弹窗失败: {}", e), None))?;
                
                // 检查用户是否确认
                if response.contains("取消") || response.contains("CANCELLED") {
                    return Ok(CallToolResult::success(vec![Content::text("❌ 用户取消沉淀".to_string())]));
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
