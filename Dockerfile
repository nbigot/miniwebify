FROM rust:1.67 AS builder
WORKDIR /usr/src/app
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo install --path .

# Build the Rust application
RUN cargo build --release


#FROM alpine:3.21
#RUN apk add --no-cache libgcc
#FROM debian:buster-slim
FROM debian:bullseye-slim
RUN apt-get update && apt-get install -y libgcc1 && rm -rf /var/lib/apt/lists/*

# Set the working directory inside the container
WORKDIR /app


# Copy the current directory contents into the container at /app
COPY README.md ./
COPY config ./config

COPY --from=0 /usr/src/app/target/release/miniwebify .


# Set the startup command to run the binary
CMD ["/app/miniwebify"]
