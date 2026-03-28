use std::fs;
use std::path::PathBuf;
use std::process::Command;
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;
use std::sync::Mutex;
use serde::Serialize;
use tauri::State;

struct AppState {
    passwords: Mutex<Vec<String>>,
    seven_zip_dir: Mutex<String>,
    language: Mutex<String>,
    needs_setup: Mutex<bool>,
}

/// 可执行文件所在目录
fn exe_dir() -> PathBuf {
    let exe = std::env::current_exe().expect("failed to get exe path");
    exe.parent().unwrap().to_path_buf()
}

fn ini_path() -> PathBuf {
    exe_dir().join("config.ini")
}

fn expand_env_vars(path: &str) -> String {
    let mut result = path.to_string();
    // Windows: %VAR%
    while let Some(start) = result.find('%') {
        if let Some(end) = result[start + 1..].find('%') {
            let var_name = &result[start + 1..start + 1 + end];
            let val = std::env::var(var_name).unwrap_or_default();
            result = format!("{}{}{}", &result[..start], val, &result[start + 2 + end..]);
        } else {
            break;
        }
    }
    // Unix: $VAR or ${VAR}
    while let Some(start) = result.find('$') {
        if result[start + 1..].starts_with('{') {
            if let Some(end) = result[start + 2..].find('}') {
                let var_name = &result[start + 2..start + 2 + end];
                let val = std::env::var(var_name).unwrap_or_default();
                result = format!("{}{}{}", &result[..start], val, &result[start + 3 + end..]);
            } else { break; }
        } else {
            let rest = &result[start + 1..];
            let end = rest.find(|c: char| !c.is_alphanumeric() && c != '_').unwrap_or(rest.len());
            let var_name = &rest[..end];
            if var_name.is_empty() { break; }
            let val = std::env::var(var_name).unwrap_or_default();
            result = format!("{}{}{}", &result[..start], val, &rest[end..]);
        }
    }
    result
}

fn resolve_seven_zip_exe(dir: &str) -> PathBuf {
    let d = PathBuf::from(expand_env_vars(dir));
    #[cfg(target_os = "windows")]
    {
        let exe = d.join("7z.exe");
        if exe.exists() { return exe; }
        d.join("7zz.exe")
    }
    #[cfg(not(target_os = "windows"))]
    {
        let exe = d.join("7z");
        if exe.exists() { return exe; }
        d.join("7zz")
    }
}

// ── INI 读写 ──

fn load_config() -> (Vec<String>, String, String, bool) {
    let path = ini_path();
    let mut first_run = false;
    if !path.exists() {
        first_run = true;
        return (Vec::new(), String::new(), String::new(), first_run);
    }
    let content = fs::read_to_string(&path).unwrap_or_default();
    let mut passwords = Vec::new();
    let mut seven_zip_dir = String::new();
    let mut language = String::new();
    let mut current_section = String::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            current_section = trimmed[1..trimmed.len()-1].to_string();
            continue;
        }
        if trimmed.is_empty() { continue; }
        match current_section.as_str() {
            "passwords" => passwords.push(trimmed.to_string()),
            "settings" => {
                if let Some(val) = trimmed.strip_prefix("7zip_dir=") {
                    seven_zip_dir = val.trim().to_string();
                }
                if let Some(val) = trimmed.strip_prefix("language=") {
                    language = val.trim().to_string();
                }
            }
            _ => {}
        }
    }
    if seven_zip_dir.is_empty() {
        first_run = true;
    }
    (passwords, seven_zip_dir, language, first_run)
}

fn save_config(passwords: &[String], seven_zip_dir: &str, language: &str) -> Result<(), String> {
    let path = ini_path();
    let mut content = String::from("[settings]\n");
    content.push_str(&format!("7zip_dir={}\n", seven_zip_dir));
    content.push_str(&format!("language={}\n\n", language));
    content.push_str("[passwords]\n");
    for p in passwords {
        content.push_str(p);
        content.push('\n');
    }
    fs::write(&path, &content).map_err(|e| e.to_string())
}

// ── 密码命令 ──

#[tauri::command]
fn get_passwords(state: State<AppState>) -> Vec<String> {
    state.passwords.lock().unwrap().clone()
}

#[tauri::command]
fn save_passwords(state: State<AppState>, passwords: Vec<String>) -> Result<(), String> {
    let mut seen = std::collections::HashSet::new();
    let deduped: Vec<String> = passwords.into_iter().filter(|p| seen.insert(p.clone())).collect();
    let mut sorted = deduped;
    sorted.sort();
    let mut stored = state.passwords.lock().unwrap();
    *stored = sorted;
    let dir = state.seven_zip_dir.lock().unwrap().clone();
    let lang = state.language.lock().unwrap().clone();
    save_config(&stored, &dir, &lang)
}

// ── 7-Zip 路径命令 ──

#[tauri::command]
fn get_seven_zip_dir(state: State<AppState>) -> String {
    state.seven_zip_dir.lock().unwrap().clone()
}

#[tauri::command]
fn save_seven_zip_dir(state: State<AppState>, dir: String) -> Result<(), String> {
    let mut stored = state.seven_zip_dir.lock().unwrap();
    *stored = dir.clone();
    let passwords = state.passwords.lock().unwrap().clone();
    let lang = state.language.lock().unwrap().clone();
    let result = save_config(&passwords, &dir, &lang);
    if result.is_ok() {
        *state.needs_setup.lock().unwrap() = false;
    }
    result
}

#[tauri::command]
fn get_language(state: State<AppState>) -> String {
    state.language.lock().unwrap().clone()
}

