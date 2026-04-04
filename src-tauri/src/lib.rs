use std::fs;
use std::path::PathBuf;
use std::process::Command;
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;
use std::sync::Mutex;
use serde::Serialize;
use tauri::{State, AppHandle, Emitter};

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
    // On macOS .app bundles, exe_dir is inside the bundle and may not be writable.
    // Use exe_dir on Windows (portable), and a platform-appropriate location on macOS/Linux.
    #[cfg(target_os = "windows")]
    { exe_dir().join("config.ini") }
    #[cfg(not(target_os = "windows"))]
    {
        if let Some(dir) = dirs_next::config_dir() {
            let app_dir = dir.join("yauz");
            let _ = fs::create_dir_all(&app_dir);
            app_dir.join("config.ini")
        } else {
            exe_dir().join("config.ini")
        }
    }
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

/// Search for 7z/7zz in system PATH (and common Homebrew paths on macOS).
/// Returns the full path to the executable if found.
fn find_seven_zip_in_path() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    let names = ["7z.exe", "7zz.exe"];
    #[cfg(not(target_os = "windows"))]
    let names = ["7z", "7zz"];

    // Collect PATH dirs, then append well-known Homebrew dirs on macOS
    #[allow(unused_mut)]
    let mut dirs: Vec<PathBuf> = std::env::var_os("PATH")
        .map(|p| std::env::split_paths(&p).collect())
        .unwrap_or_default();

    #[cfg(target_os = "macos")]
    {
        let homebrew_dirs = [
            "/opt/homebrew/bin",          // Apple Silicon
            "/usr/local/bin",             // Intel
            "/opt/homebrew/sbin",
            "/usr/local/sbin",
        ];
        for d in homebrew_dirs {
            let p = PathBuf::from(d);
            if !dirs.contains(&p) {
                dirs.push(p);
            }
        }
    }

    for dir in &dirs {
        for name in &names {
            let candidate = dir.join(name);
            if candidate.exists() {
                return Some(candidate);
            }
        }
    }
    None
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

/// Check if 7z is available in system PATH.
/// Returns the directory containing the executable, or empty string if not found.
#[tauri::command]
fn detect_seven_zip_in_path() -> String {
    match find_seven_zip_in_path() {
        Some(exe_path) => exe_path.parent()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default(),
        None => String::new(),
    }
}

