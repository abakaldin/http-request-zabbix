//! A Rust library for interacting with the Zabbix API.
//!
//! This crate provides a convenient and idiomatic way to communicate with a Zabbix server,
//! handling authentication, version checking, and raw API requests.
//!
//! # Example
//!
//! ```no_run
//! use http_request_zabbix::ZabbixInstance;
//!
//! let mut zabbix = ZabbixInstance::builder("http://zabbix.example.com/zabbix/api_jsonrpc.php")
//!     .danger_accept_invalid_certs(true)
//!     .build()
//!     .unwrap();
//!
//! zabbix.login("Admin".to_string(), "zabbix".to_string()).unwrap();
//! println!("Zabbix Version: {}", zabbix.get_version().unwrap());
//! ```

use reqwest::blocking::Client;
use semver::{Version, VersionReq};
use serde_json::Value;
use thiserror::Error;
use uuid::Uuid;

/// Error type for Zabbix interactions.
#[derive(Error, Debug)]
pub enum ZabbixError {
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Version parse error: {0}")]
    VersionParse(#[from] semver::Error),
    #[error("Zabbix API Error: {message} {data}")]
    ApiError { message: String, data: String },
    #[error("Unknown error: {0}")]
    Other(String),
}

/// Represents an active connection to a Zabbix server.
pub struct ZabbixInstance {
    id: String,
    url: String,
    token: Option<String>,
    request_client: Client,
    need_auth_in_body: bool,
    version: String,
    need_logout: bool,
}

impl ZabbixInstance {
    /// Returns a builder to configure and create a `ZabbixInstance`.
    pub fn builder(url: &str) -> ZabbixInstanceBuilder {
        ZabbixInstanceBuilder::new(url)
    }
}

/// A builder for creating a `ZabbixInstance`.
pub struct ZabbixInstanceBuilder {
    url: String,
    accept_invalid_certs: bool,
}

impl ZabbixInstanceBuilder {
    /// Creates a new builder with the given Zabbix URL.
    pub fn new(url: &str) -> Self {
        Self {
            url: url.to_string(),
            accept_invalid_certs: false,
        }
    }

    /// Configures whether the client should verify the server's TLS certificates.
    ///
    /// Setting this to `true` is dangerous and should only be used for testing
    /// or when using self-signed certificates in a trusted environment.
    pub fn danger_accept_invalid_certs(mut self, accept: bool) -> Self {
        self.accept_invalid_certs = accept;
        self
    }

    /// Builds the `ZabbixInstance` by connecting to the server and verifying the API version.
    pub fn build(self) -> Result<ZabbixInstance, ZabbixError> {
        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "apiinfo.version",
            "params": [],
            "id": 1
        });

        let client = Client::builder()
            .danger_accept_invalid_certs(self.accept_invalid_certs)
            .build()?;

        let v6_4_req = VersionReq::parse(">=6.4")?;

        let version_str_raw =
            ZabbixInstance::zabbix_raw_request(&client, &self.url, body, None, false)?;
        let version_str = version_str_raw.trim_matches('"');

        let current_v = Version::parse(version_str)?;

        let need_auth_in_body = !(v6_4_req.matches(&current_v));

        Ok(ZabbixInstance {
            id: Uuid::new_v4().to_string(),
            need_auth_in_body,
            token: None,
            request_client: client,
            url: self.url,
            version: version_str.to_string(),
            need_logout: false,
        })
    }
}

impl ZabbixInstance {
    /// Returns the internally generated UUID for this instance.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Returns the Zabbix API URL this instance connects to.
    pub fn url(&self) -> &str {
        &self.url
    }

