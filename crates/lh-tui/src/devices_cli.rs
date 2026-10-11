//! `lh devices`: browsers (and later phones) paired with this machine.

use serde_json::{json, Value};

use crate::client::{Client, Result};

pub const HELP: &str = "lh devices                    paired devices and any waiting to pair\nlh devices pair [minutes]     open pairing for a few minutes (a browser cannot ask before this)
lh devices accept <PIN>       pair the device that is showing this PIN
lh devices remove <id>        unpair a device; it must pair again to come back";

pub async fn run(c: &Client, args: &[String]) -> Result<String> {
    match args.first().map(String::as_str) {
        Some("help" | "--help" | "-h") => Ok(HELP.into()),
        Some("pair") => {
            let m: u64 = args.get(1).and_then(|m| m.parse().ok()).unwrap_or(5);
            let v = c.call("POST", "/v1/devices/open", Some(json!({"minutes": m}))).await?;
            Ok(format!("Pairing is open for {} minutes. On the other device, open the address and press Pair; then type here: lh devices accept <PIN>", v["open_for_minutes"]))
        }
        Some("accept") => {
            let pin = args.get(1).ok_or("give the PIN the device is showing: lh devices accept <PIN>")?;
            let v = c.call("POST", "/v1/devices/accept", Some(json!({"pin": pin}))).await?;
            Ok(format!("Paired {} ({}). It can sign in now.", v["name"].as_str().unwrap_or(""), v["id"].as_str().unwrap_or("")))
        }
        Some("remove") => {
            let id = args.get(1).ok_or("which device? Run `lh devices` to see them.")?;
            c.call("DELETE", &format!("/v1/devices/{id}"), None).await.map(|_| format!("Removed {id}."))
        }
        None => {
            let list = c.call("GET", "/v1/devices", None).await?;
            let pend = c.call("GET", "/v1/devices/pending", None).await?;
            let mut out = format!("This machine's certificate fingerprint:\n  {}\n", pend["fingerprint"].as_str().unwrap_or("(the browser door is off)"));
            let devices: Vec<&Value> = list["devices"].as_array().into_iter().flatten().filter(|d| d["revoked"] == false).collect();
            out.push_str(&if devices.is_empty() { "\nNo paired devices.".to_string() } else { format!("\nPaired:\n{}", devices.iter().map(|d| format!("  {}  {}", d["id"].as_str().unwrap_or(""), d["name"].as_str().unwrap_or(""))).collect::<Vec<_>>().join("\n")) });
            for p in pend["pending"].as_array().into_iter().flatten() {
                out.push_str(&format!("\n\nWaiting to pair: {} (key {}), {}s ago. Type the PIN it shows: lh devices accept <PIN>", p["name"].as_str().unwrap_or(""), p["key_fingerprint"].as_str().unwrap_or("?").chars().take(23).collect::<String>(), p["waiting_secs"]));
            }
            Ok(out)
        }
        Some(o) => Err(format!("unknown command {o}. Run `lh devices help`.")),
    }
}
