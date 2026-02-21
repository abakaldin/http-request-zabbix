use http_request_zabbix::ZabbixInstance;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let zabbix_login = ZabbixInstance::builder("http://zabbix.hetzner.lan/zabbix/api_jsonrpc.php")
        .danger_accept_invalid_certs(true)
        .build()?
        .login_with_username_password("Admin".to_string(), "zabbix".to_string())?;

    println!("Zabbix Version: {}", zabbix_login.get_version()?);

    match zabbix_login.zabbix_request(
        "host.get",
        serde_json::json!({"output": ["host", "name"], "limit": 1}),
    ) {
        Ok(res) => println!("Host get (json!): {}", res),
        Err(e) => println!("Host get failed: {}", e),
    }

    Ok(())
}