    /// Sets an existing authentication token and verifies it with the server.
    pub fn set_token(&mut self, token: String) -> Result<&mut Self, ZabbixError> {
        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "user.checkAuthentication",
            "params": {
                "token": token
            },
            "id": 1
        });

        match Self::zabbix_raw_request(
            &self.request_client,
            &self.url,
            body,
            None,
            self.need_auth_in_body,
        ) {
            Ok(_) => {
                self.token = Some(token);
                Ok(self)
            }
            Err(e) => return Err(ZabbixError::Other(format!("Invalid token: {}", e))),
        }
    }

    /// Logs in to the Zabbix server with a username and password.
    pub fn login(&mut self, username: String, password: String) -> Result<&mut Self, ZabbixError> {
        let v5_2 = Version::parse("5.2.0")?;
        let current_v = Version::parse(&self.version)?;
        let user_param = if current_v <= v5_2 {
            "user"
        } else {
            "username"
        };
        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "user.login",
            "params": {
                user_param: username,
                "password": password
            },
            "id": 1
        });

        match Self::zabbix_raw_request(
            &self.request_client,
            &self.url,
            body,
            None,
            self.need_auth_in_body,
        ) {
            Ok(token) => {
                self.token = Some(token.trim_matches('"').to_string());
                self.need_logout = true;
                Ok(self)
            }
            Err(e) => Err(e),
        }
    }

    /// Logs out of the Zabbix server and invalidates the current token.
    pub fn logout(&mut self) -> Result<&mut Self, ZabbixError> {
        if !self.need_logout {
            return Ok(self);
        }

        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "user.logout",
            "params": [],
            "id": 1
        });

        match Self::zabbix_raw_request(
            &self.request_client,
            &self.url,
            body,
            self.token.as_ref(),
            self.need_auth_in_body,
        ) {
            Ok(_) => {
                self.token = None;
                self.need_logout = false;
                Ok(self)
            }
            Err(e) => Err(e),
        }
    }

    /// Retrieves the Zabbix server API version.
    pub fn get_version(&self) -> Result<String, ZabbixError> {
        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "apiinfo.version",
            "params": [],
            "id": 1
        });

        let version_str =
            Self::zabbix_raw_request(&self.request_client, &self.url, body, None, false)?;

        Ok(version_str)
    }

    /// Checks if the connected Zabbix server's version matches a semantic version requirement.
    /// Example requirement: `>=6.4, <7.0`
    pub fn check_version(&self, version_req: &str) -> Result<bool, ZabbixError> {
        let version_req = VersionReq::parse(version_req)?;
        let current_v = Version::parse(&self.version)?;

        Ok(version_req.matches(&current_v))
    }

    /// Makes a raw JSON-RPC request to the Zabbix API using a `serde_json::Value` parameter.
    pub fn zabbix_request(&self, method: &str, params: Value) -> Result<String, ZabbixError> {
        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params,
            "id": Uuid::new_v4().to_string()
        });

        Self::zabbix_raw_request(
            &self.request_client,
            &self.url,
            body,
            self.token.as_ref(),
            self.need_auth_in_body,
        )
    }

    /// Makes a raw JSON-RPC request to the Zabbix API using a JSON string parameter.
    pub fn zabbix_request_string(&self, method: &str, params: &str) -> Result<String, ZabbixError> {
        let body = format!(
            r#"{{
                "jsonrpc": "2.0",
                "method": "{}",
                "params": {},
                "id": "{}"
            }}"#,
            method,
            params,
            Uuid::new_v4().to_string()
        );

        let payload = serde_json::from_str(&body)?;

        Self::zabbix_raw_request(
            &self.request_client,
            &self.url,
            payload,
            self.token.as_ref(),
            self.need_auth_in_body,
        )
    }

    fn zabbix_raw_request(
        client: &Client,
        url: &str,
        mut payload: Value,
        token: Option<&String>,
        need_auth_in_body: bool,
    ) -> Result<String, ZabbixError> {
        let mut request_builder = client
            .post(url)
            .header("Content-Type", "application/json-rpc");

        if let Some(tok) = token {
            if !need_auth_in_body {
                request_builder =
                    request_builder.header("Authorization", format!("Bearer {}", tok));
            } else {
                if let Some(obj) = payload.as_object_mut() {
                    obj.insert("auth".to_string(), Value::String(tok.to_string()));
                }
            }
        }

        let response = request_builder.json(&payload).send()?;

        if !response.status().is_success() {
            return Err(ZabbixError::Other(format!(
                "HTTP Error: {}",
                response.status()
            )));
        }

        let text = response.text()?;

        let json: Value = serde_json::from_str(&text)?;

        if let Some(error) = json.get("error") {
            if error.is_object() {
                let msg = error
                    .get("message")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown error");
                let data = error.get("data").and_then(|v| v.as_str()).unwrap_or("");
                return Err(ZabbixError::ApiError {
                    message: msg.to_string(),
                    data: data.to_string(),
                });
            }
            return Err(ZabbixError::Other(error.to_string()));
        }

        if let Some(result) = json.get("result") {
            if let Some(s) = result.as_str() {
                return Ok(s.to_string());
            }
            return Ok(result.to_string());
        }

        Err(ZabbixError::Other("Unknown response format".to_string()))
    }
}

impl Drop for ZabbixInstance {
    fn drop(&mut self) {
        if self.need_logout {
            self.logout().ok();
        }
    }
}
