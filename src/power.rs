use zbus::blocking::{Connection, Proxy};

use crate::error::AppError;

pub fn is_on_ac_power() -> Result<bool, AppError> {
    let conn = Connection::system()?;
    let proxy = Proxy::new(
        &conn,
        "org.freedesktop.UPower",
        "/org/freedesktop/UPower",
        "org.freedesktop.UPower",
    )?;

    let on_battery: bool = proxy.get_property("OnBattery")?;
    Ok(!on_battery)
}
