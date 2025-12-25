use anyhow::Result;
use rmcp::{
    Error as McpError, ServerHandler, ServiceExt, RoleServer,
    model::*,
    transport::stdio,
    service::RequestContext,
};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Instant, Duration};
use parking_lot::Mutex;

use super::tools::{InteractionTool, MemoryTool, AcemcpTool, DispatchTool, XiTool, CiTool};
use super::types::{ZhiRequest, JiyiRequest, PaiRequest, XiRequest, CiRequest};
use crate::config::load_standalone_config;
use crate::{log_important, log_debug};

/// 需要 zhi 前置确认的工具（写入/危险操作）
const TOOLS_REQUIRING_ZHI: &[&str] = &["ji", "pai"];

/// zhi 授权有效期（秒）
const ZHI_AUTH_TIMEOUT_SECS: u64 = 300; // 5 分钟

/// 全局状态：记录最后一次 zhi 调用时间
static ZHI_LAST_CALL: std::sync::OnceLock<Arc<Mutex<Option<Instant>>>> = std::sync::OnceLock::new();

fn get_zhi_last_call() -> &'static Arc<Mutex<Option<Instant>>> {
    ZHI_LAST_CALL.get_or_init(|| Arc::new(Mutex::new(None)))
}

#[derive(Clone)]
pub struct ZhiServer {
    enabled_tools: HashMap<String, bool>,
}

impl Default for ZhiServer {
    fn default() -> Self {
        Self::new()
    }
}

impl ZhiServer {
    pub fn new() -> Self {
        // 尝试加载配置，如果失败则使用默认配置
        let enabled_tools = match load_standalone_config() {
            Ok(config) => config.mcp_config.tools,
            Err(e) => {
                log_important!(warn, "无法加载配置文件，使用默认工具配置: {}", e);
                crate::config::default_mcp_tools()
            }
        };

        Self { enabled_tools }
    }

    /// 检查工具是否启用 - 动态读取最新配置
    fn is_tool_enabled(&self, tool_name: &str) -> bool {
        // 每次都重新读取配置，确保获取最新状态
        match load_standalone_config() {
            Ok(config) => {
                let enabled = config.mcp_config.tools.get(tool_name).copied().unwrap_or(true);
                log_debug!("工具 {} 当前状态: {}", tool_name, enabled);
                enabled
            }
            Err(e) => {
                log_important!(warn, "读取配置失败，使用缓存状态: {}", e);
                // 如果读取失败，使用缓存的配置
                self.enabled_tools.get(tool_name).copied().unwrap_or(true)
            }
        }
    }
}

