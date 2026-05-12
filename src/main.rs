//! acp-bridge — ACP adapter for local AI services.
//!
//! Bridges any OpenAI-compatible API (Ollama, LocalAI, vLLM, llama.cpp,
//! LM Studio, text-generation-webui) to Agent Client Protocol (ACP).
//!
//! Reads JSON-RPC 2.0 from stdin, translates to HTTP chat completions,
//! and writes JSON-RPC notifications/responses to stdout.
//!
//! Compatible with openab and any ACP-compliant harness.

use acp_bridge::acp;
use acp_bridge::config::ConfigFile;
use acp_bridge::llm;
use acp_bridge::protocol::{AcpError, JsonRpcRequest, Session};
use acp_bridge::tools;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::RwLock;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, BufReader};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// Maximum number of tool call rounds to prevent infinite loops.
const MAX_TOOL_ROUNDS: usize = 5;

// ---------------------------------------------------------------------------
// Global state
// ---------------------------------------------------------------------------

static SESSIONS: std::sync::LazyLock<RwLock<HashMap<String, Session>>> =
    std::sync::LazyLock::new(|| RwLock::new(HashMap::new()));

/// 后台探活拉取到的模型列表；`session/new` 与 `session/set_config_option` 复用。
static CACHED_BACKEND_MODELS: std::sync::LazyLock<RwLock<Vec<String>>> =
    std::sync::LazyLock::new(|| RwLock::new(Vec::new()));

// ---------------------------------------------------------------------------
// Session helpers
// ---------------------------------------------------------------------------

fn sessions_write() -> std::sync::RwLockWriteGuard<'static, HashMap<String, Session>> {
    match SESSIONS.write() {
        Ok(s) => s,
        Err(p) => {
            warn!("Session lock poisoned, recovering");
            p.into_inner()
        }
    }
}

fn sessions_read() -> std::sync::RwLockReadGuard<'static, HashMap<String, Session>> {
    match SESSIONS.read() {
        Ok(s) => s,
        Err(p) => {
            warn!("Session lock poisoned, recovering");
            p.into_inner()
        }
    }
}

fn cached_models_read() -> std::sync::RwLockReadGuard<'static, Vec<String>> {
    match CACHED_BACKEND_MODELS.read() {
        Ok(m) => m,
        Err(p) => {
            warn!("Model cache lock poisoned, recovering");
            p.into_inner()
        }
    }
}

fn cached_models_write() -> std::sync::RwLockWriteGuard<'static, Vec<String>> {
    match CACHED_BACKEND_MODELS.write() {
        Ok(m) => m,
        Err(p) => {
            warn!("Model cache lock poisoned, recovering");
            p.into_inner()
        }
    }
}

/// 保证默认配置的模型出现在下拉列表中。
fn merge_models_with_default(config: &llm::LlmConfig, mut models: Vec<String>) -> Vec<String> {
    if !models.iter().any(|m| m == &config.model) {
        models.insert(0, config.model.clone());
    }
    models
}

/// 供 `configOptions` 使用的模型列表：**只读**后台探活写入的缓存，不在此处发起 HTTP。
///
/// 若在探活完成前创建会话，列表仅含当前配置的 `LLM_MODEL`；探活结束后缓存更新，
/// 新开会话或 `session/set_config_option` 会拿到完整列表。这样 `session/new` 永不阻塞
/// stdin 主循环，避免 JetBrains 等客户端因响应超时发送 SIGTERM（退出码 143）。
fn model_list_for_config_options(config: &llm::LlmConfig) -> Vec<String> {
    let cached = cached_models_read();
    if !cached.is_empty() {
        merge_models_with_default(config, cached.clone())
    } else {
        debug!(
            "Model cache empty — configOptions list only configured model until probe completes"
        );
        vec![config.model.clone()]
    }
}

