# sshdock

Сервис реагирует на подключение к заранее описанным Wi‑Fi сетям: запускает (или останавливает) `sshd`, запрещает/разрешает перевод ноутбука в сон и учитывает, подключено ли питание. Всё это делается через systemd + UPower, так что работать может как ручной процесс (`cargo run`), так и полноценный unit.

## Возможности
- Определяет активную Wi‑Fi сеть через `nmcli` и выбирает профиль по SSID/BSSID/интерфейсу.
- Запускает нужный systemd‑юнит (обычно `sshd.service`) при подключении, а при разрыве отключает его.
- Берёт ингибиторы `login1`, чтобы не засыпать при закрытой крышке или не уходить в suspend (экран при этом может гаснуть).
- Проверяет питание через `org.freedesktop.UPower` и может игнорировать профили, если ноут работает от батареи.
- Распространяется как готовый бинарь, сопровождаемый systemd‑юнитом и шаблоном конфигурации.

## Установка

```bash
sudo bash <(curl -fsSL https://raw.githubusercontent.com/myrs/sshdock/main/install.sh)
```

Инсталлятор делает следующее:

1. Скачивает последнюю сборку (`sshdock-x86_64-unknown-linux-gnu`) из релизов GitHub.
2. Копирует бинарь в `/usr/local/bin/sshdock`.
3. Кладёт `sshdock.service` в `/etc/systemd/system/`.
4. Создаёт `/etc/sshdock/config.toml` с подробным шаблоном (или оставляет ваш файл без изменений).
5. Выполняет `systemctl daemon-reload` и предлагает сразу включить сервис.

Требования: root, наличие `curl`, `systemctl`, `install`.

## Конфигурация (`~/.config/sshdock/config.toml` или `/etc/sshdock/config.toml`)

### Общие настройки

| Параметр | Описание |
| --- | --- |
| `poll_interval_secs` | Период опроса активного Wi‑Fi (по умолчанию 5 секунд). |
| `ssh_service` | Привязанный unit systemd (например, `sshd.service` или `sshd@home.service`). |

### Профиль сети (`[[networks]]`)

| Параметр | По умолчанию | Описание |
| --- | --- | --- |
| `ssid` | — | Имя Wi‑Fi сети (обязательное поле). |
| `name` | `ssid` | Человекопонятная подпись в логах. |
| `bssid` | — | MAC точки доступа; если указан, профиль сработает только при точном совпадении. |
| `interface` | — | Имя интерфейса (например, `wlan0`); полезно, если на машине несколько адаптеров. |
| `enable_ssh` | `true` | Запускать указанную службу `systemctl start …` при подключении. |
| `stop_ssh_on_disconnect` | `true` | Останавливать службу после ухода с сети. |
| `prevent_lid_sleep` | `true` | Берёт ингибитор `handle-lid-switch`, чтобы закрытие крышки не отправляло ноут в сон. |
| `prevent_idle_sleep` | `true` | Берёт ингибитор `sleep` – блокирует перевод системы в suspend по таймауту, но не мешает экрану гаснуть. |
| `ssh_service` | `ssh_service` из корня | Перекрывает глобальное имя unit’а для конкретной сети. |
| `require_ac_power` | `true` | Профиль активируется только если ноут подключён к питанию (через UPower). Поставь `false`, если нужно работать от батареи. |

Пример:

```toml
poll_interval_secs = 5
ssh_service = "sshd.service"

[[networks]]
name = "Док-станция"
ssid = "CorpWifi"
bssid = "AA:BB:CC:DD:EE:FF"
interface = "wlan0"
enable_ssh = true
stop_ssh_on_disconnect = true
prevent_lid_sleep = true
prevent_idle_sleep = true
require_ac_power = true

[[networks]]
name = "Дом"
ssid = "HomeWifi"
enable_ssh = false
prevent_lid_sleep = true
prevent_idle_sleep = false
require_ac_power = false
```

## Журнал

Сервис использует `env_logger`. Чтобы увидеть сообщения при ручном запуске, выставь `RUST_LOG=info cargo run`. В режиме systemd смотри `journalctl -u sshdock.service -f`.
