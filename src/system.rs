use std::ffi::{OsStr, OsString};
use std::process::Command;

use log::{info, warn};
use zbus::blocking::{Connection, Proxy};
use zbus::zvariant::OwnedFd;

use crate::error::AppError;

#[derive(Debug)]
pub struct SleepInhibitor {
    _fd: OwnedFd,
}

impl SleepInhibitor {
    pub fn acquire(what: &str, profile_name: &str) -> Result<Self, AppError> {
        let conn = Connection::system()?;
        let proxy = Proxy::new(
            &conn,
            "org.freedesktop.login1",
            "/org/freedesktop/login1",
            "org.freedesktop.login1.Manager",
        )?;

        let reason = format!("active profile '{}'", profile_name);
        let fd: OwnedFd = proxy.call("Inhibit", &(&what, "sshdock", reason.as_str(), "block"))?;
        info!("acquired inhibitors for {what}");
        Ok(Self { _fd: fd })
    }
}

#[derive(Debug, Clone, Copy)]
pub enum StateChange {
    Changed,
    AlreadyActive,
}

pub fn ensure_service_started(service: &str) -> Result<StateChange, AppError> {
    if is_service_active(service)? {
        return Ok(StateChange::AlreadyActive);
    }
    run_command_status("systemctl", ["start", service])?;
    Ok(StateChange::Changed)
}

pub fn stop_service(service: &str) -> Result<StateChange, AppError> {
    if !is_service_active(service)? {
        return Ok(StateChange::AlreadyActive);
    }
    run_command_status("systemctl", ["stop", service])?;
    Ok(StateChange::Changed)
}

pub fn release_ssh_service(service: &str, stop_on_disconnect: bool) {
    if !stop_on_disconnect {
        return;
    }
    match stop_service(service) {
        Ok(StateChange::Changed) => info!("stopped {service}"),
        Ok(StateChange::AlreadyActive) => {}
        Err(err) => warn!("failed to stop {service}: {err}"),
    }
}

fn is_service_active(service: &str) -> Result<bool, AppError> {
    let status = Command::new("systemctl")
        .arg("is-active")
        .arg("--quiet")
        .arg(service)
        .status()
        .map_err(|err| AppError::Command(format!("failed to run systemctl: {err}")))?;
    Ok(status.success())
}

fn run_command_status<I, S>(cmd: &str, args: I) -> Result<(), AppError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let collected: Vec<OsString> = args
        .into_iter()
        .map(|arg| arg.as_ref().to_os_string())
        .collect();
    let status = Command::new(cmd)
        .args(&collected)
        .status()
        .map_err(|err| AppError::Command(format!("failed to run {cmd}: {err}")))?;
    if status.success() {
        Ok(())
    } else {
        Err(AppError::Command(format!(
            "{cmd} {:?} exited with status {status}",
            format_args_list(&collected)
        )))
    }
}

fn format_args_list(args: &[OsString]) -> Vec<String> {
    args.iter()
        .map(|arg| arg.to_string_lossy().into_owned())
        .collect()
}
