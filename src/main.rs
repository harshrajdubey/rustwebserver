use std::fs;
use std::io::{self, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use std::collections::HashMap;
use std::path::Path;

const PORT: u16 = 8000;
const MAX_CONCURRENT_CONNECTIONS: usize = 4;
const RATE_LIMIT_WINDOW: Duration = Duration::from_secs(60);
const MAX_REQUESTS_PER_WINDOW: usize = 100;

fn main() -> io::Result<()> {
    let listener = TcpListener::bind(format!("0.0.0.0:{}", PORT))?;
    let visitor_count = Arc::new(Mutex::new(0));
    let ip_requests = Arc::new(Mutex::new(HashMap::new()));
    
    println!("Server running on port {}", PORT);
    
    // Create a thread pool with a fixed size
    let mut thread_pool = Vec::with_capacity(MAX_CONCURRENT_CONNECTIONS);
    
    // Accept connections indefinitely
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let visitor_count = Arc::clone(&visitor_count);
                let ip_requests = Arc::clone(&ip_requests);
                
                // Clean up finished threads - FIX APPLIED HERE
                thread_pool.retain(|handle: &thread::JoinHandle<_>| !handle.is_finished());
                
                // If we've reached max concurrent connections, wait for one to finish
                if thread_pool.len() >= MAX_CONCURRENT_CONNECTIONS {
                    if let Some(handle) = thread_pool.pop() {
                        if let Err(e) = handle.join() {
                            eprintln!("Error joining thread: {:?}", e);
                        }
                    }
                }
                
                // Handle the new connection in a thread
                let handle = thread::spawn(move || {
                    if let Err(e) = handle_client(stream, visitor_count, ip_requests) {
                        eprintln!("Error handling client: {}", e);
                    }
                });
                thread_pool.push(handle);
            },
            Err(e) => {
                eprintln!("Error accepting connection: {}", e);
            }
        }
    }

    Ok(())
}

fn handle_client(
    mut stream: TcpStream, 
    visitor_count: Arc<Mutex<u32>>, 
    ip_requests: Arc<Mutex<HashMap<String, Vec<SystemTime>>>>
) -> io::Result<()> {
    // Read the HTTP request
    let mut buffer = [0; 4096]; // Larger buffer for bigger requests
    let bytes_read = match stream.read(&mut buffer) {
        Ok(n) => n,
        Err(e) => {
            eprintln!("Failed to read from stream: {}", e);
            return Err(e);
        }
    };
    
    if bytes_read == 0 {
        return Ok(()); // Empty request, client disconnected
    }
    
    let request_str = String::from_utf8_lossy(&buffer[..bytes_read]);
    
    // Get client IP for rate limiting and logging
    let client_ip = match stream.peer_addr() {
        Ok(addr) => addr.ip().to_string(),
        Err(_) => "unknown".to_string(),
    };
    
    // Parse the request to get the path
    let request_lines: Vec<&str> = request_str.lines().collect();
    if request_lines.is_empty() {
        return Ok(());
    }
    
    let first_line = request_lines[0];
    let parts: Vec<&str> = first_line.split_whitespace().collect();
    
    if parts.len() < 2 {
        return send_error(&mut stream, 400, "Bad Request");
    }
    
    let method = parts[0];
    let path = parts[1];
    
    // Print request to console
    println!("[{}] {} {} from {}", 
        format_timestamp(SystemTime::now()),
        method,
        path,
        client_ip
    );
    
    // Rate limit check
    if !rate_limit(&client_ip, &ip_requests) {
        println!("Rate limit exceeded for {}", client_ip);
        let response = "HTTP/1.1 429 Too Many Requests\r\n\
                       Content-Length: 19\r\n\
                       Content-Type: text/plain\r\n\
                       Access-Control-Allow-Origin: *\r\n\
                       \r\n\
                       Rate limit exceeded";
        stream.write_all(response.as_bytes())?;
        return Ok(());
    }
    
    // Special endpoint for visitor count
    if path == "/visitor-count" {
        // Always increment the counter for now (for testing)
        let count = match visitor_count.lock() {
            Ok(mut guard) => {
                // Always increment for now
                *guard += 1;
                println!("Incrementing visitor count to: {}", *guard);
                *guard // Return the current value
            },
            Err(_) => {
                eprintln!("Visitor count mutex was poisoned");
                return send_error(&mut stream, 500, "Internal Server Error");
            }
        };
        
        let body = format!("{}", count);
        
        let header = format!(
            "HTTP/1.1 200 OK\r\n\
            Content-Length: {}\r\n\
            Content-Type: text/plain\r\n\
            Access-Control-Allow-Origin: *\r\n\
            Access-Control-Allow-Methods: GET, POST, OPTIONS\r\n\
            Access-Control-Allow-Headers: Content-Type\r\n\
            \r\n",
            body.len()
        );
        
        stream.write_all(header.as_bytes())?;
        stream.write_all(body.as_bytes())?;
        return Ok(());
    }
    
    // Handle OPTIONS request for CORS preflight
    if method == "OPTIONS" {
        return send_cors_preflight(&mut stream);
    }
    
    // Only handle GET requests for simplicity
    if method != "GET" {
        return send_error(&mut stream, 405, "Method Not Allowed");
    }
    
    // Determine the requested file path
    let requested_path = if path == "/" {
        "public_html/index.html".to_string()
    } else {
        format!("public_html{}", path)
    };
    
    // Security check to prevent directory traversal
    let path_obj = Path::new(&requested_path);
    if path_obj.components().any(|c| c.as_os_str() == "..") {
        println!("Security: Blocked path with .. component: {}", requested_path);
        return send_not_found(&mut stream);
    }
    
    // Try to read the requested file as binary data
    match fs::read(&requested_path) {
        Ok(contents) => {
            let content_type = get_content_type(&requested_path);
            
            let header = format!(
                "HTTP/1.1 200 OK\r\n\
                Content-Length: {}\r\n\
                Content-Type: {}\r\n\
                Access-Control-Allow-Origin: *\r\n\
                Access-Control-Allow-Methods: GET, POST, OPTIONS\r\n\
                Access-Control-Allow-Headers: Content-Type\r\n\
                \r\n",
                contents.len(),
                content_type
            );
            
            // Log successful request to file and print status to console
            println!("[{}] 200 OK: {}", format_timestamp(SystemTime::now()), requested_path);
            log_request(&client_ip, &format!("{} {} 200", method, path));
            
            // Send response
            stream.write_all(header.as_bytes())?;
            stream.write_all(&contents)?;
        },
        Err(e) => {
            // Log error to console
            println!("[{}] 404 Not Found: {} - {}", format_timestamp(SystemTime::now()), requested_path, e);
            send_not_found(&mut stream)?;
        }
    }
    
    stream.flush()?;
    Ok(())
}

