// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // On Windows, point WebView2 to the bundled Fixed Version runtime
    // located in a "runtime" folder next to the executable, so the app
    // works on machines without Edge or the Evergreen WebView2 Runtime.
    #[cfg(target_os = "windows")]
    {
        let exe = std::env::current_exe().expect("failed to get exe path");
        let runtime_dir = exe.parent().unwrap().join("runtime");
        if runtime_dir.exists() {
            std::env::set_var("WEBVIEW2_BROWSER_EXECUTABLE_FOLDER", &runtime_dir);
        }
    }

    app_lib::run();
}