fn mode_instructions(mode: &str) -> &'static str {
    match mode {
        "ask" => "You are in Ask mode. Answer conversationally without using workspace tools; if you need file contents, ask the user to paste them.",
        "plan" => "You are in Plan mode. Produce clear plans and tradeoffs; use read-only tools only when they substantially improve the plan.",
        "agent" => "You are in Agent mode. Use the provided tools when they help you give accurate, actionable answers.",
        _ => "",
    }
}

/// 为 LLM 请求构造消息列表：在系统提示后附加当前模式说明。
fn augment_messages_for_llm(messages: &[Value], mode: &str) -> Vec<Value> {
    if messages.is_empty() {
        return vec![];
    }
    let mut out = messages.to_vec();
    if out[0].get("role").and_then(|r| r.as_str()) == Some("system") {
        let base = out[0]
            .get("content")
            .and_then(|c| c.as_str())
            .unwrap_or("");
        let suffix = mode_instructions(mode);
        let combined = if suffix.is_empty() {
            base.to_string()
        } else {
            format!("{base}\n\n{suffix}")
        };
        out[0] = json!({"role": "system", "content": combined});
    }
    out
}

fn build_config_options(session: &Session, models: &[String]) -> Vec<Value> {
    let model_options: Vec<Value> = models
        .iter()
        .map(|m| {
            json!({
                "value": m,
                "name": m,
                "description": serde_json::Value::Null
            })
        })
        .collect();

    vec![
        json!({
            "id": "mode",
            "name": "Session mode",
            "description": "Ask: chat only. Plan: structured planning with read-only exploration. Agent: full tool use.",
            "category": "mode",
            "type": "select",
            "currentValue": session.mode,
            "options": [
                {"value": "ask", "name": "Ask", "description": "Answer without running workspace tools."},
                {"value": "plan", "name": "Plan", "description": "Focus on plans and analysis; read-only tools when helpful."},
                {"value": "agent", "name": "Agent", "description": "Use tools to inspect the project and respond accurately."}
            ]
        }),
        json!({
            "id": "model",
            "name": "Model",
            "description": "Backend language model for this session.",
            "category": "model",
            "type": "select",
            "currentValue": session.model,
            "options": model_options
        }),
    ]
}

fn modes_state_json(current: &str) -> Value {
    json!({
        "currentModeId": current,
        "availableModes": [
            {"id": "ask", "name": "Ask", "description": "Answer without running workspace tools."},
            {"id": "plan", "name": "Plan", "description": "Focus on plans and analysis; read-only tools when helpful."},
            {"id": "agent", "name": "Agent", "description": "Use tools to inspect the project and respond accurately."}
        ]
    })
}

/// Evict sessions that have been idle longer than the timeout.
fn evict_idle_sessions(timeout_secs: u64) {
    if timeout_secs == 0 {
        return;
    }
    let timeout = Duration::from_secs(timeout_secs);
    let mut sessions = sessions_write();
    let before = sessions.len();
    sessions.retain(|_id, session| session.last_active.elapsed() < timeout);
    let evicted = before - sessions.len();
    if evicted > 0 {
        info!(evicted, remaining = sessions.len(), "Evicted idle sessions");
    }
}

// ---------------------------------------------------------------------------
// ACP method handlers
// ---------------------------------------------------------------------------

fn handle_initialize(id: &Value, params: &Value, config: &llm::LlmConfig) {
    let client_pv = params
        .get("protocolVersion")
        .and_then(|v| v.as_u64().or_else(|| v.as_i64().map(|i| i as u64)))
        .unwrap_or(1);
    let negotiated = client_pv.min(1);

    info!(model = %config.model, base_url = %config.base_url, negotiated, "Initialize");

    acp::send_response(
        id,
        json!({
            "protocolVersion": negotiated,
            "agentInfo": {
                "name": format!("acp-bridge ({})", config.model),
                "version": env!("CARGO_PKG_VERSION")
            },
            "agentCapabilities": {
                "loadSession": false,
                "mcpCapabilities": { "http": false, "sse": false },
                "promptCapabilities": {
                    "audio": false,
                    "embeddedContext": false,
                    "image": false
                },
                "sessionCapabilities": {
                    "close": {}
                }
            },
            "authMethods": []
        }),
    );
}