fn send_not_found(stream: &mut TcpStream) -> io::Result<()> {
    // Try to use custom 404 page if available
    let (contents, status_line) = match fs::read("server_assets/404.html") {
        Ok(data) => (data, "HTTP/1.1 404 NOT FOUND"),
        Err(_) => (b"404 Not Found".to_vec(), "HTTP/1.1 404 NOT FOUND"),
    };
    
    let header = format!(
        "{}\r\n\
        Content-Length: {}\r\n\
        Content-Type: text/html\r\n\
        Access-Control-Allow-Origin: *\r\n\
        Access-Control-Allow-Methods: GET, POST, OPTIONS\r\n\
        Access-Control-Allow-Headers: Content-Type\r\n\
        \r\n",
        status_line,
        contents.len()
    );
    
    stream.write_all(header.as_bytes())?;
    stream.write_all(&contents)?;
    stream.flush()?;
    Ok(())
}

fn send_error(stream: &mut TcpStream, code: u16, message: &str) -> io::Result<()> {
    let body = format!("<html><body><h1>{} {}</h1></body></html>", code, message);
    let header = format!(
        "HTTP/1.1 {} {}\r\n\
        Content-Length: {}\r\n\
        Content-Type: text/html\r\n\
        Access-Control-Allow-Origin: *\r\n\
        \r\n",
        code,
        message,
        body.len()
    );
    
    stream.write_all(header.as_bytes())?;
    stream.write_all(body.as_bytes())?;
    stream.flush()?;
    Ok(())
}

fn send_cors_preflight(stream: &mut TcpStream) -> io::Result<()> {
    let response = "HTTP/1.1 204 No Content\r\n\
                   Access-Control-Allow-Origin: *\r\n\
                   Access-Control-Allow-Methods: GET, POST, OPTIONS\r\n\
                   Access-Control-Allow-Headers: Content-Type\r\n\
                   \r\n";
    
    stream.write_all(response.as_bytes())?;
    stream.flush()?;
    Ok(())
}

/// Returns a simple MIME type based on the file extension
fn get_content_type(filename: &str) -> &str {
    if filename.ends_with(".html") {
         "text/html"
    } else if filename.ends_with(".css") {
         "text/css"
    } else if filename.ends_with(".js") {
         "application/javascript"
    } else if filename.ends_with(".png") {
         "image/png"
    } else if filename.ends_with(".jpg") || filename.ends_with(".jpeg") {
         "image/jpeg"
    } else if filename.ends_with(".gif") {
         "image/gif"
    } else if filename.ends_with(".svg") {
         "image/svg+xml"
    } else if filename.ends_with(".ico") {
         "image/x-icon"
    } else {
         "application/octet-stream"
    }
}

fn rate_limit(ip: &str, ip_requests: &Arc<Mutex<HashMap<String, Vec<SystemTime>>>>) -> bool {
    let ip_requests_guard = match ip_requests.lock() {
        Ok(guard) => guard,
        Err(_) => {
            eprintln!("Failed to acquire lock for rate limiting");
            return true; // Default to allowing if lock fails
        }
    };
    
    let mut ip_requests = ip_requests_guard;
    let now = SystemTime::now();
    let requests = ip_requests.entry(ip.to_string()).or_insert_with(Vec::new);
    
    // Clean up old requests
    requests.retain(|&time| {
        match now.duration_since(time) {
            Ok(duration) => duration < RATE_LIMIT_WINDOW,
            Err(_) => false, // Remove if time calculation fails
        }
    });
    
    // Check if rate limit is exceeded
    if requests.len() >= MAX_REQUESTS_PER_WINDOW {
        return false;
    }
    
    // Add current request
    requests.push(now);
    true
}

fn log_request(ip: &str, request: &str) {
    let timestamp = format_timestamp(SystemTime::now());
    let log_entry = format!("[{}] {} - {}\n", timestamp, ip, request);
    
    match fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open("server.log")
    {
        Ok(mut file) => {
            if let Err(e) = file.write_all(log_entry.as_bytes()) {
                eprintln!("Failed to write to log file: {}", e);
            }
        },
        Err(e) => eprintln!("Failed to open log file: {}", e),
    }
}

fn format_timestamp(time: SystemTime) -> String {
    match time.duration_since(UNIX_EPOCH) {
        Ok(duration) => {
            let secs = duration.as_secs();
            let nanos = duration.subsec_nanos();
            format!("{}.{:09}", secs, nanos)
        },
        Err(_) => "unknown_time".to_string(),
    }
}