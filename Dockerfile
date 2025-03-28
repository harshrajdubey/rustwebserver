# Use a single Rust image
FROM rust:slim

# Set the working directory
WORKDIR /app

# Copy project files
COPY Cargo.toml Cargo.lock* ./
COPY src ./src

# Build the application
RUN cargo build --release

# Copy static files
COPY public_html ./public_html
COPY server_assets ./server_assets

# Expose the web server port
EXPOSE 8000

# Run the binary
CMD ["./target/release/rustwebserver"]