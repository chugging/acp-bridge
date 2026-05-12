use serde::Deserialize;
use serde_json::Value;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

/// JSON-RPC 请求行（stdin 一行）。`id` 可为空：客户端通知（如 `session/cancel`）无 `id`。
#[derive(Debug, Deserialize)]
pub struct JsonRpcRequest {
    #[serde(default)]
    pub id: Option<Value>,
    pub method: String,
    pub params: Option<Value>,
}

pub struct Session {
    pub messages: Vec<Value>,
    /// Last activity timestamp for idle timeout.
    pub last_active: Instant,
    /// Working directory for this session (used for tool sandboxing).
    pub working_dir: PathBuf,
    /// 客户端发送 `session/cancel` 后置位；下一轮 `session/prompt` 开始时清零。
    pub cancelled: AtomicBool,
    /// ACP session mode: `ask` | `plan` | `agent`（与 JetBrains / ACP `configOptions` 对齐）。
    pub mode: String,
    /// 本会话使用的后端模型 id（可由 `session/set_config_option` 覆盖）。
    pub model: String,
}

impl Session {
    pub fn new(system_message: Value, working_dir: PathBuf, model: String) -> Self {
        Self {
            messages: vec![system_message],
            last_active: Instant::now(),
            working_dir,
            cancelled: AtomicBool::new(false),
            mode: "agent".into(),
            model,
        }
    }

    /// 是否为协议支持的三种模式之一。
    pub fn is_valid_mode(mode: &str) -> bool {
        matches!(mode, "ask" | "plan" | "agent")
    }

    /// 由客户端取消当前 prompt 轮次时调用。
    pub fn request_cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }

    /// 是否已请求取消（供 prompt 循环轮询）。
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }

    /// 新一轮用户 prompt 开始时清除取消标志。
    pub fn clear_cancel(&self) {
        self.cancelled.store(false, Ordering::SeqCst);
    }

    pub fn touch(&mut self) {
        self.last_active = Instant::now();
    }
}

impl Session {
    /// Trim conversation history to keep the system prompt + last `max_turns` pairs.
    /// Each "turn" = one user message + one assistant message.
    /// The system prompt (first message) is always preserved.
    pub fn trim_history(&mut self, max_turns: usize) {
        // messages[0] = system prompt, then alternating user/assistant
        let keep = max_turns * 2; // user + assistant per turn
        if self.messages.len() > keep + 1 {
            let system = self.messages[0].clone();
            let tail = self.messages.split_off(self.messages.len() - keep);
            self.messages = vec![system];
            self.messages.extend(tail);
        }
    }
}

/// ACP-layer error codes following JSON-RPC 2.0 conventions.
#[derive(Debug, thiserror::Error)]
pub enum AcpError {
    #[error("Missing required parameter: {field}")]
    MissingParam { field: String },

    #[error("Unknown session: {session_id}")]
    UnknownSession { session_id: String },

    #[error("Method not found: {method}")]
    MethodNotFound { method: String },

    #[error("LLM communication error: {reason}")]
    LlmError { reason: String },

    #[error("Session limit reached (max: {max})")]
    SessionLimitReached { max: usize },
}

impl AcpError {
    /// JSON-RPC error code for this variant.
    pub fn code(&self) -> i64 {
        match self {
            AcpError::MissingParam { .. } => -32602,   // Invalid params
            AcpError::UnknownSession { .. } => -32001, // Application error
            AcpError::MethodNotFound { .. } => -32601, // Method not found
            AcpError::LlmError { .. } => -32003,       // Application error
            AcpError::SessionLimitReached { .. } => -32004, // Application error
        }
    }
}
