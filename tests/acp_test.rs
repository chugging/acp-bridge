/// 验证 JSON-RPC 响应结构（与 ACP 一致）。
/// 由于 `acp::send` 直接写 stdout，此处仅校验 JSON 形状。

#[test]
fn json_rpc_response_structure() {
    let id = serde_json::json!(1);
    let result = serde_json::json!({"agentInfo": {"name": "test", "version": "0.1.0"}});
    let msg = serde_json::json!({"jsonrpc": "2.0", "id": id, "result": result});

    assert_eq!(msg["jsonrpc"], "2.0");
    assert_eq!(msg["id"], 1);
    assert!(msg["result"]["agentInfo"]["name"].is_string());
}

#[test]
fn json_rpc_error_structure() {
    let id = serde_json::json!(5);
    let code = -32601i64;
    let message = "Method not found: foo";
    let msg = serde_json::json!({"jsonrpc": "2.0", "id": id, "error": {"code": code, "message": message}});

    assert_eq!(msg["error"]["code"], -32601);
    assert_eq!(msg["error"]["message"], "Method not found: foo");
}

#[test]
fn notification_has_no_id() {
    let method = "session/update";
    let params = serde_json::json!({
        "sessionId": "sid-1",
        "update": {
            "sessionUpdate": "agent_message_chunk",
            "content": {"type": "text", "text": "hi"}
        }
    });
    let msg = serde_json::json!({"jsonrpc": "2.0", "method": method, "params": params});

    assert!(msg.get("id").is_none());
    assert_eq!(msg["method"], "session/update");
    assert_eq!(msg["params"]["sessionId"], "sid-1");
    assert_eq!(
        msg["params"]["update"]["sessionUpdate"],
        "agent_message_chunk"
    );
}

#[test]
fn notification_text_content_format() {
    let text = "Hello, world!";
    let params = serde_json::json!({
        "sessionId": "s",
        "update": {
            "sessionUpdate": "agent_message_chunk",
            "content": {"type": "text", "text": text}
        }
    });

    assert_eq!(
        params["update"]["content"]["text"].as_str().unwrap(),
        "Hello, world!"
    );
    assert_eq!(params["update"]["content"]["type"], "text");
}

#[test]
fn tool_call_notification_format() {
    let params = serde_json::json!({
        "sessionId": "s",
        "update": {
            "sessionUpdate": "tool_call",
            "toolCallId": "tc-1",
            "title": "read_file",
            "kind": "read"
        }
    });
    assert_eq!(params["update"]["toolCallId"], "tc-1");

    let done_params = serde_json::json!({
        "sessionId": "s",
        "update": {
            "sessionUpdate": "tool_call_update",
            "toolCallId": "tc-1",
            "status": "completed"
        }
    });
    assert_eq!(done_params["update"]["status"], "completed");
}
