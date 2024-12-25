# Miniwebify

Miniwebify is a lightweight web server that executes local shell commands and returns their output as HTTP responses.

## Build and run

```bash
# Build the project
cargo build --release

# Create endpoints.yaml (use the format shown above)
# Run the server
./target/release/miniwebify
```


## Test endpoints

```bash
# List all available endpoints
curl http://localhost:8080/endpoints

# Test specific endpoint
curl http://localhost:8080/date
```


## Docker

Build the docker image:

```bash
docker build -t miniwebify .
```

Run the docker image:

```bash
docker run --name miniwebify -p 8080:8080 miniwebify
```
