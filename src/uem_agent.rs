// Teamdesk UEM — встроенный managed-агент клиента (Фаза 1).
//
// Активируется ТОЛЬКО если задан enroll-токен (config "uem-token" или переменная
// окружения TEAMDESK_UEM_TOKEN), либо устройство уже зачислено (сохранён
// "uem-device-token"). В обычном (не managed) режиме модуль полностью инертен —
// поток даже не создаётся, на рядового пользователя влияния нет.
//
// Провижининг для организации:
//   * MSI/командой:  выставить config-опцию "uem-token" (и при желании "uem-url");
//   * либо переменной окружения TEAMDESK_UEM_TOKEN / TEAMDESK_UEM_URL.
// После первого enroll устройство запоминает device-token и enroll-токен больше
// не нужен. teamdesk_id устройства = ID этого клиента → консоль сможет предложить
// удалённое подключение к управляемой машине штатным клиентом Teamdesk.

use hbb_common::config::Config;
use hbb_common::log;
use serde_json::{json, Value};
use std::time::Duration;

const DEFAULT_URL: &str = "https://uem.teamdesk.su/api";

fn uem_url() -> String {
    if let Ok(v) = std::env::var("TEAMDESK_UEM_URL") {
        if !v.is_empty() {
            return v;
        }
    }
    let v = Config::get_option("uem-url");
    if v.is_empty() {
        DEFAULT_URL.to_owned()
    } else {
        v
    }
}

fn enroll_token() -> String {
    if let Ok(v) = std::env::var("TEAMDESK_UEM_TOKEN") {
        if !v.is_empty() {
            return v;
        }
    }
    Config::get_option("uem-token")
}

fn platform_name() -> &'static str {
    #[cfg(target_os = "windows")]
    {
        "windows"
    }
    #[cfg(target_os = "macos")]
    {
        "macos"
    }
    #[cfg(target_os = "linux")]
    {
        "linux"
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        "other"
    }
}

fn sh(cmd: &str) -> String {
    #[cfg(windows)]
    let out = std::process::Command::new("cmd").args(["/C", cmd]).output();
    #[cfg(not(windows))]
    let out = std::process::Command::new("sh").args(["-c", cmd]).output();
    match out {
        Ok(o) => String::from_utf8_lossy(&o.stdout).trim().to_string(),
        Err(_) => String::new(),
    }
}

fn hostname() -> String {
    #[cfg(windows)]
    {
        let h = std::env::var("COMPUTERNAME").unwrap_or_default();
        if !h.is_empty() {
            return h;
        }
    }
    let h = sh("hostname");
    if h.is_empty() {
        "unknown".to_owned()
    } else {
        h
    }
}

fn inventory() -> Value {
    let mut inv = json!({
        "имя хоста": hostname(),
        "ОС": std::env::consts::OS,
        "архитектура": std::env::consts::ARCH,
        "teamdesk_id": Config::get_id(),
    });
    #[cfg(target_os = "windows")]
    {
        inv["система"] = json!(sh("ver"));
        inv["процессор"] = json!(std::env::var("PROCESSOR_IDENTIFIER").unwrap_or_default());
        inv["пользователь"] = json!(std::env::var("USERNAME").unwrap_or_default());
        inv["домен"] = json!(std::env::var("USERDOMAIN").unwrap_or_default());
    }
    #[cfg(target_os = "linux")]
    {
        inv["дистрибутив"] =
            json!(sh("grep PRETTY_NAME /etc/os-release | cut -d= -f2 | tr -d '\"'"));
        inv["ядро"] = json!(sh("uname -r"));
        inv["процессор"] = json!(sh("grep -m1 'model name' /proc/cpuinfo | cut -d: -f2 | sed 's/^ //'"));
        inv["ОЗУ, ГБ"] = json!(sh("awk '/MemTotal/{printf \"%.0f\", $2/1024/1024}' /proc/meminfo"));
        inv["диск /"] = json!(sh("df -h / | tail -1 | awk '{print $2\" всего, \"$4\" своб\"}'"));
        inv["uptime"] = json!(sh("uptime -p"));
        inv["пользователь"] = json!(std::env::var("USER").unwrap_or_default());
    }
    #[cfg(target_os = "macos")]
    {
        inv["система"] = json!(format!("macOS {}", sh("sw_vers -productVersion")));
        inv["процессор"] = json!(sh("sysctl -n machdep.cpu.brand_string"));
        inv["ОЗУ, ГБ"] = json!(sh("sysctl -n hw.memsize | awk '{printf \"%.0f\", $1/1073741824}'"));
        inv["диск /"] = json!(sh("df -h / | tail -1 | awk '{print $2\" всего, \"$4\" своб\"}'"));
        inv["пользователь"] = json!(std::env::var("USER").unwrap_or_default());
    }
    inv
}

