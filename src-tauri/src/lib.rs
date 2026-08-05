#![allow(non_snake_case)]
use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use std::path::PathBuf;

mod shared;

// Windows API FFI —— 单实例锁
const ERROR_ALREADY_EXISTS: u32 = 183;
const SW_RESTORE: i32 = 9;

extern "system" {
    fn CreateMutexW(
        lpMutexAttributes: *const std::ffi::c_void,
        bInitialOwner: i32,
        lpName: *const u16,
    ) -> isize;
    fn GetLastError() -> u32;
    fn FindWindowW(lpClassName: *const u16, lpWindowName: *const u16) -> isize;
    fn SetForegroundWindow(hWnd: isize) -> i32;
    fn ShowWindow(hWnd: isize, nCmdShow: i32) -> i32;
}

fn to_wide(s: &str) -> Vec<u16> {
    OsStr::new(s).encode_wide().chain(std::iter::once(0)).collect()
}

/// 检查是否已有实例在运行。返回 true 表示已有实例。
fn check_single_instance() -> bool {
    let name = to_wide("Local\\NovelReader_SingleInstance");
    unsafe {
        CreateMutexW(std::ptr::null(), 0, name.as_ptr());
        GetLastError() == ERROR_ALREADY_EXISTS
    }
}

/// 激活已有窗口（找到并置前）
fn activate_existing_window() {
    let title = to_wide("小说阅读器");
    unsafe {
        let hwnd = FindWindowW(std::ptr::null(), title.as_ptr());
        if hwnd != 0 {
            ShowWindow(hwnd, SW_RESTORE);
            SetForegroundWindow(hwnd);
        }
    }
}

fn app_dir() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."))
}

// ===== Tauri 命令（薄包装 shared 层） =====

#[tauri::command]
fn get_books(app: tauri::AppHandle) -> Vec<shared::Book> {
    let _ = app;
    shared::get_books(&app_dir())
}

#[tauri::command]
fn get_settings(app: tauri::AppHandle) -> shared::Settings {
    let _ = app;
    shared::get_settings(&app_dir())
}

#[tauri::command]
fn save_settings(app: tauri::AppHandle, lastBookId: Option<String>, theme: Option<String>, fontSize: Option<u32>, lineHeight: Option<f64>) {
    let _ = app;
    shared::save_settings(&app_dir(), lastBookId, theme, fontSize, lineHeight)
}

#[tauri::command]
fn import_book(app: tauri::AppHandle, name: String, content: String) -> Result<shared::ImportResult, String> {
    let _ = app;
    shared::import_book(&app_dir(), name, &content)
}

#[tauri::command]
fn open_saved(app: tauri::AppHandle, id: String) -> Result<shared::OpenSavedResult, String> {
    let _ = app;
    shared::open_saved(&app_dir(), &id)
}

#[tauri::command]
fn update_book(app: tauri::AppHandle, id: String, lastChapter: Option<usize>, lastPosition: Option<f64>, fontSize: Option<u32>, lineHeight: Option<f64>, theme: Option<String>) -> Result<(), String> {
    let _ = app;
    shared::update_book(&app_dir(), &id, lastChapter, lastPosition, fontSize, lineHeight, theme)
}

#[tauri::command]
fn delete_book(app: tauri::AppHandle, id: String) -> Result<(), String> {
    let _ = app;
    shared::delete_book(&app_dir(), &id)
}

#[tauri::command]
fn rename_book(app: tauri::AppHandle, id: String, newName: String) -> Result<(), String> {
    let _ = app;
    shared::rename_book(&app_dir(), &id, &newName)
}

#[tauri::command]
fn set_window_bg(app: tauri::AppHandle, color: String) {
    use tauri::Manager;
    if let Some(win) = app.get_webview_window("main") {
        // 同步更新 WebView2 原生背景色，防止 resize 时闪烁
        if let Ok(c) = u32::from_str_radix(color.trim_start_matches('#'), 16) {
            let r = ((c >> 16) & 0xFF) as u8;
            let g = ((c >> 8) & 0xFF) as u8;
            let b = (c & 0xFF) as u8;
            let _ = win.set_background_color(Some(tauri::window::Color::from((r, g, b, 255u8))));
        }
        // 同步更新 CSS 背景色
        let _ = win.eval(&format!(
            "document.documentElement.style.setProperty('background-color','{}','important')",
            color
        ));
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // 单实例锁：如果已有实例在运行，激活其窗口后退出
    if check_single_instance() {
        activate_existing_window();
        std::process::exit(0);
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .setup(|app| {
            shared::ensure_data(&app_dir());

            // 根据已保存主题创建窗口，避免 resize 时 WebView2 白闪
            let settings = shared::get_settings(&app_dir());
            tauri::WebviewWindowBuilder::new(app, "main", tauri::WebviewUrl::App("index.html".into()))
                .title("小说阅读器")
                .inner_size(1200.0, 800.0)
                .min_inner_size(350.0, 400.0)
                .resizable(true)
                .fullscreen(false)
                .background_color({
                    let c = if settings.theme == "night" { (26u8, 26u8, 26u8, 255u8) } else { (245u8, 240u8, 232u8, 255u8) };
                    tauri::window::Color::from(c)
                })
                .build()?;

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_books,
            get_settings,
            save_settings,
            import_book,
            open_saved,
            update_book,
            delete_book,
            rename_book,
            set_window_bg,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
