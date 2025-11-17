use log::{error, info};

use crate::config::{Config, NetworkConfig};
use crate::error::AppError;
use crate::system::{ensure_service_started, release_ssh_service, SleepInhibitor, StateChange};

#[derive(Debug, Default)]
pub struct AppliedState {
    active: Option<ActiveContext>,
}

impl AppliedState {
    pub fn apply_profile(
        &mut self,
        idx: usize,
        profile: &NetworkConfig,
        config: &Config,
    ) -> Result<(), AppError> {
        let already_active = self
            .active
            .as_ref()
            .map(|ctx| ctx.profile_idx == idx)
            .unwrap_or(false);
        if already_active {
            return Ok(());
        }

        self.clear()?;
        info!("profile '{}' matched", profile.display_name());

        let ssh_handle = if profile.enable_ssh() {
            let service = profile.ssh_service(config).to_owned();
            match ensure_service_started(&service) {
                Ok(StateChange::Changed) => {
                    info!("started {service}");
                    Some(SshHandle {
                        service_name: service,
                        stop_on_disconnect: profile.stop_ssh_on_disconnect(),
                    })
                }
                Ok(StateChange::AlreadyActive) => {
                    info!("{service} already active");
                    Some(SshHandle {
                        service_name: service,
                        stop_on_disconnect: false,
                    })
                }
                Err(err) => {
                    error!("failed to start {}: {err}", service);
                    None
                }
            }
        } else {
            None
        };

        let inhibitor = if let Some(what) = profile.inhibitor_targets() {
            match SleepInhibitor::acquire(&what, profile.display_name()) {
                Ok(handle) => Some(handle),
                Err(err) => {
                    error!("failed to acquire inhibitor: {err}");
                    None
                }
            }
        } else {
            None
        };

        self.active = Some(ActiveContext {
            profile_idx: idx,
            profile_name: profile.display_name().to_string(),
            ssh: ssh_handle,
            inhibitor,
        });

        Ok(())
    }

    pub fn clear(&mut self) -> Result<(), AppError> {
        if let Some(ctx) = self.active.take() {
            ctx.release();
        }
        Ok(())
    }
}

#[derive(Debug)]
struct ActiveContext {
    profile_idx: usize,
    profile_name: String,
    ssh: Option<SshHandle>,
    inhibitor: Option<SleepInhibitor>,
}

impl ActiveContext {
    fn release(self) {
        info!("leaving profile '{}'", self.profile_name);
        if let Some(ssh) = self.ssh {
            ssh.release();
        }
        drop(self.inhibitor);
    }
}

#[derive(Debug)]
struct SshHandle {
    service_name: String,
    stop_on_disconnect: bool,
}

impl SshHandle {
    fn release(self) {
        release_ssh_service(&self.service_name, self.stop_on_disconnect);
    }
}