/// 处理客户端 `session/cancel` 通知：在当前会话上置取消位（无 JSON-RPC 响应）。
fn handle_session_cancel(params: &Value) {
    let Some(sid) = params.get("sessionId").and_then(|v| v.as_str()) else {
        warn!("session/cancel missing sessionId");
        return;
    };
    let sessions = sessions_read();
    if let Some(s) = sessions.get(sid) {
        s.request_cancel();
        info!(session_id = sid, "session/cancel received");
    } else {
        warn!(session_id = sid, "session/cancel for unknown session");
    }
}

fn handle_session_new(id: &Value, params: &Value, config: &llm::LlmConfig) {
    // Enforce max_sessions limit
    if config.max_sessions > 0 {
        let count = sessions_read().len();
        if count >= config.max_sessions {
            let err = AcpError::SessionLimitReached {
                max: config.max_sessions,
            };
            acp::send_error(id, err.code(), &err.to_string());
            return;
        }
    }

    let raw_cwd = params.get("cwd").and_then(|v| v.as_str()).unwrap_or("/tmp");

    // Sanitize cwd: only allow typical path characters to prevent prompt injection.
    let cwd: String = raw_cwd
        .chars()
        .filter(|c| c.is_alphanumeric() || matches!(c, '/' | '-' | '_' | '.' | ' ' | '~'))
        .collect();

    let session_id = Uuid::new_v4().to_string();

    let system_prompt = std::env::var("LLM_SYSTEM_PROMPT").unwrap_or_else(|_| {
        format!("You are a helpful coding assistant. The user's working directory is: {cwd}")
    });

    let models = model_list_for_config_options(config);
    let initial_model = config.model.clone();

    let session = Session::new(
        json!({"role": "system", "content": system_prompt}),
        PathBuf::from(&cwd),
        initial_model,
    );
    sessions_write().insert(session_id.clone(), session);

    let (config_options, modes) = {
        let sessions = sessions_read();
        let session = sessions.get(&session_id).expect("session just inserted");
        (
            build_config_options(session, &models),
            modes_state_json(&session.mode),
        )
    };

    info!(session_id = %session_id, max_history = config.max_history_turns, "New session");
    acp::send_response(
        id,
        json!({
            "sessionId": session_id,
            "configOptions": config_options,
            "modes": modes
        }),
    );
}

/// `session/set_config_option` — 更新模式或模型后返回完整 `configOptions`。
fn handle_session_set_config_option(id: &Value, params: &Value, config: &llm::LlmConfig) {
    let session_id = match params.get("sessionId").and_then(|v| v.as_str()) {
        Some(s) => s.to_string(),
        None => {
            let err = AcpError::MissingParam {
                field: "sessionId".into(),
            };
            acp::send_error(id, err.code(), &err.to_string());
            return;
        }
    };
    let config_id = match params.get("configId").and_then(|v| v.as_str()) {
        Some(s) => s.to_string(),
        None => {
            let err = AcpError::MissingParam {
                field: "configId".into(),
            };
            acp::send_error(id, err.code(), &err.to_string());
            return;
        }
    };
    let value = match params.get("value").and_then(|v| v.as_str()) {
        Some(s) => s.to_string(),
        None => {
            let err = AcpError::MissingParam {
                field: "value".into(),
            };
            acp::send_error(id, err.code(), &err.to_string());
            return;
        }
    };

    let models = model_list_for_config_options(config);

    match config_id.as_str() {
        "mode" => {
            if !Session::is_valid_mode(&value) {
                acp::send_error(
                    id,
                    -32602,
                    "Invalid mode value (expected ask, plan, or agent)",
                );
                return;
            }
        }
        "model" => {
            if !models.iter().any(|m| m == &value) {
                acp::send_error(
                    id,
                    -32602,
                    "Model value is not in the available models list",
                );
                return;
            }
        }
        _ => {
            acp::send_error(
                id,
                -32602,
                &format!("Unknown config option: {config_id}"),
            );
            return;
        }
    }

    let mut sessions = sessions_write();
    let session = match sessions.get_mut(&session_id) {
        Some(s) => s,
        None => {
            let err = AcpError::UnknownSession {
                session_id: session_id.clone(),
            };
            acp::send_error(id, err.code(), &err.to_string());
            return;
        }
    };

    match config_id.as_str() {
        "mode" => {
            session.mode = value;
        }
        "model" => {
            session.model = value;
        }
        _ => {}
    }

    let opts = build_config_options(session, &models);
    acp::send_response(id, json!({ "configOptions": opts }));
}

