use http_request_zabbix::{AuthType, ZabbixInstance};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let zabbix_login = ZabbixInstance::builder("http://zabbix.hetzner.lan/zabbix")
        .danger_accept_invalid_certs(true)
        .build()?
        .login(AuthType::UsernamePassword(
            "Admin".to_string(),
            "zabbix".to_string(),
        ))?;

    println!("Zabbix Version: {}", zabbix_login.get_version()?);

    match zabbix_login.zabbix_request(
        "host.get",
        r#"{
            "output": ["host", "name"],
            "limit": 1
        }"#,
    ) {
        Ok(res) => println!("Host get (string): {}", res),
        Err(e) => println!("Host get failed: {}", e),
    }

    Ok(())
}
