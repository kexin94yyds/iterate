use rmcp::{Error as McpError, model::*};

use crate::mcp::types::PaiRequest;
use crate::log_debug;

/// 子代理派发工具
///
/// 根据任务参数生成子代理提示词，供用户复制到新聊天窗口
#[derive(Clone)]
pub struct DispatchTool;

impl DispatchTool {
    /// 生成子代理提示词
    fn generate_subagent_prompt(request: &PaiRequest) -> String {
        let items_list = request.items
            .iter()
            .enumerate()
            .map(|(i, item)| format!("{}. {}", i + 1, item))
            .collect::<Vec<_>>()
            .join("\n");

        let mut prompt = format!(
            r#"## 子代理任务

**任务类型**: {}
**范围**（共 {} 个）：
{}
"#,
            request.task_type,
            request.items.len(),
            items_list
        );

        // 添加源文件和目标文件
        if let Some(ref source) = request.source_file {
            prompt.push_str(&format!("**源文件**: {}\n", source));
        }
        if let Some(ref target) = request.target_file {
            prompt.push_str(&format!("**目标文件**: {}\n", target));
        }

        // 添加步骤
        prompt.push_str("\n### 步骤\n");
        prompt.push_str("1. 读取源文件中以上列表对应的条目\n");
        prompt.push_str("2. 按格式要求生成目标内容");

        // 添加输出格式模板
        if let Some(ref format) = request.output_format {
            prompt.push_str(&format!("，格式：\n\n{}\n", format));
        } else {
            prompt.push_str("\n");
        }

        prompt.push_str("3. 追加到目标文件末尾\n");
        prompt.push_str("4. 完成后报告：已处理 X 条\n");

        // 添加额外步骤
        if let Some(ref extra) = request.extra_steps {
            prompt.push_str(&format!("\n### 额外说明\n{}\n", extra));
        }

        // 添加验收标准
        prompt.push_str(r#"
### 验收标准
- 条目数量正确
- 格式符合规范
- 无重复条目

*你是子代理现在帮我做*：
"#);

        prompt
    }

    pub async fn pai(request: PaiRequest) -> Result<CallToolResult, McpError> {
        log_debug!("生成子代理提示词，任务类型: {}, 条目数: {}", 
            request.task_type, request.items.len());

        if request.items.is_empty() {
            return Err(McpError::invalid_params(
                "任务范围列表不能为空",
                None
            ));
        }

        let prompt = Self::generate_subagent_prompt(&request);

        let result = format!(
            r#"📋 **子代理提示词已生成**

请复制以下内容到新聊天窗口（Cmd+T）：

---

{}

---

**提示词长度**: {} 字符
**任务条目数**: {} 个"#,
            prompt,
            prompt.len(),
            request.items.len()
        );

        Ok(CallToolResult::success(vec![Content::text(result)]))
    }
}
