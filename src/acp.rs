//! ACP JSON-RPC 辅助 — stdout 传输与通知构造。
//!
//! 与 [Agent Client Protocol](https://agentclientprotocol.com) 对齐：流式与工具进度使用
//! **`session/update`**，参数中必须包含 **`sessionId`**；文本块使用 **`ContentBlock`**（`type: "text"`）；
//! 工具调用通知携带 **`toolCallId`**（与 `ToolCall` / `ToolCallUpdate` schema 一致）。

use serde_json::{json, Value};
use std::io::Write;

/// 将单个 JSON 对象写入 stdout（换行分隔的一条消息）。
pub fn send(obj: &Value) {
    let mut stdout = std::io::stdout().lock();
    let _ = serde_json::to_writer(&mut stdout, obj);
    let _ = stdout.write_all(b"\n");
    let _ = stdout.flush();
}

/// 发送 JSON-RPC 成功响应（`result`）。
pub fn send_response(id: &Value, result: Value) {
    send(&json!({"jsonrpc": "2.0", "id": id, "result": result}));
}

/// 发送 JSON-RPC 错误响应。
pub fn send_error(id: &Value, code: i64, message: &str) {
    send(&json!({"jsonrpc": "2.0", "id": id, "error": {"code": code, "message": message}}));
}

/// 发送 JSON-RPC 通知（无 `id` 字段）。
pub fn send_notification(method: &str, params: Value) {
    send(&json!({"jsonrpc": "2.0", "method": method, "params": params}));
}

/// 发送 `session/update` 通知（规范方法名），携带会话 id 与 `update` 载荷。
pub fn send_session_update(session_id: &str, update: Value) {
    send_notification(
        "session/update",
        json!({
            "sessionId": session_id,
            "update": update
        }),
    );
}

/// 将内置工具名映射为 ACP [`ToolKind`](https://agentclientprotocol.com) 常量字符串。
pub fn infer_tool_kind(name: &str) -> &'static str {
    match name {
        "read_file" => "read",
        "list_dir" => "read",
        "search_code" => "search",
        "llm_chat" => "think",
        _ if name.contains("search") => "search",
        _ if name.contains("read") => "read",
        _ => "other",
    }
}

/// 流式输出代理可见文本（`agent_message_chunk`）。
pub fn notify_text(session_id: &str, text: &str) {
    send_session_update(
        session_id,
        json!({
            "sessionUpdate": "agent_message_chunk",
            "content": {
                "type": "text",
                "text": text
            }
        }),
    );
}

/// 流式输出「思考」块（`agent_thought_chunk`），使用空文本作为占位。
pub fn notify_thinking(session_id: &str) {
    send_session_update(
        session_id,
        json!({
            "sessionUpdate": "agent_thought_chunk",
            "content": {
                "type": "text",
                "text": ""
            }
        }),
    );
}

/// 报告新发起的工具调用（`tool_call`）。`kind` 须为 schema 允许的 `ToolKind` 之一。
pub fn notify_tool_start(session_id: &str, tool_call_id: &str, title: &str, kind: &str) {
    send_session_update(
        session_id,
        json!({
            "sessionUpdate": "tool_call",
            "toolCallId": tool_call_id,
            "title": title,
            "kind": kind
        }),
    );
}

/// 报告工具调用状态变化（`tool_call_update`）。`status` 须为 `pending` | `in_progress` | `completed` | `failed`。
pub fn notify_tool_done(session_id: &str, tool_call_id: &str, status: &str) {
    send_session_update(
        session_id,
        json!({
            "sessionUpdate": "tool_call_update",
            "toolCallId": tool_call_id,
            "status": status
        }),
    );
}