fn handle_session_set_mode(id: &Value, params: &Value) {
    let session_id = match params.get("sessionId").and_then(|v| v.as_str()) {
        Some(s) => s.to_string(),
        None => {
            let err = AcpError::MissingParam {
                field: "sessionId".into(),
            };
            acp::send_error(id, err.code(), &err.to_string());
            return;
        }
    };
    let mode_id = match params.get("modeId").and_then(|v| v.as_str()) {
        Some(s) => s.to_string(),
        None => {
            let err = AcpError::MissingParam {
                field: "modeId".into(),
            };
            acp::send_error(id, err.code(), &err.to_string());
            return;
        }
    };

    if !Session::is_valid_mode(&mode_id) {
        acp::send_error(
            id,
            -32602,
            "Invalid modeId (expected ask, plan, or agent)",
        );
        return;
    }

    let mut sessions = sessions_write();
    let Some(session) = sessions.get_mut(&session_id) else {
        let err = AcpError::UnknownSession {
            session_id: session_id.clone(),
        };
        acp::send_error(id, err.code(), &err.to_string());
        return;
    };
    session.mode = mode_id;
    acp::send_response(id, json!({}));
}

async fn handle_session_prompt(id: &Value, params: &Value, config: &llm::LlmConfig) {
    let session_id = match params.get("sessionId").and_then(|v| v.as_str()) {
        Some(s) => s.to_string(),
        None => {
            let err = AcpError::MissingParam {
                field: "sessionId".into(),
            };
            acp::send_error(id, err.code(), &err.to_string());
            return;
        }
    };

    let user_text = params
        .get("prompt")
        .and_then(|p| p.as_array())
        .map(|arr| {
            arr.iter()
                .filter(|p| p.get("type").and_then(|t| t.as_str()) == Some("text"))
                .filter_map(|p| p.get("text").and_then(|t| t.as_str()))
                .collect::<Vec<_>>()
                .join("\n")
        })
        .unwrap_or_default();

    // Add user message, touch session, trim history, snapshot mode/model for this turn
    let (session_mode, session_model) = {
        let mut sessions = sessions_write();
        let session = match sessions.get_mut(&session_id) {
            Some(s) => s,
            None => {
                let err = AcpError::UnknownSession {
                    session_id: session_id.clone(),
                };
                acp::send_error(id, err.code(), &err.to_string());
                return;
            }
        };
        session.clear_cancel();
        session.touch();
        session
            .messages
            .push(json!({"role": "user", "content": user_text}));

        if config.max_history_turns > 0 {
            let before = session.messages.len();
            session.trim_history(config.max_history_turns);
            let after = session.messages.len();
            if before != after {
                debug!(before, after, "Trimmed conversation history");
            }
        }
        (session.mode.clone(), session.model.clone())
    };

    let llm_tool_id = Uuid::new_v4().to_string();
    acp::notify_thinking(&session_id);
    acp::notify_tool_start(
        &session_id,
        &llm_tool_id,
        "LLM chat",
        acp::infer_tool_kind("llm_chat"),
    );

    let mut had_error = false;
    let mut cancelled_turn = false;

    // Tool call loop: LLM may request tools, we execute and feed results back
    for round in 0..MAX_TOOL_ROUNDS {
        if sessions_read()
            .get(&session_id)
            .map(|s| s.is_cancelled())
            .unwrap_or(false)
        {
            cancelled_turn = true;
            break;
        }

        let messages_raw = {
            let sessions = sessions_read();
            sessions
                .get(&session_id)
                .map(|s| s.messages.clone())
                .unwrap_or_default()
        };
        let messages = augment_messages_for_llm(&messages_raw, &session_mode);

        let working_dir = {
            let sessions = sessions_read();
            sessions
                .get(&session_id)
                .map(|s| s.working_dir.clone())
                .unwrap_or_default()
        };

        let tool_defs = tools::tool_definitions_for_mode(session_mode.as_str());
        let tools_arg = if tool_defs.is_empty() {
            None
        } else {
            Some(tool_defs.as_slice())
        };

        // Try non-streaming first to check for tool calls
        let chat_result = llm::chat(
            config,
            &messages,
            Some(session_model.as_str()),
            tools_arg,
        )
        .await;

        match chat_result {
            Ok(response) => {
                // Check for tool calls in response
                let tool_calls = extract_tool_calls(&response);

                if tool_calls.is_empty() {
                    // No tool calls — extract text and stream it
                    let text = extract_response_text(&response);
                    if !text.is_empty() {
                        acp::notify_text(&session_id, &text);
                        let mut sessions = sessions_write();
                        if let Some(session) = sessions.get_mut(&session_id) {
                            session
                                .messages
                                .push(json!({"role": "assistant", "content": text}));
                        }
                    }
                    break;
                }

                // Has tool calls — execute them
                info!(round, count = tool_calls.len(), "Executing tool calls");

                // Add assistant message with tool_calls to history
                {
                    let mut sessions = sessions_write();
                    if let Some(session) = sessions.get_mut(&session_id) {
                        let assistant_msg = if config.is_ollama_native() {
                            json!({"role": "assistant", "content": "", "tool_calls": tool_calls})
                        } else {
                            // OpenAI format
                            response["choices"][0]["message"].clone()
                        };
                        session.messages.push(assistant_msg);
                    }
                }

                for tc in &tool_calls {
                    let func = &tc["function"];
                    let name = func["name"].as_str().unwrap_or("unknown");
                    let args_str = func["arguments"].as_str().unwrap_or("{}");
                    let args: Value = serde_json::from_str(args_str)
                        .unwrap_or_else(|_| func["arguments"].clone());

                    let tool_call_id = Uuid::new_v4().to_string();
                    let kind = acp::infer_tool_kind(name);
                    acp::notify_tool_start(&session_id, &tool_call_id, name, kind);
                    let result = tools::execute_tool(&working_dir, name, &args);
                    acp::notify_tool_done(&session_id, &tool_call_id, "completed");

                    debug!(tool = name, result_len = result.len(), "Tool executed");

                    // Add tool result to history
                    {
                        let mut sessions = sessions_write();
                        if let Some(session) = sessions.get_mut(&session_id) {
                            session.messages.push(json!({
                                "role": "tool",
                                "content": result
                            }));
                        }
                    }
                }

                // Continue loop — LLM will see tool results and respond
            }
            Err(e) => {
                let err = AcpError::LlmError { reason: e };
                error!(error = %err, "LLM communication failed");
                acp::notify_text(&session_id, &format!("\n\n**Error:** {err}\n"));
                had_error = true;
                break;
            }
        }
    }

    let llm_tool_status = if cancelled_turn || had_error {
        "failed"
    } else {
        "completed"
    };
    acp::notify_tool_done(&session_id, &llm_tool_id, llm_tool_status);

    let stop_reason = if cancelled_turn {
        "cancelled"
    } else if had_error {
        "refusal"
    } else {
        "end_turn"
    };
    acp::send_response(id, json!({ "stopReason": stop_reason }));
}

