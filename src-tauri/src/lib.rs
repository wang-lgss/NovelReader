#![allow(non_snake_case)]
use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use std::time::Duration;

pub mod shared;
pub mod fanqie;
pub mod heif;

// Windows API FFI —— 单实例锁
const ERROR_ALREADY_EXISTS: u32 = 183;
const SW_RESTORE: i32 = 9;
const MB_TOPMOST: u32 = 0x00040000;

extern "system" {
    fn CreateMutexW(
        lpMutexAttributes: *const std::ffi::c_void,
        bInitialOwner: i32,
        lpName: *const u16,
    ) -> isize;
    fn GetLastError() -> u32;
    fn SetLastError(dwErrCode: u32);
    fn FindWindowW(lpClassName: *const u16, lpWindowName: *const u16) -> isize;
    fn SetForegroundWindow(hWnd: isize) -> i32;
    fn ShowWindow(hWnd: isize, nCmdShow: i32) -> i32;
    fn MessageBoxW(hWnd: isize, lpText: *const u16, lpCaption: *const u16, uType: u32) -> i32;
}

fn to_wide(s: &str) -> Vec<u16> {
    OsStr::new(s).encode_wide().chain(std::iter::once(0)).collect()
}

/// 检查是否已有实例在运行。返回 true 表示已有实例。
fn check_single_instance() -> bool {
    let name = to_wide("Local\\NovelReader_SingleInstance");
    unsafe {
        // 必须先清零，否则可能把上一个 Win32 调用残留的错误码误判成“已有实例”
        SetLastError(0);
        CreateMutexW(std::ptr::null(), 0, name.as_ptr());
        GetLastError() == ERROR_ALREADY_EXISTS
    }
}

/// 激活已有窗口（找到并置前）。返回 false 表示找不到窗口。
fn activate_existing_window() -> bool {
    let title = to_wide("NovelReader");
    unsafe {
        let hwnd = FindWindowW(std::ptr::null(), title.as_ptr());
        if hwnd != 0 {
            ShowWindow(hwnd, SW_RESTORE);
            SetForegroundWindow(hwnd);
            true
        } else {
            false
        }
    }
}

/// 提示用户已有实例在运行（找不到旧窗口时使用，避免静默闪退）。
fn show_already_running_message() {
    let text = to_wide(
        "NovelReader 已经在运行，但没有找到它的窗口。\n\n\
         请稍等片刻再试；如果仍然没有窗口，请在任务管理器中结束 NovelReader 进程后重新启动。",
    );
    let caption = to_wide("NovelReader");
    unsafe {
        MessageBoxW(0, text.as_ptr(), caption.as_ptr(), MB_TOPMOST);
    }
}

// ===== Tauri 命令（薄包装 shared 层） =====

#[tauri::command]
fn get_books(app: tauri::AppHandle) -> Vec<shared::Book> {
    let _ = app;
    shared::get_books(&shared::data_root())
}

#[tauri::command]
fn get_settings(app: tauri::AppHandle) -> shared::Settings {
    let _ = app;
    shared::get_settings(&shared::data_root())
}

#[tauri::command]
fn save_settings(app: tauri::AppHandle, lastBookId: Option<String>, theme: Option<String>, fontSize: Option<u32>, lineHeight: Option<f64>) {
    let _ = app;
    shared::save_settings(&shared::data_root(), lastBookId, theme, fontSize, lineHeight)
}

#[tauri::command]
fn import_book(app: tauri::AppHandle, name: String, content: String, overwrite: bool) -> Result<shared::ImportResult, String> {
    let _ = app;
    shared::import_book_with_overwrite(&shared::data_root(), name, &content, overwrite)
}

#[tauri::command]
fn open_saved(app: tauri::AppHandle, id: String) -> Result<shared::OpenSavedResult, String> {
    let _ = app;
    shared::open_saved(&shared::data_root(), &id)
}

#[tauri::command]
fn update_book(app: tauri::AppHandle, id: String, lastChapter: Option<usize>, lastPosition: Option<f64>) -> Result<(), String> {
    let _ = app;
    shared::update_book(&shared::data_root(), &id, lastChapter, lastPosition)
}

#[tauri::command]
fn delete_book(app: tauri::AppHandle, id: String) -> Result<(), String> {
    let _ = app;
    shared::delete_book(&shared::data_root(), &id)
}

#[tauri::command]
fn rename_book(app: tauri::AppHandle, id: String, newName: String) -> Result<(), String> {
    let _ = app;
    shared::rename_book(&shared::data_root(), &id, &newName)
}

#[tauri::command]
fn set_window_frame(app: tauri::AppHandle, theme: String) {
    use tauri::Manager;
    if let Some(win) = app.get_webview_window("main") {
        apply_window_frame(&win, theme == "night");
    }
}

