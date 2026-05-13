//! MCP server functionality


use anyhow::Result;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::RwLock;
use xavier::memory::qmd_memory::{MemoryDocument, QmdMemory};
use xavier::memory::sqlite_vec_store::VecSqliteMemoryStore;
use xavier::memory::store::{MemoryRecord, MemoryStore};

pub async fn start_mcp_stdio() -> Result<()> {
    // Initialize memory store directly (same as HTTP server)
    let store: Arc<dyn MemoryStore> = Arc::new(VecSqliteMemoryStore::from_env().await?);
    let workspace_id =
        std::env::var("XAVIER_DEFAULT_WORKSPACE_ID").unwrap_or_else(|_| "default".to_string());
    let durable_state = store.load_workspace_state(&workspace_id).await?;
    let docs = Arc::new(RwLock::new(
        durable_state
            .memories
            .iter()
            .map(MemoryRecord::to_document)
            .collect::<Vec<MemoryDocument>>(),
    ));
    let memory = Arc::new(QmdMemory::new_with_workspace(docs, workspace_id.clone()));
    memory.set_store(Arc::clone(&store)).await;
    memory.init().await?;

    // Async stdin/stdout
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();
    let mut reader = BufReader::new(stdin).lines();
    let mut writer = tokio::io::BufWriter::new(stdout);

    const JSONRPC: &str = "2.0";

    loop {
        // Read next line (blocking until client sends something)
        let line = match reader.next_line().await {
            Ok(Some(l)) => l,
            Ok(None) => break, // EOF
            Err(_e) => {
                // Read error - bail out silently
                break;
            }
        };

        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        // Parse JSON-RPC request
        let request: serde_json::Value = match serde_json::from_str(trimmed) {
            Ok(v) => v,
            Err(e) => {
                // Parse error - respond and continue
                let resp = serde_json::json!({
                    "jsonrpc": JSONRPC,
                    "id": serde_json::Value::Null,
                    "error": {
                        "code": -32700,
                        "message": format!("Parse error: {}", e)
                    }
                });
                let line = format!("{}\n", resp);
                let _ = writer.write_all(line.as_bytes()).await;
                let _ = writer.flush().await;
                continue;
            }
        };

        let method = request.get("method").and_then(|v| v.as_str());
        let id = request.get("id");
        // A notification has id === null or no id field
        let is_notification = id.as_ref().is_none_or(|v| v.is_null());

        let response: Option<serde_json::Value> = match method {
            None => Some(serde_json::json!({
                "jsonrpc": JSONRPC,
                "id": serde_json::Value::Null,
                "error": {
                    "code": -32600,
                    "message": "Invalid Request: method is required"
                }
            })),

            // ── initialize ──────────────────────────────────────────────
            Some("initialize") => {
                // initialized = true;  // MCP spec: remember to enforce pre-init rejection if needed
                Some(serde_json::json!({
                    "jsonrpc": JSONRPC,
                    "id": id,
                    "result": {
                        "protocolVersion": "2024-11-05",
                        "capabilities": { "tools": {} },
                        "serverInfo": {
                            "name": "xavier",
                            "version": "0.4.1"
                        }
                    }
                }))
            }

            // ── initialized (notification - no response) ───────────────
            Some("initialized") if is_notification => None,

            // ── tools/list ──────────────────────────────────────────────
            Some("tools/list") => Some(serde_json::json!({
                "jsonrpc": JSONRPC,
                "id": id,
                "result": {
                    "tools": [
                        {
                            "name": "search_memory",
                            "description": "Search memories in Xavier vector store",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "query": {
                                        "type": "string",
                                        "description": "Search query string"
                                    },
                                    "limit": {
                                        "type": "number",
                                        "description": "Max results (default 10, max 100)",
                                        "default": 10
                                    }
                                },
                                "required": ["query"]
                            }
                        },
                        {
                            "name": "create_memory",
                            "description": "Add a memory to Xavier",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "path": {
                                        "type": "string",
                                        "description": "Optional path/identifier for the memory"
                                    },
                                    "content": {
                                        "type": "string",
                                        "description": "Memory content text"
                                    },
                                    "title": {
                                        "type": "string",
                                        "description": "Optional title"
                                    }
                                },
                                "required": ["content"]
                            }
                        },
                        {
                            "name": "search",
                            "description": "Legacy alias for search_memory",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "query": { "type": "string" },
                                    "limit": { "type": "number", "default": 10 }
                                },
                                "required": ["query"]
                            }
                        },
                        {
                            "name": "add",
                            "description": "Legacy alias for create_memory",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "content": { "type": "string" },
                                    "title": { "type": "string" }
                                },
                                "required": ["content"]
                            }
                        },
                        {
                            "name": "lend_secret",
                            "description": "Lend an ephemeral secret to an agent with TTL",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "secret_name": { "type": "string" },
                                    "secret_value": { "type": "string" },
                                    "agent_id": { "type": "string" },
                                    "ttl_seconds": { "type": "number", "default": 3600 }
                                },
                                "required": ["secret_name", "secret_value", "agent_id"]
                            }
                        },
                        {
                            "name": "list_active_leases",
                            "description": "List all active ephemeral secret leases",
                            "inputSchema": {
                                "type": "object",
                                "properties": {}
                            }
                        },
                        {
                            "name": "revoke_secret",
                            "description": "Revoke an ephemeral secret lease by token",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "token": { "type": "string" }
                                },
                                "required": ["token"]
                            }
                        }
                    ]
                }
            })),

            // ── tools/call ──────────────────────────────────────────────
            Some("tools/call") => {
                let args = request
                    .get("params")
                    .and_then(|p| p.get("arguments"))
                    .and_then(|a| a.as_object())
                    .cloned()
                    .unwrap_or_default();

                let tool_name = request
                    .get("params")
                    .and_then(|p| p.get("name"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("");

                let text = match tool_name {
                    "search" | "search_memory" => {
                        let query = args.get("query").and_then(|v| v.as_str()).unwrap_or("");
                        let limit =
                            args.get("limit").and_then(|v| v.as_u64()).unwrap_or(10) as usize;
                        let limit = limit.clamp(1, 100);
                        match memory.search(query, limit).await {
                            Ok(results) => {
                                let summary = serde_json::json!({
                                    "count": results.len(),
                                    "results": results.into_iter().map(|doc| {
                                        serde_json::json!({
                                            "id": doc.id,
                                            "content": doc.content,
                                        })
                                    }).collect::<Vec<_>>()
                                });
                                serde_json::to_string_pretty(&summary).unwrap_or_default()
                            }
                            Err(e) => format!("{{\"error\": \"{}\"}}", e),
                        }
                    }
                    "add" | "create_memory" => {
                        let content = args.get("content").and_then(|v| v.as_str()).unwrap_or("");
                        let title = args.get("title").and_then(|v| v.as_str());
                        let path = args
                            .get("path")
                            .and_then(|v| v.as_str())
                            .map(str::to_string)
                            .unwrap_or_else(|| format!("memory/{}", ulid::Ulid::new()));
                        let mut metadata = serde_json::json!({});
                        if let Some(t) = title {
                            if let Some(obj) = metadata.as_object_mut() {
                                obj.insert("title".to_string(), serde_json::json!(t));
                            }
                        }
                        match memory
                            .add_document(path.clone(), content.to_string(), metadata)
                            .await
                        {
                            Ok(id) => serde_json::json!({
                                "status": "ok",
                                "path": path,
                                "id": id,
                            })
                            .to_string(),
                            Err(e) => format!("{{\"error\": \"{}\"}}", e),
                        }
                    }
                    "stats" => serde_json::json!({
                        "status": "ok",
                        "workspace_id": workspace_id,
                        "version": "0.4.1",
                    })
                    .to_string(),
                    "lend_secret" => {
                        // For MCP stdio, we'd need a reference to the global KeyLendingEngine
                        // Since start_mcp_stdio is standalone, we'll need to pass it or init it
                        "Error: Secrets tools require Xavier server to be running in HTTP mode for shared state management.".to_string()
                    }
                    _ => format!(
                        "Unknown tool: {}. Available tools: search_memory, create_memory, search, add, stats, lend_secret, list_active_leases, revoke_secret",
                        tool_name
                    ),
                };

                Some(serde_json::json!({
                    "jsonrpc": JSONRPC,
                    "id": id,
                    "result": {
                        "content": [{
                            "type": "text",
                            "text": text
                        }]
                    }
                }))
            }

            // ── Unknown method ──────────────────────────────────────────
            Some(m) => Some(serde_json::json!({
                "jsonrpc": JSONRPC,
                "id": id,
                "error": {
                    "code": -32601,
                    "message": format!("Method not found: {}", m)
                }
            })),
        };

        // Send response only if not a notification
        if !is_notification {
            if let Some(resp) = response {
                let line = format!("{}\n", resp);
                let _ = writer.write_all(line.as_bytes()).await;
                let _ = writer.flush().await;
            }
        }
    }

    Ok(())
}
