use reqwest::blocking::Client;
use semver::{Version, VersionReq};
use serde_json::Value;
use uuid::Uuid;

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
    pub fn new(url: String, accept_invalid_cerst: Option<bool>) -> Result<Self, String> {
        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "apiinfo.version",
            "params": [],
            "id": 1
        });

        let client = Client::builder()
            .tls_danger_accept_invalid_certs(accept_invalid_cerst.unwrap_or(false))
            .build()
            .map_err(|e| e.to_string())?;

        let v6_4_req = VersionReq::parse(">=6.4").map_err(|e| e.to_string())?;

        let version_str_raw = Self::zabbix_raw_request(&client, &url, body, None, false)?;
        let version_str = version_str_raw.trim_matches('"');

        let current_v = Version::parse(version_str).map_err(|e| e.to_string())?;

        let need_auth_in_body = !(v6_4_req.matches(&current_v));

        Ok(ZabbixInstance {
            id: Uuid::new_v4().to_string(),
            need_auth_in_body: need_auth_in_body,
            token: None,
            request_client: client,
            url: url,
            version: version_str.to_string(),
            need_logout: false,
        })
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn url(&self) -> &str {
        &self.url
    }

    #[allow(dead_code)]
    pub fn set_token(&mut self, token: String) -> Result<&mut Self, String> {
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
            Err(e) => return Err(format!("Invalid token: {}", e)),
        }
    }

    #[allow(dead_code)]
    pub fn login(&mut self, username: String, password: String) -> Result<&mut Self, String> {
        let v5_2 = Version::parse("5.2.0").map_err(|e| e.to_string())?;
        let current_v = Version::parse(&self.version).map_err(|e| e.to_string())?;
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

    pub fn logout(&mut self) -> Result<&mut Self, String> {
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

    #[allow(dead_code)]
    pub fn get_version(&self) -> Result<String, String> {
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

    #[allow(dead_code)]
    pub fn check_version(&self, version_req: &str) -> Result<bool, String> {
        let version_req = VersionReq::parse(version_req).map_err(|e| e.to_string())?;
        let current_v = Version::parse(&self.version).map_err(|e| e.to_string())?;

        Ok(version_req.matches(&current_v))
    }

    #[allow(dead_code)]
    pub fn zabbix_request(&self, method: &str, params: Value) -> Result<String, String> {
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

    #[allow(dead_code)]
    pub fn zabbix_request_string(&self, method: &str, params: &str) -> Result<String, String> {
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

        let payload = serde_json::from_str(&body).map_err(|e| e.to_string())?;

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
    ) -> Result<String, String> {
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

        let response = request_builder
            .json(&payload)
            .send()
            .map_err(|e| e.to_string())?;

        if !response.status().is_success() {
            return Err(format!("HTTP Error: {}", response.status()));
        }

        let text = response.text().map_err(|e| e.to_string())?;

        let json: Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;

        if let Some(error) = json.get("error") {
            if error.is_object() {
                let msg = error
                    .get("message")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown error");
                let data = error.get("data").and_then(|v| v.as_str()).unwrap_or("");
                return Err(format!("Zabbix API Error: {} {}", msg, data));
            }
            return Err(error.to_string());
        }

        if let Some(result) = json.get("result") {
            if let Some(s) = result.as_str() {
                return Ok(s.to_string());
            }
            return Ok(result.to_string());
        }

        Err("Unknown response format".to_string())
    }
}

impl Drop for ZabbixInstance {
    fn drop(&mut self) {
        if self.need_logout {
            self.logout().ok();
        }
    }
}
