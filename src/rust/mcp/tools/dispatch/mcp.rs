use rmcp::{Error as McpError, model::*};

use crate::mcp::types::PaiRequest;
use crate::mcp::{PopupRequest, handlers::create_tauri_popup, utils::generate_request_id};
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

        // 添加验收标准和汇报要求
        prompt.push_str(r#"
### 验收标准
- 条目数量正确
- 格式符合规范
- 无重复条目

### 完成后汇报（必须）
任务完成后，**必须调用 `zhi` 工具**向用户汇报结果，包含：
- 处理条目数量
- 完成的操作列表
- Git commit 信息（如有）

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

        // 通过寸止窗口显示提示词，方便用户复制
        let popup_message = format!(
            r#"## 📋 子代理提示词

**任务类型**: {}
**条目数量**: {} 个

---

复制以下内容到新窗口（Cmd+T）：

```
{}
```

---
💡 复制后在新窗口末尾输入批次号开始执行"#,
            request.task_type,
            request.items.len(),
            prompt
        );

        let popup_request = PopupRequest {
            id: generate_request_id(),
            message: popup_message,
            predefined_options: Some(vec![
                "已复制，开始执行".to_string(),
                "取消".to_string(),
            ]),
            is_markdown: true,
            project_path: None,
            link_url: None,
            link_title: None,
        };

        match create_tauri_popup(&popup_request) {
            Ok(response) => {
                let result = format!(
                    "子代理提示词已显示在寸止窗口\n\n用户响应: {}\n\n提示词长度: {} 字符",
                    response,
                    prompt.len()
                );
                Ok(CallToolResult::success(vec![Content::text(result)]))
            }
            Err(e) => {
                // 降级：直接返回提示词
                log_debug!("寸止窗口显示失败，降级返回文本: {}", e);
                let result = format!(
                    r#"📋 **子代理提示词**（寸止窗口不可用，直接显示）

```markdown
{}
```

**提示词长度**: {} 字符"#,
                    prompt,
                    prompt.len()
                );
                Ok(CallToolResult::success(vec![Content::text(result)]))
            }
        }
    }
}