/// Extract tool calls from an LLM response (supports both Ollama and OpenAI format).
fn extract_tool_calls(response: &Value) -> Vec<Value> {
    // Ollama native: response.message.tool_calls
    if let Some(calls) = response
        .get("message")
        .and_then(|m| m.get("tool_calls"))
        .and_then(|tc| tc.as_array())
    {
        return calls.clone();
    }

    // OpenAI compat: response.choices[0].message.tool_calls
    if let Some(calls) = response
        .get("choices")
        .and_then(|c| c.get(0))
        .and_then(|c| c.get("message"))
        .and_then(|m| m.get("tool_calls"))
        .and_then(|tc| tc.as_array())
    {
        return calls.clone();
    }

    vec![]
}

/// Extract text content from an LLM response (supports both formats).
fn extract_response_text(response: &Value) -> String {
    // Ollama native: response.message.content
    if let Some(text) = response
        .get("message")
        .and_then(|m| m.get("content"))
        .and_then(|c| c.as_str())
    {
        if !text.is_empty() {
            return text.to_string();
        }
    }

    // OpenAI compat: response.choices[0].message.content
    if let Some(text) = response
        .get("choices")
        .and_then(|c| c.get(0))
        .and_then(|c| c.get("message"))
        .and_then(|m| m.get("content"))
        .and_then(|c| c.as_str())
    {
        return text.to_string();
    }

    String::new()
}