#[tauri::command]
fn save_language(state: State<AppState>, language: String) -> Result<(), String> {
    let mut stored = state.language.lock().unwrap();
    *stored = language.clone();
    let passwords = state.passwords.lock().unwrap().clone();
    let dir = state.seven_zip_dir.lock().unwrap().clone();
    save_config(&passwords, &dir, &language)
}

#[tauri::command]
fn check_seven_zip_dir(dir: String) -> bool {
    let exe = resolve_seven_zip_exe(&dir);
    exe.exists()
}

#[tauri::command]
fn get_seven_zip_version(dir: String) -> String {
    let exe = resolve_seven_zip_exe(&dir);
    if !exe.exists() { return "?".to_string(); }
    let mut cmd = Command::new(&exe);
    cmd.args(["i"]);
    #[cfg(target_os = "windows")]
    cmd.creation_flags(0x08000000);
    match cmd.output() {
        Ok(o) => {
            let stdout = String::from_utf8_lossy(&o.stdout);
            // Standard: "7-Zip 24.09 (x64) : Copyright ..."
            // ZS:       "7-Zip 26.00 ZS v1.5.7 (x64) : Copyright ..."
            for line in stdout.lines() {
                if line.starts_with("7-Zip") {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 2 {
                        let ver = parts[1];
                        // Check for ZS edition
                        if parts.len() >= 4 && parts[2] == "ZS" {
                            return format!("{} ZS {}", ver, parts[3]);
                        }
                        return ver.to_string();
                    }
                }
            }
            "?".to_string()
        }
        Err(_) => "?".to_string(),
    }
}

#[tauri::command]
fn check_needs_setup(state: State<AppState>) -> bool {
    *state.needs_setup.lock().unwrap()
}

// ── 解压逻辑 ──

#[derive(Serialize, Clone)]
struct ExtractResult {
    file: String,
    success: bool,
    reason: String,
    password: String,
}

fn needs_password(sz: &PathBuf, archive: &str) -> bool {
    let mut cmd = Command::new(sz);
    cmd.args(["t", archive, "-p-"]);
    #[cfg(target_os = "windows")]
    cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
    let output = cmd.output();
    match output {
        Ok(o) => {
            let stderr = String::from_utf8_lossy(&o.stderr);
            let stdout = String::from_utf8_lossy(&o.stdout);
            let combined = format!("{}{}", stdout, stderr);
            combined.contains("Wrong password")
                || combined.contains("Cannot open encrypted archive")
                || combined.contains("Enter password")
        }
        Err(_) => false,
    }
}

fn try_extract(sz: &PathBuf, archive: &str, out_dir: &str, password: Option<&str>) -> (bool, String) {
    let mut args = vec!["x".to_string(), archive.to_string(), format!("-o{}", out_dir), "-aoa".to_string()];
    if let Some(pw) = password {
        args.push(format!("-p{}", pw));
    }
    let mut cmd = Command::new(sz);
    cmd.args(&args);
    #[cfg(target_os = "windows")]
    cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
    let output = cmd.output();
    match output {
        Ok(o) => {
            let code = o.status.code().unwrap_or(-1);
            if code == 0 {
                (true, String::new())
            } else {
                let stderr = String::from_utf8_lossy(&o.stderr);
                let stdout = String::from_utf8_lossy(&o.stdout);
                (false, format!("{}{}", stdout, stderr))
            }
        }
        Err(e) => (false, e.to_string()),
    }
}

#[tauri::command]
fn extract_files(state: State<AppState>, files: Vec<String>, out_dir: String) -> Vec<ExtractResult> {
    let dir = state.seven_zip_dir.lock().unwrap().clone();
    let sz = resolve_seven_zip_exe(&dir);
    if !sz.exists() {
        return files.iter().map(|f| ExtractResult {
            file: f.clone(), success: false,
            reason: format!("7-Zip 未找到: {}", sz.display()),
            password: String::new(),
        }).collect();
    }

    let passwords = state.passwords.lock().unwrap().clone();
    let mut results = Vec::new();

    for file_path in &files {
        let archive = file_path.as_str();
        let out_dir = if out_dir.is_empty() {
            std::path::Path::new(archive)
                .parent()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|| ".".to_string())
        } else {
            out_dir.clone()
        };

        let (ok, msg) = try_extract(&sz, archive, &out_dir, None);
        if ok {
            results.push(ExtractResult { file: file_path.clone(), success: true, reason: String::new(), password: String::new() });
            continue;
        }

        if needs_password(&sz, archive) {
            let mut extracted = false;
            for pw in &passwords {
                let (ok, _) = try_extract(&sz, archive, &out_dir, Some(pw));
                if ok {
                    results.push(ExtractResult { file: file_path.clone(), success: true, reason: String::new(), password: pw.clone() });
                    extracted = true;
                    break;
                }
            }
            if !extracted {
                results.push(ExtractResult { file: file_path.clone(), success: false, reason: "所有密码均不正确".to_string(), password: String::new() });
            }
        } else {
            results.push(ExtractResult {
                file: file_path.clone(), success: false,
                reason: msg.lines().last().unwrap_or("未知错误").to_string(),
                password: String::new(),
            });
        }
    }
    results
}

pub fn run() {
    let (passwords, seven_zip_dir, language, needs_setup) = load_config();
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .manage(AppState {
            passwords: Mutex::new(passwords),
            seven_zip_dir: Mutex::new(seven_zip_dir),
            language: Mutex::new(language),
            needs_setup: Mutex::new(needs_setup),
        })
        .invoke_handler(tauri::generate_handler![
            get_passwords, save_passwords,
            get_seven_zip_dir, save_seven_zip_dir, check_seven_zip_dir, get_seven_zip_version,
            get_language, save_language,
            check_needs_setup,
            extract_files
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