fn client() -> reqwest::blocking::Client {
    reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .unwrap_or_else(|_| reqwest::blocking::Client::new())
}

fn post(cli: &reqwest::blocking::Client, path: &str, body: &Value) -> Option<Value> {
    let url = format!("{}{}", uem_url(), path);
    match cli.post(&url).json(body).send() {
        Ok(r) => r.json::<Value>().ok(),
        Err(e) => {
            log::debug!("[UEM] POST {} error: {}", path, e);
            None
        }
    }
}

fn run_shell(ty: &str, payload: &str) -> (String, String) {
    #[cfg(windows)]
    let out = if ty == "script" {
        std::process::Command::new("powershell")
            .args(["-NoProfile", "-Command", payload])
            .output()
    } else {
        std::process::Command::new("cmd").args(["/C", payload]).output()
    };
    #[cfg(not(windows))]
    let out = {
        let _ = ty;
        std::process::Command::new("sh").args(["-c", payload]).output()
    };
    match out {
        Ok(o) => {
            let mut s = String::from_utf8_lossy(&o.stdout).to_string();
            s.push_str(&String::from_utf8_lossy(&o.stderr));
            let status = if o.status.success() { "success" } else { "error" };
            (status.to_owned(), s)
        }
        Err(e) => ("error".to_owned(), e.to_string()),
    }
}

/// Локальный прогон Ansible-плейбука (ansible-pull модель: -c local).
/// Windows не является нативным control-node Ansible → понятная ошибка.
fn run_ansible(playbook: &str) -> (String, String) {
    #[cfg(windows)]
    {
        let _ = playbook;
        (
            "error".to_owned(),
            "ansible на Windows не поддерживается (используйте команды/скрипты; Ansible — для Linux/macOS)"
                .to_owned(),
        )
    }
    #[cfg(not(windows))]
    {
        use std::io::Write;
        let present = std::process::Command::new("ansible-playbook")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);
        if !present {
            return (
                "error".to_owned(),
                "ansible-playbook не установлен на устройстве".to_owned(),
            );
        }
        let path = std::env::temp_dir().join(format!("td-play-{}.yml", std::process::id()));
        if let Ok(mut f) = std::fs::File::create(&path) {
            let _ = f.write_all(playbook.as_bytes());
        }
        let out = std::process::Command::new("ansible-playbook")
            .args(["-i", "localhost,", "-c", "local"])
            .arg(&path)
            .output();
        let _ = std::fs::remove_file(&path);
        match out {
            Ok(o) => {
                let mut s = String::from_utf8_lossy(&o.stdout).to_string();
                s.push_str(&String::from_utf8_lossy(&o.stderr));
                let status = if o.status.success() { "success" } else { "error" };
                (status.to_owned(), s)
            }
            Err(e) => ("error".to_owned(), e.to_string()),
        }
    }
}

fn do_reboot() {
    #[cfg(windows)]
    {
        let _ = std::process::Command::new("shutdown")
            .args(["/r", "/t", "30"])
            .spawn();
    }
    #[cfg(not(windows))]
    {
        let _ = std::process::Command::new("sh")
            .args(["-c", "sleep 30; reboot"])
            .spawn();
    }
}

fn exec_task(cli: &reqwest::blocking::Client, dtok: &str, t: &Value) {
    let ty = t.get("type").and_then(|x| x.as_str()).unwrap_or("");
    let payload = t.get("payload").and_then(|x| x.as_str()).unwrap_or("");
    let run_id = t.get("run_id").and_then(|x| x.as_str()).unwrap_or("");
    log::info!("[UEM] task {} run_id={}", ty, run_id);
    let (status, output) = match ty {
        "command" | "script" => run_shell(ty, payload),
        "ansible" => run_ansible(payload),
        "inventory" => {
            let _ = post(
                cli,
                "/agent/inventory",
                &json!({"device_token": dtok, "data": inventory()}),
            );
            ("success".to_owned(), "инвентарь отправлен".to_owned())
        }
        "reboot" => {
            do_reboot();
            ("success".to_owned(), "перезагрузка через 30 сек".to_owned())
        }
        _ => ("error".to_owned(), "неизвестный тип задачи".to_owned()),
    };
    let out: String = output.chars().take(20000).collect();
    let _ = post(
        cli,
        "/agent/task-result",
        &json!({"device_token": dtok, "run_id": run_id, "status": status, "output": out}),
    );
}

