use http_request::*;
use serde_json::{Value};
use http_type::RequestError;

pub struct ZabbixInstance {
    url: String,
    user: Option<String>,
    password: Option<String>,
    token: String,
    pub timeout: u64,
}

impl ZabbixInstance {
    pub fn new(url: String, auth_id: String, password: Option<String>, version: Option<String>) -> Self {
        match  version {
            Some(v) => {
                let instance_version =  ZabbixInstance::raw_request(&url, json_value!({
                    "jsonrpc": "2.0",
                    "method": "apiinfo.version",
                    "params": [],
                    "id": 1
                }), 10).unwrap();
                if instance_version != "5.0" && instance_version != "6.0" && instance_version != "6.4" && instance_version != "latest" {
                    panic!("Zabbix version must be 5.0, 6.0, 6.4 or latest");
                }
            },
            None => {}
        }
        ZabbixInstance {
            url,
            user: Some(auth_id),
            password,
            token: String::new(),
            timeout: 10
        }
    }

    fn raw_request (url: &str, payload: JsonValue, timeout: u64) -> Result<String, String> {
        let mut header: HashMapXxHash3_64<&str, &str> = hash_map_xx_hash3_64();

        header.insert("Content-Type", "application/json");

        let mut request_builder = RequestBuilder::new()
            .post(url)
            .json(payload)
            .headers(header)
            .timeout(timeout)
            .redirect()
            .max_redirect_times(8)
            .http1_1_only()
            .buffer(4096)
            .build_sync();

        request_builder
            .send()
            .and_then(|response| {
                let data = response.decode(4096).text().get_body();
                let v: Value = serde_json::from_str(data.as_str()).unwrap();
                match v.get("result") {
                    Some(result) => Ok(result.to_string()),
                    None => Err(RequestError::Request(format!("code: {}; message: {}; data: {}", v.get("code").unwrap_or(&Value::Null), v.get("message").unwrap_or(&Value::Null), v.get("data").unwrap_or(&Value::Null)))),
                }
            })
            .map_err(|e| format!("Error => {}", e))
    }
}

fn main() {

    let mut zi = ZabbixInstance::new("http://127.0.0.1:9589/api_jsonrpc.php".to_string(), "auth_id".to_string(), None, Some("8.0".to_string()));

    // let mut header: HashMapXxHash3_64<&str, &str> = hash_map_xx_hash3_64();

    // header.insert("Content-Type", "application/json");

    // let body: JsonValue = json_value!({
    //     "jsonrpc": "2.0",
    //     "method": "apiinfo.version",
    //     "params": [],
    //     "id": 1
    // });

    // let mut request_builder = RequestBuilder::new()
    //     .post("http://127.0.0.1:9589/api_jsonrpc.php")
    //     .json(body)
    //     .headers(header)
    //     .timeout(10)
    //     .redirect()
    //     .max_redirect_times(8)
    //     .http1_1_only()
    //     .buffer(4096)
    //     .build_sync();
    // request_builder
    //     .send()
    //     .and_then(|response| {
    //         println!("{:?}", response.decode(4096).text());
    //         Ok(())
    //     })
    //     .unwrap_or_else(|e| println!("Error => {}", e));
}