fn apply_window_frame(win: &tauri::WebviewWindow, night: bool) {
    let (r, g, b) = if night {
        (26u8, 26u8, 26u8)
    } else {
        (245u8, 240u8, 232u8)
    };
    let _ = win.set_background_color(Some(tauri::window::Color::from((r, g, b, 255u8))));
    let _ = win.set_theme(Some(if night {
        tauri::Theme::Dark
    } else {
        tauri::Theme::Light
    }));
    #[cfg(windows)]
    {
        if let Ok(hwnd) = win.hwnd() {
            apply_dwm_titlebar(hwnd.0 as isize, night);
        }
    }
}

#[cfg(windows)]
#[link(name = "dwmapi")]
extern "system" {
    fn DwmSetWindowAttribute(
        hwnd: isize,
        attribute: u32,
        value: *const std::ffi::c_void,
        size: u32,
    ) -> i32;
}

#[cfg(windows)]
fn apply_dwm_titlebar(hwnd: isize, night: bool) {
    const DWMWA_USE_IMMERSIVE_DARK_MODE: u32 = 20;
    const DWMWA_BORDER_COLOR: u32 = 34;
    const DWMWA_CAPTION_COLOR: u32 = 35;
    const DWMWA_TEXT_COLOR: u32 = 36;

    let caption = if night { 0x001B1B1B_u32 } else { 0x00E6EEF3_u32 };
    let text = if night { 0x00CCCCCC_u32 } else { 0x0038424A_u32 };
    let border = if night { 0x00333333_u32 } else { 0x00CFDBE3_u32 };
    let dark_mode = if night { 1_u32 } else { 0_u32 };

    unsafe fn set(hwnd: isize, attribute: u32, value: u32) {
        unsafe {
            let _ = DwmSetWindowAttribute(
                hwnd,
                attribute,
                &value as *const u32 as *const std::ffi::c_void,
                std::mem::size_of::<u32>() as u32,
            );
        }
    }
    unsafe {
        set(hwnd, DWMWA_USE_IMMERSIVE_DARK_MODE, dark_mode);
        set(hwnd, DWMWA_CAPTION_COLOR, caption);
        set(hwnd, DWMWA_TEXT_COLOR, text);
        set(hwnd, DWMWA_BORDER_COLOR, border);
    }
}

#[tauri::command]
fn store_info(bookId: String) -> Result<fanqie::BookDetail, String> {
    fanqie::book_info(&bookId)
}

#[tauri::command]
fn store_chapters(bookId: String) -> Result<Vec<fanqie::ChapterItem>, String> {
    fanqie::chapters(&bookId)
}

#[tauri::command]
fn store_contents(bookId: String, itemIds: Vec<String>) -> Result<std::collections::HashMap<String, String>, String> {
    fanqie::fetch_contents(&itemIds, &bookId)
}

#[tauri::command]
fn store_download_contents(bookId: String, itemIds: Vec<String>) -> Result<std::collections::HashMap<String, String>, String> {
    fanqie::fetch_contents_download(&itemIds, &bookId)
}

#[tauri::command]
fn add_store_book(storeId: String, name: String, replace: bool) -> Result<shared::Book, String> {
    shared::add_store_book_with_replace(&shared::data_root(), &storeId, &name, replace)
}

#[tauri::command]
fn get_store_progress(storeId: String) -> Option<shared::StoreProgress> {
    shared::get_store_progress(&shared::data_root(), &storeId)
}

#[tauri::command]
fn save_store_progress(storeId: String, lastChapter: usize, lastPosition: f64, name: Option<String>, lastItemId: Option<String>) -> Result<(), String> {
    shared::save_store_progress(
        &shared::data_root(),
        &storeId,
        lastChapter,
        lastPosition,
        name.as_deref(),
        lastItemId.as_deref(),
    )
}

#[tauri::command]
fn save_txt_dialog(app: tauri::AppHandle, fileName: String, content: String) -> Result<Option<String>, String> {
    use tauri_plugin_dialog::DialogExt;
    let picked = app.dialog().file().set_file_name(fileName).blocking_save_file();
    match picked {
        None => Ok(None),
        Some(path) => {
            let p = path.into_path().map_err(|e| format!("保存路径无效：{:?}", e))?;
            std::fs::write(&p, content).map_err(|e| format!("保存文件失败：{}", e))?;
            Ok(Some(p.to_string_lossy().to_string()))
        }
    }
}

#[tauri::command]
fn cover_jpeg(url: String) -> Result<String, String> {
    use base64::Engine as _;
    let bytes = fanqie::cover_jpeg(&url)?;
    Ok(format!(
        "data:image/jpeg;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(bytes)
    ))
}

#[tauri::command]
fn cover_batch(urls: Vec<String>) -> Result<Vec<Option<String>>, String> {
    use base64::Engine as _;
    Ok(fanqie::cover_batch(&urls)
        .into_iter()
        .map(|bytes| {
            bytes.map(|bytes| {
                format!(
                    "data:image/jpeg;base64,{}",
                    base64::engine::general_purpose::STANDARD.encode(bytes)
                )
            })
        })
        .collect())
}