#[tauri::command]
fn get_seven_zip_version(dir: String) -> String {
    let exe = if dir.is_empty() {
        match find_seven_zip_in_path() {
            Some(p) => p,
            None => return "?".to_string(),
        }
    } else {
        let e = resolve_seven_zip_exe(&dir);
        if !e.exists() { return "?".to_string(); }
        e
    };
    let mut cmd = Command::new(&exe);
    cmd.args(["i"]);
    #[cfg(target_os = "windows")]
    cmd.creation_flags(0x08000000);
    match cmd.output() {
        Ok(o) => {
            let stdout = String::from_utf8_lossy(&o.stdout);
            // Standard:  "7-Zip 24.09 (x64) : Copyright ..."
            // ZS:        "7-Zip 26.00 ZS v1.5.7 (x64) : Copyright ..."
            // macOS 7zz: "7-Zip (z) 24.09 (arm64) : Copyright ..."
            for line in stdout.lines() {
                if line.starts_with("7-Zip") {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    // "7-Zip (z) 24.09 ..." → parts[0]="7-Zip", parts[1]="(z)", parts[2]="24.09"
                    if parts.len() >= 3 && parts[1] == "(z)" {
                        let ver = parts[2];
                        return format!("{} (z)", ver);
                    }
                    // "7-Zip 26.00 ZS v1.5.7 ..."
                    if parts.len() >= 4 && parts[2] == "ZS" {
                        return format!("{} ZS {}", parts[1], parts[3]);
                    }
                    // "7-Zip 24.09 (x64) ..."
                    if parts.len() >= 2 {
                        return parts[1].to_string();
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
    parts: Vec<String>,
}

#[derive(Serialize, Clone)]
struct ExtractProgress {
    current: usize,
    total: usize,
    file: String,
}

/// Detect split-volume siblings for an archive file.
/// Returns sorted list of sibling part filenames (not including the main file itself).
fn detect_split_parts(archive: &str) -> Vec<String> {
    use std::path::Path;
    let path = Path::new(archive);
    let parent = match path.parent() { Some(p) => p, None => return vec![] };
    let filename = match path.file_name() { Some(f) => f.to_string_lossy().to_string(), None => return vec![] };

    // Pattern 1: name.7z.001, name.7z.002, ...
    // Pattern 2: name.zip.001, name.zip.002, ...
    // Pattern 3: name.001, name.002, ...
    if let Some(caps) = regex_match_numbered(&filename) {
        let prefix = &caps.0;
        let ext_len = caps.1;
        let mut parts = Vec::new();
        if let Ok(entries) = fs::read_dir(parent) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name == filename { continue; }
                if name.starts_with(prefix) {
                    let suffix = &name[prefix.len()..];
                    // Check suffix is .NNN (same digit count)
                    if suffix.len() == ext_len + 1 && suffix.starts_with('.') && suffix[1..].chars().all(|c| c.is_ascii_digit()) {
                        parts.push(name);
                    }
                }
            }
        }
        parts.sort();
        return parts;
    }

    // Pattern 4: name.part1.rar, name.part2.rar, ...
    let lower = filename.to_lowercase();
    if let Some(pos) = lower.find(".part") {
        let after_part = &lower[pos + 5..];
        if let Some(dot_pos) = after_part.find('.') {
            let num_str = &after_part[..dot_pos];
            if num_str.chars().all(|c| c.is_ascii_digit()) {
                let prefix = &filename[..pos + 5]; // up to ".partN" → ".part"
                let ext = &filename[pos + 5 + dot_pos..]; // ".rar"
                let mut parts = Vec::new();
                if let Ok(entries) = fs::read_dir(parent) {
                    for entry in entries.flatten() {
                        let name = entry.file_name().to_string_lossy().to_string();
                        if name == filename { continue; }
                        let name_lower = name.to_lowercase();
                        if name_lower.starts_with(&prefix.to_lowercase()) && name_lower.ends_with(&ext.to_lowercase()) {
                            let mid = &name_lower[prefix.len()..name_lower.len() - ext.len()];
                            if mid.chars().all(|c| c.is_ascii_digit()) {
                                parts.push(name);
                            }
                        }
                    }
                }
                parts.sort();
                return parts;
            }
        }
    }

    // Pattern 5: name.z01, name.z02 (main is .zip)
    // Pattern 6: name.r00, name.r01 (main is .rar)
    if lower.ends_with(".zip") || lower.ends_with(".rar") {
        let base = &filename[..filename.len() - 4];
        let letter = if lower.ends_with(".zip") { "z" } else { "r" };
        let mut parts = Vec::new();
        if let Ok(entries) = fs::read_dir(parent) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name == filename { continue; }
                let nl = name.to_lowercase();
                let bl = base.to_lowercase();
                if nl.starts_with(&bl) && nl.len() == bl.len() + 4 {
                    let suffix = &nl[bl.len()..];
                    if suffix.starts_with(&format!(".{}", letter)) && suffix[2..].chars().all(|c| c.is_ascii_digit()) {
                        parts.push(name);
                    }
                }
            }
        }
        parts.sort();
        return parts;
    }

    vec![]
}

/// Match numbered split patterns like "name.7z.001" or "name.001"
/// Returns (prefix_before_number, digit_extension_length)
fn regex_match_numbered(filename: &str) -> Option<(String, usize)> {
    // Find last dot-separated segment that is all digits
    if let Some(last_dot) = filename.rfind('.') {
        let suffix = &filename[last_dot + 1..];
        if suffix.len() >= 3 && suffix.chars().all(|c| c.is_ascii_digit()) {
            let prefix = &filename[..last_dot];
            return Some((prefix.to_string(), suffix.len()));
        }
    }
    None
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
                || combined.contains("Encrypted")
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
fn extract_files(app: AppHandle, state: State<AppState>, files: Vec<String>, out_dir: String) {
    let dir = state.seven_zip_dir.lock().unwrap().clone();
    let passwords = state.passwords.lock().unwrap().clone();
    let sz = if dir.is_empty() {
        // No configured dir — try system PATH
        find_seven_zip_in_path().unwrap_or_default()
    } else {
        resolve_seven_zip_exe(&dir)
    };

    std::thread::spawn(move || {
        if !sz.exists() {
            let results: Vec<ExtractResult> = files.iter().map(|f| ExtractResult {
                file: f.clone(), success: false,
                reason: format!("7-Zip 未找到: {}", sz.display()),
                password: String::new(),
                parts: vec![],
            }).collect();
            let _ = app.emit("extract-done", results);
            return;
        }

        let total = files.len();
        let mut results = Vec::new();

        for (i, file_path) in files.iter().enumerate() {
            let _ = app.emit("extract-progress", ExtractProgress {
                current: i + 1, total,
                file: file_path.clone(),
            });

            let archive = file_path.as_str();
            let out = if out_dir.is_empty() {
                std::path::Path::new(archive)
                    .parent()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_else(|| ".".to_string())
            } else {
                out_dir.clone()
            };

            let (ok, msg) = try_extract(&sz, archive, &out, None);
            let parts = detect_split_parts(archive);
            if ok {
                results.push(ExtractResult { file: file_path.clone(), success: true, reason: String::new(), password: String::new(), parts });
                continue;
            }

            if needs_password(&sz, archive) {
                let mut extracted = false;
                for pw in &passwords {
                    let (ok, _) = try_extract(&sz, archive, &out, Some(pw));
                    if ok {
                        results.push(ExtractResult { file: file_path.clone(), success: true, reason: String::new(), password: pw.clone(), parts: parts.clone() });
                        extracted = true;
                        break;
                    }
                }
                if !extracted {
                    results.push(ExtractResult { file: file_path.clone(), success: false, reason: "All passwords incorrect".to_string(), password: String::new(), parts });
                }
            } else {
                results.push(ExtractResult {
                    file: file_path.clone(), success: false,
                    reason: msg.lines().last().unwrap_or("未知错误").to_string(),
                    password: String::new(),
                    parts,
                });
            }
        }

        let _ = app.emit("extract-done", results);
    });
}

pub fn run() {
    let (passwords, mut seven_zip_dir, language, mut needs_setup) = load_config();

    // If a manual 7z path is configured but invalid, clear it and fall back
    if !seven_zip_dir.is_empty() {
        let exe = resolve_seven_zip_exe(&seven_zip_dir);
        if !exe.exists() {
            seven_zip_dir = String::new();
            let _ = save_config(&passwords, "", &language);
            // Re-evaluate setup need since the saved path was invalid
            needs_setup = true;
        }
    }

    // If 7z is found in system PATH, skip setup (user can still override in settings)
    if find_seven_zip_in_path().is_some() {
        needs_setup = false;
    }

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
            check_needs_setup, detect_seven_zip_in_path,
            extract_files
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
