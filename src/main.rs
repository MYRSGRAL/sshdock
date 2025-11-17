mod config;
mod error;
mod power;
mod state;
mod system;
mod wifi;

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;

use log::{info, warn};
use signal_hook::consts::TERM_SIGNALS;
use signal_hook::flag::{register, register_conditional_shutdown};

use crate::config::Config;
use crate::error::AppError;
use crate::power::is_on_ac_power;
use crate::state::AppliedState;
use crate::wifi::detect_active_wifi;

fn main() {
    if let Err(err) = run() {
        eprintln!("{err}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), AppError> {
    env_logger::init();

    let shutdown = Arc::new(AtomicBool::new(false));
    for sig in TERM_SIGNALS {
        register_conditional_shutdown(*sig, 1, shutdown.clone())?;
        register(*sig, shutdown.clone())?;
    }

    let config_path = std::env::var("SSHFDOCK_CONFIG").ok().map(PathBuf::from);
    let config = Config::load(config_path.as_deref())?;
    if config.networks().is_empty() {
        return Err(AppError::Config(
            "configuration must declare at least one network profile".into(),
        ));
    }

    let poll_interval = config.poll_interval();
    info!(
        "watching {} wifi profile(s) with polling interval {:?}",
        config.networks().len(),
        poll_interval
    );

    let mut state = AppliedState::default();

    while !shutdown.load(Ordering::Relaxed) {
        let ac_online = match is_on_ac_power() {
            Ok(state) => state,
            Err(err) => {
                warn!("unable to query AC power state: {err}");
                false
            }
        };

        match detect_active_wifi() {
            Ok(Some(wifi)) => {
                if let Some((idx, profile)) = config
                    .networks()
                    .iter()
                    .enumerate()
                    .find(|(_, prof)| prof.matches(&wifi))
                {
                    if profile.requires_ac_power() && !ac_online {
                        info!(
                            "skipping profile '{}' because charger is not connected",
                            profile.display_name()
                        );
                        state.clear()?;
                        continue;
                    }
                    state.apply_profile(idx, profile, &config)?;
                } else {
                    state.clear()?;
                }
            }
            Ok(None) => state.clear()?,
            Err(err) => warn!("unable to query Wi-Fi state: {err}"),
        }

        thread::sleep(poll_interval);
    }

    state.clear()?;
    info!("shutdown requested, exiting");
    Ok(())
}