fn handle_session_end(id: &Value, params: &Value) {
    let session_id = match params.get("sessionId").and_then(|v| v.as_str()) {
        Some(s) => s.to_string(),
        None => {
            let err = AcpError::MissingParam {
                field: "sessionId".into(),
            };
            acp::send_error(id, err.code(), &err.to_string());
            return;
        }
    };

    let removed = sessions_write().remove(&session_id).is_some();

    if removed {
        info!(session_id = %session_id, "Session ended");
        acp::send_response(id, json!({}));
    } else {
        let err = AcpError::UnknownSession { session_id };
        acp::send_error(id, err.code(), &err.to_string());
    }
}

// ---------------------------------------------------------------------------
// Main loop
// ---------------------------------------------------------------------------

#[tokio::main]
async fn main() {
    // Handle --version / -V before anything else
    if let Some(arg) = std::env::args().nth(1) {
        if arg == "--version" || arg == "-V" {
            println!("acp-bridge {}", env!("CARGO_PKG_VERSION"));
            return;
        }
    }

    // Initialize tracing — writes to stderr, respects RUST_LOG env.
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "acp_bridge=info".parse().unwrap()),
        )
        .with_target(true)
        .with_writer(std::io::stderr)
        .init();

    // Load config: CLI arg (optional TOML path) → env vars → defaults
    let config_path = std::env::args().nth(1);
    let config = match config_path {
        Some(path) => {
            let file = ConfigFile::load(std::path::Path::new(&path));
            file.into_llm_config()
        }
        None => llm::LlmConfig::from_env(),
    };
    info!(
        version = env!("CARGO_PKG_VERSION"),
        model = %config.model,
        base_url = %config.base_url,
        ollama_native = config.is_ollama_native(),
        max_history_turns = config.max_history_turns,
        max_sessions = config.max_sessions,
        session_idle_timeout_secs = config.session_idle_timeout_secs,
        "Starting acp-bridge"
    );

    // 后台探活：避免阻塞 stdin，防止 IDE 在 `initialize` 前因超时发送 SIGTERM。
    let probe_cfg = config.clone();
    tokio::spawn(async move {
        match llm::probe_backend(&probe_cfg).await {
            Ok(models) if models.is_empty() => {
                info!("Connected to backend (no models listed)");
            }
            Ok(models) => {
                if !models.is_empty() {
                    *cached_models_write() = models.clone();
                }
                info!(count = models.len(), "Available models:");
                for m in &models {
                    info!("  - {m}");
                }
                if !models.iter().any(|m| {
                    m.starts_with(&probe_cfg.model)
                        || probe_cfg
                            .model
                            .starts_with(m.split(':').next().unwrap_or(""))
                }) {
                    warn!(
                        configured = %probe_cfg.model,
                        "Configured model not found in available models"
                    );
                }
            }
            Err(reason) => {
                warn!(
                    base_url = %probe_cfg.base_url,
                    error = %reason,
                    "Cannot reach backend — will retry on first request"
                );
            }
        }

        if let Some(info) = llm::query_model_info(&probe_cfg).await {
            info!(
                context_length = info.context_length,
                "Model info from /api/show"
            );
        }

        if let Some(running) = llm::query_running_models(&probe_cfg).await {
            if running.is_empty() {
                warn!(
                    model = %probe_cfg.model,
                    "No models loaded in VRAM — first request may be slow. Run: ollama run {}",
                    probe_cfg.model
                );
            } else {
                info!(count = running.len(), "Running models (loaded in VRAM):");
                for m in &running {
                    info!("  - {m}");
                }
            }
        }
    });

    // Spawn idle session cleanup task
    let idle_timeout = config.session_idle_timeout_secs;
    if idle_timeout > 0 {
        tokio::spawn(async move {
            let interval = Duration::from_secs(idle_timeout.min(60));
            loop {
                tokio::time::sleep(interval).await;
                evict_idle_sessions(idle_timeout);
            }
        });
    }

    let stdin = tokio::io::stdin();
    let reader = BufReader::new(stdin);
    let mut lines = reader.lines();

    loop {
        tokio::select! {
            line_result = lines.next_line() => {
                match line_result {
                    Ok(Some(line)) => {
                        let trimmed = line.trim().to_string();
                        if trimmed.is_empty() {
                            continue;
                        }

                        let msg: JsonRpcRequest = match serde_json::from_str(&trimmed) {
                            Ok(m) => m,
                            Err(e) => {
                                debug!(error = %e, "Skipping invalid JSON-RPC line");
                                continue;
                            }
                        };

                        let method = msg.method.as_str();
                        let params = msg.params.clone().unwrap_or(json!({}));

                        if method == "session/cancel" {
                            debug!(request_id = ?msg.id, "Received session/cancel");
                            handle_session_cancel(&params);
                            continue;
                        }

                        let Some(req_id) = msg.id.as_ref() else {
                            debug!(%method, "Skipping non-request line without id");
                            continue;
                        };

                        debug!(request_id = ?req_id, %method, "Received request");

                        match method {
                            "initialize" => handle_initialize(req_id, &params, &config),
                            "session/new" => handle_session_new(req_id, &params, &config),
                            "session/set_config_option" => {
                                handle_session_set_config_option(req_id, &params, &config)
                            }
                            "session/set_mode" => handle_session_set_mode(req_id, &params),
                            "session/prompt" => {
                                handle_session_prompt(req_id, &params, &config).await
                            }
                            "session/end" | "session/close" => handle_session_end(req_id, &params),
                            _ => {
                                let err = AcpError::MethodNotFound { method: method.to_string() };
                                acp::send_error(req_id, err.code(), &err.to_string());
                            }
                        }
                    }
                    Ok(None) => {
                        info!("stdin closed, shutting down gracefully");
                        break;
                    }
                    Err(e) => {
                        error!(error = %e, "Error reading stdin");
                        break;
                    }
                }
            }
            _ = tokio::signal::ctrl_c() => {
                info!("Received shutdown signal, exiting");
                break;
            }
        }
    }

    // Cleanup
    let session_count = {
        let mut s = sessions_write();
        let n = s.len();
        s.clear();
        n
    };
    if session_count > 0 {
        info!(sessions = session_count, "Cleaned up sessions on exit");
    }
}
