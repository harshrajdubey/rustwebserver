# Rust Web Server

A lightweight HTTP server written in Rust that serves static files with modern features including rate limiting, visitor counting, and CORS support.

## Features

- **Static File Serving**: Serves HTML, CSS, JavaScript, images and other files from the `public_html` directory
- **Visitor Counter**: Tracks and displays the number of visitors to your site
- **Rate Limiting**: Protects against abuse by limiting each IP to 100 requests per minute
- **Multi-threading**: Handles multiple concurrent connections efficiently
- **CORS Support**: Built-in support for cross-origin requests
- **Security**: Basic protection against directory traversal attacks
- **Logging**: Detailed request logging for monitoring and debugging
- **Custom Error Pages**: Customizable 404 and error pages
- **Docker Support**: Easy containerization for deployment

## Installation

### Prerequisites

- Rust (1.53.0 or newer)
- Cargo (included with Rust)

### Setup

1. Clone this repository or download the source code
2. Navigate to the project directory

```bash
# Build the project
cargo build --release

# Run the server
cargo run --release
```

### Directory Structure
```bash
rustwebserver/
├── public_html/            # Web content directory
│   ├── index.html          # Default landing page
│   ├── styles.css          # Other files
│   ├── script.js           # Other files
│   └── ...                 # All other files or folders
├── server_assets/          # Server-specific assets
│   └── 404.html            # Custom 404 error page
│   └── 500.html            # Custom 5xx error page
├── src/                    # Source code
|   └── main.rs             # Main server file
└── Dockerfile              # Docker configuration
```
#### Adding Content
Place your HTML, CSS, JavaScript and other static files in the public_html directory
The server will automatically serve them when requested

#### Visitor Counter
The server includes a built-in visitor counter. To display it on your page.

### Technical Details
#### HTTP Request Handling
The server handles HTTP requests by:
Parsing the request to extract method, path and headers
Checking rate limits based on client IP
Routing to special endpoints or serving requested files
Generating appropriate HTTP responses
#### Thread Management
Uses a thread pool with configurable size (default: 4 threads...mentioned at the top of main.rs file to be changed as required)
Each incoming connection is handled in a separate thread
Expired connections are automatically cleaned up
#### Security Features
Rate limiting
Path sanitization to prevent directory traversal
HTTP Request validation
#### Configuration Options
```bash
const PORT: u16 = 8000;                                   // HTTP port
const MAX_CONCURRENT_CONNECTIONS: usize = 4;              // Thread pool size
const RATE_LIMIT_WINDOW: Duration = Duration::from_secs(60); // Rate limit time window
const MAX_REQUESTS_PER_WINDOW: usize = 100;               // Max requests per window
```


#### Docker Deployment
```bash
# Build the Docker image
docker build -t rustwebserver .

# Run the container
docker run -p 8000:8000 rustwebserver
```