#[tauri::command]
fn rank_books(algo: String) -> Result<Vec<fanqie::RankBook>, String> {
    fanqie::rank_books(&algo)
}

#[tauri::command]
fn search_store(query: String) -> Result<Vec<shared::StoreBook>, String> {
    fanqie::search_app(&query)
}

/// 前端在关窗钩子里保存完进度后调用，真正退出应用。
#[tauri::command]
fn quit_app(app: tauri::AppHandle) {
    log_startup("quit_app invoked; exiting");
    app.exit(0);
}

/// WebView2 使用自己的数据目录：默认的 `%LOCALAPPDATA%\{identifier}` 一旦被锁或损坏，
/// 创建 webview 会直接失败（窗口一闪而过）。改用独立目录避免受其影响。
fn webview_data_dir() -> std::path::PathBuf {
    // 不复用默认的 %LOCALAPPDATA%\{identifier}：它一旦被残留进程锁住或损坏，
    // WebView2 创建会失败（0x800700AA），表现为窗口一闪而过。
    let p = shared::data_root().join("webview2");
    std::fs::create_dir_all(&p).ok();
    p
}

/// 启动诊断日志：即使控制台一闪而过，也能在 logs 目录里找到原因。
fn log_startup(msg: &str) {
    use std::io::Write;
    let dir = shared::data_root().join("logs");
    std::fs::create_dir_all(&dir).ok();
    let path = dir.join("NovelReader-startup.log");
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(f, "[{}] {}", ts, msg);
    }
}

/// 把 `log` crate（Tauri/wry 用它报告 WebView2 错误）也写进启动日志。
struct StartupLogger;

impl log::Log for StartupLogger {
    fn enabled(&self, _metadata: &log::Metadata) -> bool {
        true
    }

    fn log(&self, record: &log::Record) {
        log_startup(&format!("[{}] {}: {}", record.level(), record.target(), record.args()));
    }

    fn flush(&self) {}
}

static STARTUP_LOGGER: StartupLogger = StartupLogger;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    std::panic::set_hook(Box::new(|info| {
        log_startup(&format!("PANIC: {}", info));
        eprintln!("PANIC: {}", info);
    }));
    let _ = log::set_logger(&STARTUP_LOGGER);
    log::set_max_level(log::LevelFilter::Info);
    log_startup("run() start");

    // 单实例锁：已有实例就激活它并退出；找不到窗口时提示后退出，
    // 绝不再启动第二个实例（两个实例会抢同一个 WebView2 配置目录）。
    let single = check_single_instance();
    log_startup(&format!("single_instance={}", single));
    if single {
        // 旧实例可能正在创建窗口，最多等 5 秒。
        for _ in 0..25 {
            if activate_existing_window() {
                log_startup("activated existing window, exiting");
                std::process::exit(0);
            }
            std::thread::sleep(Duration::from_millis(200));
        }
        log_startup("existing instance has no window; showing notice and exiting");
        show_already_running_message();
        std::process::exit(0);
    }

    let data_root = shared::data_root();
    log_startup(&format!("data_root={}", data_root.display()));

    let result = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .setup(move |app| {
            log_startup("setup start");
            shared::ensure_data(&shared::data_root());

            // 根据已保存主题创建窗口，避免 resize 时 WebView2 白闪
            let settings = shared::get_settings(&shared::data_root());
            log_startup(&format!("theme={}", settings.theme));
            let built = tauri::WebviewWindowBuilder::new(app, "main", tauri::WebviewUrl::App("index.html".into()))
                .title("NovelReader")
                .inner_size(1200.0, 800.0)
                .min_inner_size(350.0, 400.0)
                .resizable(true)
                .fullscreen(false)
                .data_directory(webview_data_dir())
                .background_color({
                    let c = if settings.theme == "night" { (26u8, 26u8, 26u8, 255u8) } else { (245u8, 240u8, 232u8, 255u8) };
                    tauri::window::Color::from(c)
                })
                .build();
            let win = match built {
                Ok(w) => {
                    log_startup("window built");
                    w
                }
                Err(e) => {
                    log_startup(&format!("window build error: {}", e));
                    return Err(e.into());
                }
            };
            win.on_window_event(|event| log_startup(&format!("window_event: {:?}", event)));
            apply_window_frame(&win, settings.theme == "night");

            log_startup("setup done");
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
            set_window_frame,
            search_store,
            store_info,
            store_chapters,
            store_contents,
            store_download_contents,
            rank_books,
            cover_jpeg,
            cover_batch,
            add_store_book,
            get_store_progress,
            save_store_progress,
            save_txt_dialog,
            quit_app,
        ])
        .run(tauri::generate_context!());

    match result {
        Ok(()) => log_startup("run exited normally"),
        Err(e) => {
            log_startup(&format!("run error: {}", e));
            eprintln!("NovelReader 启动失败: {}", e);
        }
    }
}
