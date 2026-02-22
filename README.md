# http-request-zabbix

A Rust client library for interacting with the Zabbix API.

## Features

- **Builder Pattern**: Easy to configure the client (e.g., TLS verification).
- **Strongly Typed Authentication**: Enforces valid login states using Rust's type system to ensure requests are authenticated.
- **Robust Error Handling**: Uses `thiserror` to provide fine-grained, matchable errors (`Network`, `Json`, `ApiError`, etc.) rather than generic strings.
- **Flexible Parameters**: Pass parameters to API requests directly as raw JSON strings or `serde_json::Value`.
- **Version Awareness**: Intelligently handles the authentication flow changes introduced in Zabbix >= 6.4 (Bearer tokens instead of body-embedded auth).

## Usage

Add this to your `Cargo.toml`:

```toml
[dependencies]
http-request-zabbix = "0.1.0"
```

### Example

```rust
use http_request_zabbix::{AuthType, ZabbixInstance, ApiRequestParams};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Initialize and connect to the Zabbix server
    let zabbix = ZabbixInstance::builder("http://zabbix.example.com/zabbix")
        .danger_accept_invalid_certs(true)
        .build()?
        // 2. Authenticate
        .login(AuthType::UsernamePassword(
            "Admin".to_string(),
            "zabbix".to_string()
        ))?;

    // 3. Make requests
    let params = ApiRequestParams::from(serde_json::json!({
        "output": ["host", "name"],
        "limit": 5
    }));
    
    let result = zabbix.zabbix_request("host.get", params)?;
    println!("Hosts: {}", result);

    Ok(())
}
```

## Testing

The library uses `mockito` for unit testing the HTTP interactions. Run the tests with:

```bash
cargo test
```

## License

MIT
