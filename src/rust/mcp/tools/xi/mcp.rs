use anyhow::Result;
use rmcp::{Error as McpError, model::*};
use std::fs;
use std::path::Path;

use crate::mcp::types::XiRequest;

/// 经验查找工具
///
/// 在 .cunzhi-knowledge/ 中查找相关历史经验
#[derive(Clone)]
pub struct XiTool;

impl XiTool {
    /// 在知识库中搜索相关经验
    pub async fn search_experience(
        request: XiRequest,
    ) -> Result<CallToolResult, McpError> {
        let project_path = Path::new(&request.project_path);
        
        // 验证项目路径
        if !project_path.exists() {
            return Err(McpError::invalid_params(
                format!("项目路径不存在: {}", request.project_path),
                None
            ));
        }

        // 查找 .cunzhi-knowledge 目录
        let knowledge_dir = project_path.join(".cunzhi-knowledge");
        if !knowledge_dir.exists() {
            return Ok(CallToolResult::success(vec![Content::text(
                "📭 项目未接入全局知识库，无法查找历史经验"
            )]));
        }

        let query = request.query.to_lowercase();
        let mut results = Vec::new();

        // 搜索 patterns.md
        let patterns_path = knowledge_dir.join("patterns.md");
        if patterns_path.exists() {
            if let Ok(content) = fs::read_to_string(&patterns_path) {
                let matches = Self::search_in_content(&content, &query, "patterns.md");
                if !matches.is_empty() {
                    results.push(format!("## 📘 最佳实践 (patterns.md)\n\n{}", matches.join("\n\n")));
                }
            }
        }

        // 搜索 problems.md
        let problems_path = knowledge_dir.join("problems.md");
        if problems_path.exists() {
            if let Ok(content) = fs::read_to_string(&problems_path) {
                let matches = Self::search_in_content(&content, &query, "problems.md");
                if !matches.is_empty() {
                    results.push(format!("## 🐛 问题记录 (problems.md)\n\n{}", matches.join("\n\n")));
                }
            }
        }

        // 搜索 regressions.md
        let regressions_path = knowledge_dir.join("regressions.md");
        if regressions_path.exists() {
            if let Ok(content) = fs::read_to_string(&regressions_path) {
                let matches = Self::search_in_content(&content, &query, "regressions.md");
                if !matches.is_empty() {
                    results.push(format!("## 🔄 回归经验 (regressions.md)\n\n{}", matches.join("\n\n")));
                }
            }
        }

        if results.is_empty() {
            Ok(CallToolResult::success(vec![Content::text(
                format!("📭 未找到与「{}」相关的历史经验", request.query)
            )]))
        } else {
            Ok(CallToolResult::success(vec![Content::text(
                format!("# 🔍 历史经验查找结果\n\n查询：「{}」\n\n{}", request.query, results.join("\n\n---\n\n"))
            )]))
        }
    }

    /// 在内容中搜索匹配的段落
    fn search_in_content(content: &str, query: &str, _filename: &str) -> Vec<String> {
        let mut matches = Vec::new();
        
        // 按 ## 分割为段落
        let sections: Vec<&str> = content.split("\n## ").collect();
        
        for (i, section) in sections.iter().enumerate() {
            let section_lower = section.to_lowercase();
            
            // 检查段落是否包含查询关键词
            if section_lower.contains(query) {
                // 提取段落标题和前几行内容
                let lines: Vec<&str> = section.lines().collect();
                if !lines.is_empty() {
                    let title = if i == 0 {
                        // 第一个段落可能没有 ##
                        lines[0].trim_start_matches("# ").to_string()
                    } else {
                        format!("## {}", lines[0])
                    };
                    
                    // 取前 10 行作为摘要
                    let summary: Vec<&str> = lines.iter().take(10).copied().collect();
                    let truncated = if lines.len() > 10 { "\n..." } else { "" };
                    
                    matches.push(format!("{}\n{}{}", title, summary.join("\n"), truncated));
                }
            }
        }
        
        // 限制返回数量
        matches.truncate(5);
        matches
    }
}
