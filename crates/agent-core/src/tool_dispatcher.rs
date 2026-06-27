//! Tool call dispatcher — parse adk_core Part::FunctionCall into typed handlers.

use crate::error::{AgentError, Result};

/// Typed tool calls from the LLM (parsed from adk_core::Part::FunctionCall).
#[derive(Debug, Clone)]
pub enum ToolCall {
    ReadFile { path: String, offset: Option<u64>, limit: Option<u64> },
    WriteFile { path: String, content: String },
    EditFile { path: String, old_text: String, new_text: String },
    Glob { pattern: String },
    Grep { pattern: String, path: Option<String> },
    Bash { command: String },
    ReadMcpResource { server: String, uri: String },
    Unknown { name: String, args: serde_json::Value },
}

impl ToolCall {
    /// Parse from adk_core::Part::FunctionCall.
    pub fn from_part(part: &adk_core::Part) -> Option<Self> {
        let adk_core::Part::FunctionCall { name, args, .. } = part else {
            return None;
        };
        match name.as_str() {
            "read_file" | "ReadFile" => Some(Self::ReadFile {
                path: args.get("path").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                offset: args.get("offset").and_then(|v| v.as_u64()),
                limit: args.get("limit").and_then(|v| v.as_u64()),
            }),
            "write_file" | "WriteFile" => Some(Self::WriteFile {
                path: args.get("path").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                content: args.get("content").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            }),
            "edit_file" | "EditFile" => Some(Self::EditFile {
                path: args.get("path").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                old_text: args.get("old_text").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                new_text: args.get("new_text").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            }),
            "glob" | "Glob" => Some(Self::Glob {
                pattern: args.get("pattern").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            }),
            "grep" | "Grep" => Some(Self::Grep {
                pattern: args.get("pattern").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                path: args.get("path").and_then(|v| v.as_str()).map(String::from),
            }),
            "bash" | "Bash" => Some(Self::Bash {
                command: args.get("command").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            }),
            "read_mcp_resource" => Some(Self::ReadMcpResource {
                server: args.get("server").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                uri: args.get("uri").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            }),
            _ => Some(Self::Unknown { name: name.clone(), args: args.clone() }),
        }
    }
}

/// Dispatch a ToolCall to its handler and return JSON string.
pub async fn dispatch(call: ToolCall) -> Result<String> {
    match call {
        ToolCall::ReadFile { path, offset, limit } => {
            let content = tokio::fs::read_to_string(&path).await
                .map_err(|e| AgentError::ApiError(format!("read_file: {e}")))?;
            let slice = match (offset, limit) {
                (Some(off), Some(l)) => {
                    let off = off as usize;
                    let l = l as usize;
                    content.get(off..(off + l).min(content.len())).unwrap_or("").to_string()
                },
                _ => content,
            };
            Ok(serde_json::json!({ "content": slice }).to_string())
        }
        ToolCall::WriteFile { path, content } => {
            tokio::fs::write(&path, &content).await
                .map_err(|e| AgentError::ApiError(format!("write_file: {e}")))?;
            Ok(r#"{"ok":true}"#.to_string())
        }
        ToolCall::EditFile { path, old_text, new_text } => {
            let content = tokio::fs::read_to_string(&path).await
                .map_err(|e| AgentError::ApiError(format!("edit read: {e}")))?;
            let new_content = content.replace(&old_text, &new_text);
            tokio::fs::write(&path, &new_content).await
                .map_err(|e| AgentError::ApiError(format!("edit write: {e}")))?;
            Ok(r#"{"ok":true}"#.to_string())
        }
        ToolCall::Glob { .. } => Ok(r#"{"files":[]}"#.to_string()),
        ToolCall::Grep { .. } => Ok(r#"{"matches":[]}"#.to_string()),
        ToolCall::Bash { command } => {
            let output = tokio::process::Command::new("sh")
                .arg("-c").arg(&command)
                .output().await
                .map_err(|e| AgentError::ApiError(format!("bash: {e}")))?;
            Ok(serde_json::json!({
                "stdout": String::from_utf8_lossy(&output.stdout),
                "stderr": String::from_utf8_lossy(&output.stderr),
                "code": output.status.code()
            }).to_string())
        }
        ToolCall::ReadMcpResource { .. } => Ok(r#"{"error":"MCP not yet implemented"}"#.to_string()),
        ToolCall::Unknown { name, args } => Ok(serde_json::json!({
            "error": format!("unknown tool: {}", name),
            "params": args
        }).to_string()),
    }
}