impl ServerHandler for ZhiServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            server_info: Implementation {
                name: "Zhi-mcp".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
            },
            instructions: Some("Zhi 智能代码审查工具，支持交互式对话和记忆管理".to_string()),
        }
    }

    async fn initialize(
        &self,
        _request: InitializeRequestParam,
        _context: RequestContext<RoleServer>,
    ) -> Result<ServerInfo, McpError> {
        Ok(self.get_info())
    }

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParam>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, McpError> {
        use std::sync::Arc;
        use std::borrow::Cow;

        let mut tools = Vec::new();

        // iterate 工具始终可用（必需工具）
        let zhi_schema = serde_json::json!({
            "type": "object",
            "properties": {
                "message": {
                    "type": "string",
                    "description": "要显示给用户的消息"
                },
                "predefined_options": {
                    "type": "array",
                    "items": {"type": "string"},
                    "description": "预定义的选项列表（可选）"
                },
                "is_markdown": {
                    "type": "boolean",
                    "description": "消息是否为Markdown格式，默认为true"
                },
                "project_path": {
                    "type": "string",
                    "description": "当前项目的绝对路径（强烈建议传递，用于在弹窗中显示项目路径）"
                }
            },
            "required": ["message"]
        });

        if let serde_json::Value::Object(schema_map) = zhi_schema {
            tools.push(Tool {
                name: Cow::Borrowed("zhi"),
                description: Some(Cow::Borrowed("智能代码审查交互工具（L0 协调者）。所有对话必经，控制任务流程。支持预定义选项、自由文本输入和图片上传。")),
                input_schema: Arc::new(schema_map),
                annotations: None,
            });
        }

        // 记忆管理工具 - 仅在启用时添加
        if self.is_tool_enabled("ji") {
            let ji_schema = serde_json::json!({
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "description": "操作类型：记忆(添加记忆), 回忆(获取项目信息), 沉淀(写入knowledge), 摘要(添加会话摘要)"
                    },
                    "project_path": {
                        "type": "string",
                        "description": "项目路径（必需）"
                    },
                    "content": {
                        "type": "string",
                        "description": "记忆内容（记忆操作时必需）"
                    },
                    "category": {
                        "type": "string",
                        "description": "记忆分类：rule(规范规则), preference(用户偏好), pattern(最佳实践), context(项目上下文)"
                    }
                },
                "required": ["action", "project_path"]
            });

            if let serde_json::Value::Object(schema_map) = ji_schema {
                tools.push(Tool {
                    name: Cow::Borrowed("ji"),
                    description: Some(Cow::Borrowed("全局记忆管理工具。支持 4 种 action：回忆/记忆/沉淀/摘要。必须绑定 git 根目录。用于存储开发规范、用户偏好和最佳实践。")),
                    input_schema: Arc::new(schema_map),
                    annotations: None,
                });
            }
        }

        // 代码搜索工具 - 仅在启用时添加
        if self.is_tool_enabled("sou") {
            tools.push(AcemcpTool::get_tool_definition());
        }

        // 经验查找工具 - 仅在启用时添加
        if self.is_tool_enabled("xi") {
            let xi_schema = serde_json::json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "用于查找相关历史经验的自然语言查询"
                    },
                    "project_path": {
                        "type": "string",
                        "description": "项目路径（必需）"
                    }
                },
                "required": ["query", "project_path"]
            });

            if let serde_json::Value::Object(schema_map) = xi_schema {
                tools.push(Tool {
                    name: Cow::Borrowed("xi"),
                    description: Some(Cow::Borrowed("经验查找工具。在 .cunzhi-knowledge/ 中查找相关历史经验（patterns.md、problems.md、regressions.md）。")),
                    input_schema: Arc::new(schema_map),
                    annotations: None,
                });
            }
        }

        // 提示词库搜索工具 - 仅在启用时添加
        if self.is_tool_enabled("ci") {
            let ku_schema = serde_json::json!({
                "type": "object",
                "properties": {
                    "directory": {
                        "type": "string",
                        "description": "提示词库目录名（如 ci、git、testing）"
                    },
                    "project_path": {
                        "type": "string",
                        "description": "项目路径（必需）"
                    },
                    "query": {
                        "type": "string",
                        "description": "搜索关键词（可选，用于过滤模板）"
                    }
                },
                "required": ["directory", "project_path"]
            });

            if let serde_json::Value::Object(schema_map) = ku_schema {
                tools.push(Tool {
                    name: Cow::Borrowed("ci"),
                    description: Some(Cow::Borrowed("提示词库搜索工具。在 .cunzhi-knowledge/prompts/ 中搜索相关模板。触发：用户输入目录名（如 ci、git、testing）。")),
                    input_schema: Arc::new(schema_map),
                    annotations: None,
                });
            }
        }

        // 子代理派发工具 - 仅在启用时添加
        if self.is_tool_enabled("pai") {
            let pai_schema = serde_json::json!({
                "type": "object",
                "properties": {
                    "task_type": {
                        "type": "string",
                        "description": "任务类型（如：补录回归检查、批量重命名、代码审查）"
                    },
                    "items": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "任务范围列表（显式列表，不用模糊表述）"
                    },
                    "source_file": {
                        "type": "string",
                        "description": "源文件路径（可选）"
                    },
                    "target_file": {
                        "type": "string",
                        "description": "目标文件路径（可选）"
                    },
                    "output_format": {
                        "type": "string",
                        "description": "输出格式模板（可选，用于指定子代理输出格式）"
                    },
                    "extra_steps": {
                        "type": "string",
                        "description": "额外步骤说明（可选）"
                    }
                },
                "required": ["task_type", "items"]
            });

            if let serde_json::Value::Object(schema_map) = pai_schema {
                tools.push(Tool {
                    name: Cow::Borrowed("pai"),
                    description: Some(Cow::Borrowed("子代理派发工具。生成子代理提示词供用户复制到新聊天窗口执行。遵循 batch-task.md 工作流，禁止模糊范围。")),
                    input_schema: Arc::new(schema_map),
                    annotations: None,
                });
            }
        }

        log_debug!("返回给客户端的工具列表: {:?}", tools.iter().map(|t| &t.name).collect::<Vec<_>>());

        Ok(ListToolsResult {
            tools,
            next_cursor: None,
        })
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParam,
        _context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        log_debug!("收到工具调用请求: {}", request.name);

        // 守卫检查：需要 zhi 前置确认的工具
        let tool_name = request.name.as_ref();
        if TOOLS_REQUIRING_ZHI.contains(&tool_name) {
            let last_call = get_zhi_last_call().lock();
            let needs_zhi = match *last_call {
                None => true, // 从未调用过 zhi
                Some(instant) => instant.elapsed() > Duration::from_secs(ZHI_AUTH_TIMEOUT_SECS),
            };
            drop(last_call); // 释放锁
            
            if needs_zhi {
                log_important!(warn, "工具 {} 需要先调用 zhi 确认", tool_name);
                return Err(McpError::invalid_request(
                    format!("⚠️ 操作需要确认：请先调用 zhi 工具向用户确认后再执行 {} 操作", tool_name),
                    None
                ));
            }
        }

        match request.name.as_ref() {
            "zhi" => {
                // 解析请求参数
                let arguments_value = request.arguments
                    .map(serde_json::Value::Object)
                    .unwrap_or(serde_json::Value::Object(serde_json::Map::new()));

                let zhi_request: ZhiRequest = serde_json::from_value(arguments_value)
                    .map_err(|e| McpError::invalid_params(format!("参数解析失败: {}", e), None))?;

                // 调用 zhi 工具
                let result = InteractionTool::zhi(zhi_request).await;
                
                // 成功调用后更新时间戳
                if result.is_ok() {
                    let mut last_call = get_zhi_last_call().lock();
                    *last_call = Some(Instant::now());
                    log_debug!("zhi 授权时间戳已更新");
                }
                
                result
            }
            "ji" => {
                // 检查记忆管理工具是否启用
                if !self.is_tool_enabled("ji") {
                    return Err(McpError::internal_error(
                        "记忆管理工具已被禁用".to_string(),
                        None
                    ));
                }

                // 解析请求参数
                let arguments_value = request.arguments
                    .map(serde_json::Value::Object)
                    .unwrap_or(serde_json::Value::Object(serde_json::Map::new()));

                let ji_request: JiyiRequest = serde_json::from_value(arguments_value)
                    .map_err(|e| McpError::invalid_params(format!("参数解析失败: {}", e), None))?;

                // 调用记忆工具
                MemoryTool::jiyi(ji_request).await
            }
            "sou" => {
                // 检查代码搜索工具是否启用
                if !self.is_tool_enabled("sou") {
                    return Err(McpError::internal_error(
                        "代码搜索工具已被禁用".to_string(),
                        None
                    ));
                }

                // 解析请求参数
                let arguments_value = request.arguments
                    .map(serde_json::Value::Object)
                    .unwrap_or(serde_json::Value::Object(serde_json::Map::new()));

                // 使用acemcp模块中的AcemcpRequest类型
                let acemcp_request: crate::mcp::tools::acemcp::types::AcemcpRequest = serde_json::from_value(arguments_value)
                    .map_err(|e| McpError::invalid_params(format!("参数解析失败: {}", e), None))?;

                // 调用代码搜索工具
                AcemcpTool::search_context(acemcp_request).await
            }
            "pai" => {
                // 检查子代理派发工具是否启用
                if !self.is_tool_enabled("pai") {
                    return Err(McpError::internal_error(
                        "子代理派发工具已被禁用".to_string(),
                        None
                    ));
                }

                // 解析请求参数
                let arguments_value = request.arguments
                    .map(serde_json::Value::Object)
                    .unwrap_or(serde_json::Value::Object(serde_json::Map::new()));

                let pai_request: PaiRequest = serde_json::from_value(arguments_value)
                    .map_err(|e| McpError::invalid_params(format!("参数解析失败: {}", e), None))?;

                // 调用子代理派发工具
                DispatchTool::pai(pai_request).await
            }
            "xi" => {
                // 检查经验查找工具是否启用
                if !self.is_tool_enabled("xi") {
                    return Err(McpError::internal_error(
                        "经验查找工具已被禁用".to_string(),
                        None
                    ));
                }

                // 解析请求参数
                let arguments_value = request.arguments
                    .map(serde_json::Value::Object)
                    .unwrap_or(serde_json::Value::Object(serde_json::Map::new()));

                let xi_request: XiRequest = serde_json::from_value(arguments_value)
                    .map_err(|e| McpError::invalid_params(format!("参数解析失败: {}", e), None))?;

                // 调用经验查找工具
                XiTool::search_experience(xi_request).await
            }
            "ci" => {
                // 检查提示词库搜索工具是否启用
                if !self.is_tool_enabled("ci") {
                    return Err(McpError::internal_error(
                        "ci 工具已被禁用".to_string(),
                        None
                    ));
                }

                // 解析请求参数
                let arguments_value = request.arguments
                    .map(serde_json::Value::Object)
                    .unwrap_or(serde_json::Value::Object(serde_json::Map::new()));

                let ci_request: CiRequest = serde_json::from_value(arguments_value)
                    .map_err(|e| McpError::invalid_params(format!("参数解析失败: {}", e), None))?;

                // 调用提示词库搜索工具
                CiTool::search_prompts(ci_request).await
            }
            _ => {
                Err(McpError::invalid_request(
                    format!("未知的工具: {}", request.name),
                    None
                ))
            }
        }
    }
}



/// 启动MCP服务器
pub async fn run_server() -> Result<(), Box<dyn std::error::Error>> {
    // 记录启动信息
    let pid = std::process::id();
    log_important!(info, "[MCP] 服务器启动 PID={}", pid);
    
    // 创建并运行服务器
    let service = ZhiServer::new()
        .serve(stdio())
        .await
        .inspect_err(|e| {
            log_important!(error, "[MCP] 启动服务器失败 PID={}: {}", pid, e);
        })?;

    log_important!(info, "[MCP] 服务器开始监听 PID={}", pid);
    
    // 等待服务器关闭
    let result = service.waiting().await;
    log_important!(info, "[MCP] 服务器退出 PID={} result={:?}", pid, result.is_ok());
    result.map(|_| ()).map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
}
