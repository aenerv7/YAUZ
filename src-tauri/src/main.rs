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
    let text = wide(concat!(
        "YAUZ requires the Microsoft Edge WebView2 Runtime, which is not installed on this system.\n\n",
        "Click OK to open the download page, then install the runtime and relaunch YAUZ.\n\n",
        "YAUZ \u{9700}\u{8981} Microsoft Edge WebView2 Runtime\u{FF0C}\u{4F46}\u{5F53}\u{524D}\u{7CFB}\u{7EDF}\u{672A}\u{5B89}\u{88C5}\u{3002}\n\n",
        "\u{70B9}\u{51FB}\u{300C}\u{786E}\u{5B9A}\u{300D}\u{6253}\u{5F00}\u{4E0B}\u{8F7D}\u{9875}\u{9762}\u{FF0C}\u{5B89}\u{88C5}\u{540E}\u{91CD}\u{65B0}\u{542F}\u{52A8} YAUZ\u{3002}",
    ));

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
