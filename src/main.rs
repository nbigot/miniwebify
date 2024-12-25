use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::thread;

// Configuration structures
#[derive(Debug, Deserialize)]
struct ServerConfig {
    host: String,
    port: u16,
}


#[derive(Debug, Deserialize)]
struct ResponseConfig {
    headers: HashMap<String, String>,
}

#[derive(Debug, Deserialize)]
struct EndpointConfig {
    command: String,
    args: Vec<String>,
    description: String,
    response: Option<ResponseConfig>,
}

#[derive(Debug, Deserialize)]
struct Endpoints {
    endpoints: HashMap<String, EndpointConfig>,
}

#[derive(Debug, Deserialize)]
struct Config {
    server: ServerConfig,
}

// HTTP response structure
#[derive(Serialize)]
struct CommandOutput {
    status: String,
    output: String,
    error: Option<String>,
}

struct HttpResponse {
    status_code: u16,
    headers: Vec<String>,
    body: String,
}

impl HttpResponse {
    fn new(status_code: u16, headers: Vec<String>, body: String) -> Self {
        HttpResponse {
            status_code,
            headers,
            body,
        }
    }

    fn to_string(&self) -> String {
        let status_text = match self.status_code {
            200 => "OK",
            404 => "Not Found",
            500 => "Internal Server Error",
            _ => "Unknown",
        };

        let headers_str = self.headers.join("\r\n");

        format!(
            "HTTP/1.1 {} {}\r\n\
             {}\r\n\
             Content-Length: {}\r\n\
             Connection: close\r\n\
             \r\n\
             {}",
            self.status_code,
            status_text,
            headers_str,
            self.body.len(),
            self.body
        )
    }
}

// Load server configuration from YAML file
fn load_config(path: &str) -> Result<Config, Box<dyn std::error::Error>> {
    let config_str = fs::read_to_string(path)?;
    let config: Config = serde_yaml::from_str(&config_str)?;
    Ok(config)
}

// Load endpoints configuration from YAML file
fn load_endpoints(path: &str) -> Result<Endpoints, Box<dyn std::error::Error>> {
    let endpoints_str = fs::read_to_string(path)?;
    let endpoints: Endpoints = serde_yaml::from_str(&endpoints_str)?;
    Ok(endpoints)
}

// Execute command and return result
fn execute_command(endpoint: &EndpointConfig) -> CommandOutput {
    match Command::new(&endpoint.command)
        .args(&endpoint.args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
    {
        Ok(output) => {
            if output.status.success() {
                CommandOutput {
                    status: "success".to_string(),
                    output: String::from_utf8_lossy(&output.stdout).trim().to_string(),
                    error: None,
                }
            } else {
                CommandOutput {
                    status: "error".to_string(),
                    output: String::new(),
                    error: Some(String::from_utf8_lossy(&output.stderr).trim().to_string()),
                }
            }
        }
        Err(e) => CommandOutput {
            status: "error".to_string(),
            output: String::new(),
            error: Some(e.to_string()),
        },
    }
}

fn create_http_response(endpoint: &EndpointConfig) -> HttpResponse {
    let output = execute_command(endpoint);
    let mut content_type = "application/json".to_string();
    let mut headers = vec![];

    if let Some(response_config) = &endpoint.response {
        for (key, value) in &response_config.headers {
            if key.to_lowercase() == "content-type" {
                content_type = value.clone();
            } else {
                headers.push(format!("{}: {}", key, value));
            }
        }
    }

    headers.push(format!("Content-Type: {}", content_type));

    HttpResponse::new(
        200,
        headers,
        serde_json::to_string(&output).unwrap_or_default(),
    )
}

// Handle individual client connection
fn handle_client(mut stream: TcpStream, endpoints: Arc<HashMap<String, EndpointConfig>>) {
    let mut buffer = [0; 1024];

    let response = match stream.read(&mut buffer) {
        Ok(size) if size > 0 => {
            let request = String::from_utf8_lossy(&buffer[..size]);
            let first_line = request.lines().next().unwrap_or("");
            let parts: Vec<&str> = first_line.split_whitespace().collect();

            if parts.len() >= 2 {
                let path = parts[1];

                if path == "/endpoints" {
                    // Special endpoint to list all available endpoints
                    let endpoints_info: HashMap<&String, &String> = endpoints
                        .iter()
                        .map(|(k, v)| (k, &v.description))
                        .collect();

                    HttpResponse::new(
                        200,
                        vec!["Content-Type: application/json".to_string()],
                        serde_json::to_string(&endpoints_info).unwrap_or_default(),
                    )
                } else if let Some(endpoint) = endpoints.get(path) {
                    let response = create_http_response(endpoint);
                    response
                } else {
                    HttpResponse::new(
                        404,
                        vec!["Content-Type: application/json".to_string()],
                        serde_json::json!({
                            "status": "error",
                            "message": "Endpoint not found",
                            "note": "Use /endpoints to see available endpoints"
                        })
                        .to_string(),
                    )
                }
            } else {
                HttpResponse::new(
                    400,
                    vec!["Content-Type: application/json".to_string()],
                    serde_json::json!({
                        "status": "error",
                        "message": "Invalid request"
                    })
                    .to_string(),
                )
            }
        }
        _ => HttpResponse::new(
            400,
            vec!["Content-Type: application/json".to_string()],
            serde_json::json!({
                "status": "error",
                "message": "Invalid request"
            })
            .to_string(),
        ),
    };

    let _ = stream.write_all(response.to_string().as_bytes());
    let _ = stream.flush();
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load configuration
    let config = load_config("config/config.yaml")?;
    let endpoints = Arc::new(load_endpoints("config/endpoints.yaml")?.endpoints);
    //let endpoints = Arc::new(config.endpoints);

    // Create server address
    let addr = format!("{}:{}", config.server.host, config.server.port);
    let listener = TcpListener::bind(&addr)?;
    println!("Server listening on {}", addr);

    // Handle incoming connections
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let endpoints = Arc::clone(&endpoints);
                thread::spawn(move || {
                    handle_client(stream, endpoints);
                });
            }
            Err(e) => eprintln!("Error accepting connection: {}", e),
        }
    }

    Ok(())
}
