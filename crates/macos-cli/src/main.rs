use osmo_core::Scanner;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Scanning for DJI cameras (5 seconds)…");
    let scanner = Scanner::new().await?;
    for device in scanner.scan(Duration::from_secs(5)).await? {
        println!("{}  {}  RSSI {:?}", device.id, device.name, device.rssi);
    }
    Ok(())
}
