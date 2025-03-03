use tokio::{net::TcpListener, io::{AsyncReadExt, AsyncWriteExt}};
use serde::{Serialize, Deserialize};
use serde_json::{json, Value};
use std::error::Error;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[derive(Debug, Serialize, Deserialize)]
struct MCPRequest {
    id: String,
    method: String,
    params: Value,
}

#[derive(Debug, Serialize, Deserialize)]
struct MCPResponse {
    id: String,
    result: Value,
    error: Option<MCPError>,
}

#[derive(Debug, Serialize, Deserialize)]
struct MCPError {
    code: i32,
    message: String,
}

struct MCPServer {
    models: HashMap<String, ModelHandler>,
    sessions: Arc<Mutex<HashMap<String, Session>>>,
}

struct Session {
    model: String,
    context: Vec<String>,
}

type ModelHandler = fn(prompt: &str, context: &[String]) -> Result<String, Box<dyn Error + Send + Sync>>;

impl MCPServer {
    fn new() -> Self {
        MCPServer {
            models: HashMap::new(),
            sessions: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    fn register_model(&mut self, name: &str, handler: ModelHandler) {
        self.models.insert(name.to_string(), handler);
    }

    async fn handle_client(mut stream: tokio::net::TcpStream, models: HashMap<String, ModelHandler>, sessions: Arc<Mutex<HashMap<String, Session>>>) {
        let mut buffer = vec![0; 8192];
        
        match stream.read(&mut buffer).await {
            Ok(n) => {
                if n == 0 {
                    return;
                }
                
                let request_data = &buffer[0..n];
                let request_str = String::from_utf8_lossy(request_data);
                
                // Very basic HTTP parsing
                if request_str.starts_with("POST") {
                    // Find the JSON body after the double newline
                    if let Some(body_start) = request_str.find("\r\n\r\n") {
                        let body = &request_str[body_start + 4..];
                        
                        // Parse the JSON request
                        match serde_json::from_str::<MCPRequest>(body) {
                            Ok(request) => {
                                let response = Self::process_request(request, &models, &sessions).await;
                                let response_json = serde_json::to_string(&response).unwrap();
                                
                                // Send HTTP response
                                let http_response = format!(
                                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                                    response_json.len(),
                                    response_json
                                );
                                
                                let _ = stream.write_all(http_response.as_bytes()).await;
                            },
                            Err(e) => {
                                let error_response = json!({
                                    "id": "error",
                                    "error": {
                                        "code": -32700,
                                        "message": format!("Parse error: {}", e)
                                    },
                                    "result": null
                                });
                                
                                let error_json = serde_json::to_string(&error_response).unwrap();
                                let http_response = format!(
                                    "HTTP/1.1 400 Bad Request\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                                    error_json.len(),
                                    error_json
                                );
                                
                                let _ = stream.write_all(http_response.as_bytes()).await;
                            }
                        }
                    } else {
                        // No body found
                        let error_response = "HTTP/1.1 400 Bad Request\r\nContent-Type: text/plain\r\nContent-Length: 19\r\n\r\nMissing request body";
                        let _ = stream.write_all(error_response.as_bytes()).await;
                    }
                } else {
                    // Not a POST request
                    let error_response = "HTTP/1.1 405 Method Not Allowed\r\nContent-Type: text/plain\r\nContent-Length: 18\r\n\r\nUse POST requests";
                    let _ = stream.write_all(error_response.as_bytes()).await;
                }
            },
            Err(e) => {
                eprintln!("Failed to read from socket: {}", e);
            }
        }
    }

    async fn process_request(
        request: MCPRequest, 
        models: &HashMap<String, ModelHandler>,
        sessions: &Arc<Mutex<HashMap<String, Session>>>
    ) -> MCPResponse {
        match request.method.as_str() {
            "session.create" => {
                let model = match request.params.get("model") {
                    Some(Value::String(model)) => model.clone(),
                    _ => return Self::error_response(request.id, -32602, "Invalid params: missing model")
                };
                
                if !models.contains_key(&model) {
                    return Self::error_response(request.id, -32602, &format!("Model not found: {}", model));
                }
                
                let session_id = uuid::Uuid::new_v4().to_string();
                
                let mut sessions_guard = sessions.lock().unwrap();
                sessions_guard.insert(session_id.clone(), Session {
                    model,
                    context: Vec::new(),
                });
                
                MCPResponse {
                    id: request.id,
                    result: json!({ "session_id": session_id }),
                    error: None,
                }
            },
            "session.generate" => {
                let session_id = match request.params.get("session_id") {
                    Some(Value::String(sid)) => sid.clone(),
                    _ => return Self::error_response(request.id, -32602, "Invalid params: missing session_id")
                };
                
                let prompt = match request.params.get("prompt") {
                    Some(Value::String(p)) => p,
                    _ => return Self::error_response(request.id, -32602, "Invalid params: missing prompt")
                };
                
                let mut sessions_guard = sessions.lock().unwrap();
                let session = match sessions_guard.get_mut(&session_id) {
                    Some(s) => s,
                    None => return Self::error_response(request.id, -32602, &format!("Session not found: {}", session_id))
                };
                
                let model_handler = match models.get(&session.model) {
                    Some(handler) => handler,
                    None => return Self::error_response(request.id, -32603, "Internal error: model handler not found")
                };
                
                match model_handler(&prompt, &session.context) {
                    Ok(response) => {
                        // Add to context
                        session.context.push(prompt.clone());
                        session.context.push(response.clone());
                        
                        MCPResponse {
                            id: request.id,
                            result: json!({ "response": response }),
                            error: None,
                        }
                    },
                    Err(e) => Self::error_response(request.id, -32603, &format!("Model error: {}", e))
                }
            },
            "session.close" => {
                let session_id = match request.params.get("session_id") {
                    Some(Value::String(sid)) => sid.clone(),
                    _ => return Self::error_response(request.id, -32602, "Invalid params: missing session_id")
                };
                
                let mut sessions_guard = sessions.lock().unwrap();
                if sessions_guard.remove(&session_id).is_none() {
                    return Self::error_response(request.id, -32602, &format!("Session not found: {}", session_id));
                }
                
                MCPResponse {
                    id: request.id,
                    result: json!({"success": true}),
                    error: None,
                }
            },
            "models.list" => {
                let model_names: Vec<String> = models.keys().cloned().collect();
                
                MCPResponse {
                    id: request.id,
                    result: json!({"models": model_names}),
                    error: None,
                }
            },
            _ => Self::error_response(request.id, -32601, &format!("Method not found: {}", request.method))
        }
    }
    
    fn error_response(id: String, code: i32, message: &str) -> MCPResponse {
        MCPResponse {
            id,
            result: Value::Null,
            error: Some(MCPError {
                code,
                message: message.to_string(),
            }),
        }
    }
}

// Example model handler
fn example_model_handler(prompt: &str, context: &[String]) -> Result<String, Box<dyn Error + Send + Sync>> {
    let context_len = context.len() / 2;
    Ok(format!("Response to: {}. This is turn #{} in the conversation.", prompt, context_len + 1))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let addr = "127.0.0.1:8080";
    let listener = TcpListener::bind(addr).await?;
    println!("HTTP MCP Server listening on {}", addr);
    
    // Set up the server with registered models
    let mut server = MCPServer::new();
    server.register_model("example-model", example_model_handler);
    
    let models = server.models.clone();
    let sessions = server.sessions.clone();
    
    // Accept connections
    while let Ok((stream, addr)) = listener.accept().await {
        println!("New client connected: {}", addr);
        
        let client_models = models.clone();
        let client_sessions = sessions.clone();
        
        tokio::spawn(async move {
            MCPServer::handle_client(stream, client_models, client_sessions).await;
        });
    }
    
    Ok(())
}
