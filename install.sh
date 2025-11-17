#!/usr/bin/env bash
set -euo pipefail

BIN_URL=${BIN_URL:-"https://github.com/MYRSGRAL/sshdock/releases/download/main/sshdock"}
SERVICE_URL=${SERVICE_URL:-"https://raw.githubusercontent.com/MYRSGRAL/sshdock/main/sshdock.service"}
BIN_PATH=${BIN_PATH:-"/usr/local/bin/sshdock"}
SERVICE_PATH=${SERVICE_PATH:-"/etc/systemd/system/sshdock.service"}
CONFIG_PATH=${CONFIG_PATH:-"/etc/sshdock/config.toml"}

if [[ $(id -u) -ne 0 ]]; then
    echo "[ERROR] Запусти скрипт от root (sudo)." >&2
    exit 1
fi

for tool in curl install systemctl; do
    if ! command -v "$tool" >/dev/null 2>&1; then
        echo "[ERROR] Не найдена утилита '$tool'. Установи её и повтори попытку." >&2
        exit 1
    fi
done

TMP_BIN=$(mktemp)
TMP_SERVICE=$(mktemp)

cleanup() {
    rm -f "$TMP_BIN" "$TMP_SERVICE"
}
trap cleanup EXIT

printf '[INFO] Скачиваю бинарник…\n'
curl -fsSL "$BIN_URL" -o "$TMP_BIN"
chmod +x "$TMP_BIN"
install -Dm755 "$TMP_BIN" "$BIN_PATH"
printf '[INFO] Бинарь установлен в %s\n' "$BIN_PATH"

printf '[INFO] Скачиваю systemd-юнит…\n'
curl -fsSL "$SERVICE_URL" -o "$TMP_SERVICE"
install -Dm644 "$TMP_SERVICE" "$SERVICE_PATH"
printf '[INFO] Unit установлен в %s\n' "$SERVICE_PATH"

mkdir -p "$(dirname "$CONFIG_PATH")"
if [[ ! -f $CONFIG_PATH ]]; then
    cat <<'CFG' > "$CONFIG_PATH"
# sshdock configuration (редактируй под свои сети)
# Проверка Wi-Fi каждые 5 секунд
poll_interval_secs = 5

# Какой unit systemd запускать при совпадении сети
ssh_service = "sshd.service"

[[networks]]
# Профиль включается только когда ноут подключён к питанию
name = "Домашняя док-станция"
ssid = "MyWifi"                 # SSID сети (обязательно)
bssid = "AA:BB:CC:DD:EE:FF"     # MAC точки доступа (опционально)
interface = "wlan0"             # Интерфейс Wi-Fi (опционально)
enable_ssh = true                # Стартовать sshd
stop_ssh_on_disconnect = true    # Останавливать sshd при разрыве
prevent_lid_sleep = true         # Блокировать сон при закрытии крышки
prevent_idle_sleep = true        # Блокировать автоматический suspend (экран может гаснуть)
require_ac_power = true          # Работать только при подключённом питании

# Измени ssid/bssid/interface и остальное под свои условия. Можно добавить дополнительные [[networks]].
CFG
    printf '[INFO] Создан шаблон конфигурации в %s\n' "$CONFIG_PATH"
else
    printf '[INFO] Конфигурация %s уже существует — отредактируй её вручную.\n' "$CONFIG_PATH"
fi

printf '[INFO] Не забудь отредактировать %s перед запуском!\n' "$CONFIG_PATH"

systemctl daemon-reload
printf '[INFO] Перезапустил systemd, включи сервис вручную: sudo systemctl enable --now sshdock.service\n'
