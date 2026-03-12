use std::path::{Path, PathBuf};
use std::time::Duration;

use dirs::home_dir;
use serde::Deserialize;

use crate::error::AppError;
use crate::wifi::WifiInfo;

#[derive(Debug, Deserialize)]
pub struct Config {
    #[serde(default = "default_poll_interval")]
    poll_interval_secs: u64,
    #[serde(default = "default_ssh_service")]
    ssh_service: String,
    #[serde(default)]
    networks: Vec<NetworkConfig>,
}

impl Config {
    pub fn load(path: Option<&Path>) -> Result<Self, AppError> {
        let path = match path {
            Some(p) => p.to_path_buf(),
            None => default_config_path().ok_or_else(|| {
                AppError::Config("unable to determine default config path".into())
            })?,
        };

        let contents = std::fs::read_to_string(&path)
            .map_err(|err| AppError::Config(format!("failed to read {}: {err}", path.display())))?;
        let mut config: Config = toml::from_str(&contents).map_err(|err| {
            AppError::Config(format!("failed to parse {}: {err}", path.display()))
        })?;

        if config.poll_interval_secs == 0 {
            config.poll_interval_secs = default_poll_interval();
        }

        Ok(config)
    }

    pub fn poll_interval(&self) -> Duration {
        Duration::from_secs(self.poll_interval_secs)
    }

    pub fn ssh_service(&self) -> &str {
        &self.ssh_service
    }

    pub fn networks(&self) -> &[NetworkConfig] {
        &self.networks
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct NetworkConfig {
    ssid: String,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    bssid: Option<String>,
    #[serde(default)]
    interface: Option<String>,
    #[serde(default = "default_true")]
    enable_ssh: bool,
    #[serde(default = "default_true")]
    stop_ssh_on_disconnect: bool,
    #[serde(default = "default_true")]
    prevent_lid_sleep: bool,
    #[serde(default = "default_true")]
    prevent_idle_sleep: bool,
    #[serde(default)]
    ssh_service: Option<String>,
    #[serde(default = "default_true")]
    require_ac_power: bool,
}

fn default_true() -> bool {
    true
}

impl NetworkConfig {
    pub fn display_name(&self) -> &str {
        self.name.as_deref().unwrap_or(&self.ssid)
    }

    pub fn matches(&self, wifi: &WifiInfo) -> bool {
        if self.ssid != wifi.ssid {
            return false;
        }
        if let Some(expected) = &self.interface {
            if let Some(actual) = &wifi.device {
                if expected != actual {
                    return false;
                }
            } else {
                return false;
            }
        }
        if let Some(expected) = &self.bssid {
            match &wifi.bssid {
                Some(actual) => {
                    if !expected.eq_ignore_ascii_case(actual) {
                        return false;
                    }
                }
                None => return false,
            }
        }
        true
    }

    pub fn ssh_service<'a>(&'a self, config: &'a Config) -> &'a str {
        self.ssh_service
            .as_deref()
            .unwrap_or_else(|| config.ssh_service())
    }

    pub fn inhibitor_targets(&self) -> Option<String> {
        let mut parts = Vec::new();
        if self.prevent_lid_sleep {
            parts.push("handle-lid-switch");
        }
        if self.prevent_idle_sleep {
            parts.push("sleep");
        }
        if parts.is_empty() {
            None
        } else {
            parts.sort();
            parts.dedup();
            Some(parts.join(":"))
        }
    }

    pub fn enable_ssh(&self) -> bool {
        self.enable_ssh
    }

    pub fn stop_ssh_on_disconnect(&self) -> bool {
        self.stop_ssh_on_disconnect
    }

    pub fn requires_ac_power(&self) -> bool {
        self.require_ac_power
    }
}

fn default_config_path() -> Option<PathBuf> {
    home_dir().map(|mut path| {
        path.push(".config/sshdock/config.toml");
        path
    })
}

fn default_poll_interval() -> u64 {
    5
}

fn default_ssh_service() -> String {
    "sshd.service".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn network(toml_str: &str) -> NetworkConfig {
        toml::from_str(toml_str).unwrap()
    }

    fn wifi(ssid: &str, bssid: Option<&str>, device: Option<&str>) -> WifiInfo {
        WifiInfo {
            ssid: ssid.to_string(),
            bssid: bssid.map(|s| s.to_string()),
            device: device.map(|s| s.to_string()),
        }
    }

    #[test]
    fn matches_ssid_only() {
        let net = network(r#"ssid = "Home""#);
        assert!(net.matches(&wifi("Home", None, None)));
        assert!(!net.matches(&wifi("Other", None, None)));
    }

    #[test]
    fn matches_bssid_case_insensitive() {
        let net = network(
            r#"
            ssid = "Home"
            bssid = "AA:BB:CC:DD:EE:FF"
            "#,
        );
        assert!(net.matches(&wifi("Home", Some("aa:bb:cc:dd:ee:ff"), None)));
        assert!(!net.matches(&wifi("Home", Some("11:22:33:44:55:66"), None)));
        assert!(!net.matches(&wifi("Home", None, None)));
    }

    #[test]
    fn matches_interface() {
        let net = network(
            r#"
            ssid = "Home"
            interface = "wlan0"
            "#,
        );
        assert!(net.matches(&wifi("Home", None, Some("wlan0"))));
        assert!(!net.matches(&wifi("Home", None, Some("wlan1"))));
        assert!(!net.matches(&wifi("Home", None, None)));
    }

    #[test]
    fn inhibitor_targets_both() {
        let net = network(
            r#"
            ssid = "X"
            prevent_lid_sleep = true
            prevent_idle_sleep = true
            "#,
        );
        assert_eq!(
            net.inhibitor_targets(),
            Some("handle-lid-switch:sleep".to_string())
        );
    }

    #[test]
    fn inhibitor_targets_none() {
        let net = network(
            r#"
            ssid = "X"
            prevent_lid_sleep = false
            prevent_idle_sleep = false
            "#,
        );
        assert_eq!(net.inhibitor_targets(), None);
    }

    #[test]
    fn inhibitor_targets_lid_only() {
        let net = network(
            r#"
            ssid = "X"
            prevent_lid_sleep = true
            prevent_idle_sleep = false
            "#,
        );
        assert_eq!(
            net.inhibitor_targets(),
            Some("handle-lid-switch".to_string())
        );
    }

    #[test]
    fn inhibitor_targets_sleep_only() {
        let net = network(
            r#"
            ssid = "X"
            prevent_lid_sleep = false
            prevent_idle_sleep = true
            "#,
        );
        assert_eq!(net.inhibitor_targets(), Some("sleep".to_string()));
    }

    #[test]
    fn display_name_uses_name_field() {
        let net = network(
            r#"
            ssid = "CorpWifi"
            name = "Офис"
            "#,
        );
        assert_eq!(net.display_name(), "Офис");
    }

    #[test]
    fn display_name_falls_back_to_ssid() {
        let net = network(r#"ssid = "CorpWifi""#);
        assert_eq!(net.display_name(), "CorpWifi");
    }
}
