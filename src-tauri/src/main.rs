// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    #[cfg(target_os = "windows")]
    {
        if !webview2_is_installed() {
            show_webview2_missing_dialog();
            std::process::exit(1);
        }
    }

    app_lib::run();
}

/// Check whether the WebView2 Evergreen Runtime is installed by inspecting
/// the registry keys recommended by Microsoft.
#[cfg(target_os = "windows")]
fn webview2_is_installed() -> bool {
    use winreg::enums::*;
    use winreg::RegKey;

    let paths = [
        (HKEY_LOCAL_MACHINE, r"SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}"),
        (HKEY_CURRENT_USER, r"Software\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}"),
    ];

    for (root, subkey) in &paths {
        if let Ok(key) = RegKey::predef(*root).open_subkey(subkey) {
            if let Ok(pv) = key.get_value::<String, _>("pv") {
                if !pv.is_empty() && pv != "0.0.0.0" {
                    return true;
                }
            }
        }
    }
    false
}

/// Show a native Windows message box telling the user that WebView2 is
/// required, with an OK button. After the user dismisses it, open the
/// WebView2 download page in the default browser.
#[cfg(target_os = "windows")]
fn show_webview2_missing_dialog() {
    use std::ptr;

    // Wide-string helper
    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }

    let title = wide("YAUZ");
    let text = wide(
        "YAUZ requires the Microsoft Edge WebView2 Runtime, which is not installed on this system.\n\n\
         Click OK to open the download page, then install the runtime and relaunch YAUZ.\n\n\
         YAUZ 需要 Microsoft Edge WebView2 Runtime，但当前系统未安装。\n\n\
         点击"确定"打开下载页面，安装后重新启动 YAUZ。"
    );

    // MB_OK | MB_ICONERROR
    unsafe {
        windows_sys::Win32::UI::WindowsAndMessaging::MessageBoxW(
            ptr::null_mut(),
            text.as_ptr(),
            title.as_ptr(),
            0x00000010, // MB_ICONERROR
        );
    }

    // Open the download page
    let _ = std::process::Command::new("cmd")
        .args(["/c", "start", "https://developer.microsoft.com/en-us/microsoft-edge/webview2/"])
        .spawn();
}
