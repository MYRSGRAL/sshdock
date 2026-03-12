use std::process::Command;

use crate::error::AppError;

#[derive(Debug, Clone)]
pub struct WifiInfo {
    pub ssid: String,
    pub bssid: Option<String>,
    pub device: Option<String>,
}

pub fn detect_active_wifi() -> Result<Option<WifiInfo>, AppError> {
    let output = run_nmcli(&["-t", "-f", "ACTIVE,SSID,BSSID,DEVICE", "dev", "wifi"])?;

    for raw_line in output.lines().filter(|line| !line.trim().is_empty()) {
        let fields = parse_nmcli_line(raw_line);
        if fields.first().map(|s| s.as_str()) == Some("yes") {
            let ssid = fields.get(1).cloned().unwrap_or_default();
            let bssid = fields.get(2).cloned().filter(|s| !s.is_empty());
            let device = fields.get(3).cloned().filter(|s| !s.is_empty());
            if ssid.is_empty() {
                return Ok(None);
            }
            return Ok(Some(WifiInfo {
                ssid,
                bssid,
                device,
            }));
        }
    }

    Ok(None)
}

fn run_nmcli(args: &[&str]) -> Result<String, AppError> {
    let output = Command::new("nmcli")
        .env("LC_ALL", "C")
        .env("LANG", "C")
        .args(args)
        .output()
        .map_err(|err| AppError::Command(format!("failed to run nmcli: {err}")))?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(AppError::Command(format!(
            "nmcli {:?} failed: {}",
            args,
            stderr.trim()
        )))
    }
}

pub(crate) fn parse_nmcli_line(line: &str) -> Vec<String> {
    let mut fields = Vec::new();
    let mut current = String::new();
    let mut escaped = false;
    for ch in line.chars() {
        if escaped {
            current.push(ch);
            escaped = false;
            continue;
        }
        match ch {
            '\\' => escaped = true,
            ':' => {
                fields.push(std::mem::take(&mut current));
            }
            _ => current.push(ch),
        }
    }
    fields.push(current);
    fields
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_line() {
        let fields = parse_nmcli_line("yes:MyWifi:AA\\:BB\\:CC\\:DD\\:EE\\:FF:wlan0");
        assert_eq!(fields, vec!["yes", "MyWifi", "AA:BB:CC:DD:EE:FF", "wlan0"]);
    }

    #[test]
    fn parse_empty_fields() {
        let fields = parse_nmcli_line("no::::");
        assert_eq!(fields, vec!["no", "", "", "", ""]);
    }

    #[test]
    fn parse_escaped_backslash() {
        let fields = parse_nmcli_line("a\\\\b:c");
        assert_eq!(fields, vec!["a\\b", "c"]);
    }

    #[test]
    fn parse_single_field() {
        let fields = parse_nmcli_line("only");
        assert_eq!(fields, vec!["only"]);
    }
}