/// Применяет политику (desired-state): выполняет шаги по порядку и шлёт результат.
/// Сервер доставляет политику пока applied_revision < revision, поэтому агент
/// переприменяет её при каждом изменении и при первом зачислении в область.
fn apply_policy(cli: &reqwest::blocking::Client, dtok: &str, p: &Value) {
    let policy_id = p.get("policy_id").and_then(|x| x.as_str()).unwrap_or("");
    let revision = p.get("revision").and_then(|x| x.as_i64()).unwrap_or(0);
    let steps = p
        .get("spec")
        .and_then(|s| s.get("steps"))
        .and_then(|x| x.as_array())
        .cloned()
        .unwrap_or_default();
    let mut status = "applied".to_owned();
    let mut out = String::new();
    for (i, step) in steps.iter().enumerate() {
        let ty = step.get("type").and_then(|x| x.as_str()).unwrap_or("command");
        let run = step.get("run").and_then(|x| x.as_str()).unwrap_or("");
        let (st, so) = if ty == "ansible" {
            let pb = step
                .get("playbook")
                .and_then(|x| x.as_str())
                .unwrap_or(run);
            run_ansible(pb)
        } else {
            run_shell(ty, run)
        };
        let label = if ty == "ansible" { "ANSIBLE" } else { run };
        if st == "error" {
            status = "error".to_owned();
            out.push_str(&format!("[{}] ОШИБКА: {}\n{}\n", i + 1, label, so));
            break;
        }
        out.push_str(&format!("[{}] OK: {}\n{}\n", i + 1, label, so));
    }
    let out: String = out.chars().take(20000).collect();
    let _ = post(
        cli,
        "/agent/policy-result",
        &json!({"device_token": dtok, "policy_id": policy_id, "revision": revision, "status": status, "output": out}),
    );
    log::info!("[UEM] policy {} rev {} -> {}", policy_id, revision, status);
}

fn run() {
    let cli = client();
    let mut dtok = Config::get_option("uem-device-token");
    // Enroll при первом запуске.
    if dtok.is_empty() {
        let et = enroll_token();
        if et.is_empty() {
            return;
        }
        let body = json!({
            "enroll_token": et,
            "name": hostname(),
            "platform": platform_name(),
            "agent_version": "client-embedded-0.1",
            "teamdesk_id": Config::get_id(),
        });
        loop {
            if let Some(v) = post(&cli, "/agent/enroll", &body) {
                if let Some(t) = v.get("device_token").and_then(|x| x.as_str()) {
                    dtok = t.to_owned();
                    Config::set_option("uem-device-token".to_owned(), dtok.clone());
                    log::info!(
                        "[UEM] enrolled device {}",
                        v.get("device_id").and_then(|x| x.as_str()).unwrap_or("")
                    );
                    break;
                }
            }
            std::thread::sleep(Duration::from_secs(30));
        }
    }
    let _ = post(
        &cli,
        "/agent/inventory",
        &json!({"device_token": dtok, "data": inventory()}),
    );
    let mut tick: u64 = 0;
    loop {
        if let Some(hb) = post(&cli, "/agent/heartbeat", &json!({"device_token": dtok})) {
            if let Some(tasks) = hb.get("tasks").and_then(|x| x.as_array()) {
                for t in tasks {
                    exec_task(&cli, &dtok, t);
                }
            }
            if let Some(pols) = hb.get("policies").and_then(|x| x.as_array()) {
                for p in pols {
                    apply_policy(&cli, &dtok, p);
                }
            }
        }
        tick += 1;
        if tick % 30 == 0 {
            let _ = post(
                &cli,
                "/agent/inventory",
                &json!({"device_token": dtok, "data": inventory()}),
            );
        }
        std::thread::sleep(Duration::from_secs(10));
    }
}

/// Запускается из ветки `--server` (единственный всегда-живой процесс).
/// Немедленно возвращается, если устройство не под управлением.
pub fn start_managed() {
    let managed =
        !enroll_token().is_empty() || !Config::get_option("uem-device-token").is_empty();
    if !managed {
        return;
    }
    log::info!("[UEM] managed mode: агент запущен");
    std::thread::spawn(|| {
        run();
    });
}
