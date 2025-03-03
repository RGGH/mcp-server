

```markdown
# MCPServer - A Simple TCP Server for Model Management

This project implements a basic TCP server written in Rust using `tokio` and `reqwest` to handle Model Creation Protocol (MCP) requests. The server listens for incoming connections and processes JSON-RPC requests to interact with registered models, create sessions, generate responses, and close sessions.

## Features

- **Model Registration**: Register custom model handlers.
- **Session Management**: Create, generate, and close sessions for each model.
- **HTTP Parsing**: Handle POST requests with a basic HTTP parser.
- **Error Handling**: Return detailed error responses when something goes wrong.
- **Flight Data Fetching**: Fetch and filter nearby flight data from the OpenSky Network API.

## Getting Started

### Prerequisites

- Rust (latest stable version)
- Cargo (Rust package manager)
- `tokio` for async processing
- `reqwest` for HTTP requests

### Installation

1. Clone the repository:

   ```bash
   git clone https://github.com/your-username/mcp-server.git
   cd mcp-server
   ```

2. Install dependencies:

   ```bash
   cargo build
   ```

3. Run the server:

   ```bash
   cargo run
   ```

   The server will start listening on `127.0.0.1:8080`.

### Structure

- `MCPServer`: The main server struct that manages models and sessions.
- `MCPRequest`: Struct representing an incoming JSON-RPC request.
- `MCPResponse`: Struct representing the response to a JSON-RPC request.
- `MCPError`: Struct representing an error response.
- `Session`: Struct that stores session-related data.
- `ModelHandler`: Type alias for functions that handle model prompts.

### Example Request

To interact with the server, send a `POST` request with a JSON payload like this:

```json
{
  "id": "1",
  "method": "session.create",
  "params": {
    "model": "example-model"
  }
}
```

### Example Response

The server will respond with something like this:

```json
{
  "id": "1",
  "result": {
    "session_id": "uuid-string"
  },
  "error": null
}
```

### Supported Methods

- **session.create**: Create a new session for a model.
- **session.generate**: Generate a response from a model based on the session.
- **session.close**: Close an existing session.
- **models.list**: List all registered models.

### Flight Data Fetching

You can fetch nearby flight data by calling the `fetch_flight_data` function, which pulls data from the [OpenSky Network](https://opensky-network.org/api/states/all) API and filters flights within a 1-degree radius.

Example of using `fetch_flight_data`:

```rust
let lon = -0.1278;  // Longitude of London
let lat = 51.5074;  // Latitude of London
let flight_data = fetch_flight_data(lon, lat).await?;
```

This will return nearby flights in JSON format.

## Testing

You can test the server by sending JSON-RPC requests via `curl` or using any HTTP client (e.g., Postman). Here's an example using `curl`:

```bash
curl -X POST http://127.0.0.1:8080 -d '{"id": "1", "method": "session.create", "params": {"model": "example-model"}}'
```

## Example Model Handler

An example model handler function is provided. It simply generates a response based on the conversation history (context) and the incoming prompt:

```rust
fn example_model_handler(prompt: &str, context: &[String]) -> Result<String, Box<dyn Error + Send + Sync>> {
    let context_len = context.len() / 2;
    Ok(format!("Response to: {}. This is turn #{} in the conversation.", prompt, context_len + 1))
}
```

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
```

### How to Use This README

- Replace `https://github.com/your-username/mcp-server.git` with the actual URL of your repository.
- Add any additional sections as needed, such as contributing guidelines or a changelog, depending on how you'd like to structure the repository.

This README provides the necessary instructions for setting up and running the MCPServer project, along with examples for usage and testing!
