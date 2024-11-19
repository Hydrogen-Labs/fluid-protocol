# Build stage
FROM rust:1.79-slim-bullseye AS builder

# Create a new empty shell project
WORKDIR /usr/src/app

# Install wasm target and wasm-pack
RUN rustup target add wasm32-unknown-unknown && \
    curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh

# Copy the entire workspace
COPY . .

# Build the services package specifically
RUN cargo build --release -p services

# Final stage
FROM debian:bullseye-slim

# Install necessary runtime dependencies
RUN apt-get update && \
    apt-get install -y ca-certificates && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /app

ENV NETWORK=mainnet

# Copy the binary from builder
COPY --from=builder /usr/src/app/target/release/services /app/services

# Copy any necessary environment files
COPY .env .env
COPY mainnet.contracts.json mainnet.contracts.json
COPY testnet.contracts.json testnet.contracts.json

# Run the binary
CMD ["./services"]