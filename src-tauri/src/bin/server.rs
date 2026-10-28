// 独立 HTTP 服务端，替代 Node.js server.js
// 与 Tauri 共用 shared.rs 业务逻辑
// 编译: cargo build --bin server
#[path = "../shared.rs"]
mod shared;
#[path = "../fanqie.rs"]
mod fanqie;
#[path = "../heif.rs"]
mod heif;

use std::fs;
use std::path::{Path, PathBuf};
use tiny_http::{Header, Method, Request, Response};

const PORT: u16 = 2048;

fn main() {
    let server = tiny_http::Server::http(format!("0.0.0.0:{}", PORT)).unwrap();
    let frontend = find_frontend();
    let base = shared::data_root();

    println!("小说阅读器 HTTP 服务已启动: http://localhost:{}", PORT);
    println!("前端目录: {}", frontend.display());
    println!("数据目录: {}", base.display());

    // 自动打开浏览器（桌面端一键启动脚本会传 --no-open）
    if !std::env::args().any(|a| a == "--no-open") {
        let _ = std::process::Command::new("cmd")
            .args(["/c", "start", &format!("http://localhost:{}", PORT)])
            .spawn();
    }

    for request in server.incoming_requests() {
        let url = request.url().to_string();
        let url_path = url.split('?').next().unwrap_or(&url);
        let method = request.method().clone();

        match (method, url_path) {
            (Method::Get, "/api/books") => json_response(request, &shared::get_books(&base)),
            (Method::Get, "/api/settings") => json_response(request, &shared::get_settings(&base)),
            (Method::Post, "/api/settings") => handle_save_settings(request, &base),
            (Method::Post, "/api/import-book") => handle_import_book(request, &base),
            (Method::Post, "/api/open-saved") => handle_open_saved(request, &base),
            (Method::Post, "/api/store/search") => handle_store_search(request),
            (Method::Post, "/api/store/info") => handle_store_info(request),
            (Method::Post, "/api/store/chapters") => handle_store_chapters(request),
            (Method::Post, "/api/store/contents") => handle_store_contents(request),
            (Method::Post, "/api/store/download-contents") => handle_store_download_contents(request),
    (Method::Post, "/api/store/rank") => handle_store_rank(request),
    (Method::Post, "/api/cover") => handle_cover(request),
    (Method::Post, "/api/cover-batch") => handle_cover_batch(request),
            (Method::Post, "/api/store/add-book") => handle_add_store_book(request, &base),
            (Method::Post, "/api/store/get-progress") => handle_get_store_progress(request, &base),
            (Method::Post, "/api/store/save-progress") => handle_save_store_progress(request, &base),
            (Method::Post, "/api/update-book") => handle_update_book(request, &base),
            (Method::Post, "/api/delete-book") => handle_delete_book(request, &base),
            (Method::Post, "/api/rename-book") => handle_rename_book(request, &base),
            _ => serve_static(request, &frontend),
        }
    }
}

// ===== 工具 =====

fn find_frontend() -> PathBuf {
    let mut dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."));
    for _ in 0..4 {
        let f = dir.join("frontend").join("index.html");
        if f.exists() {
            return dir.join("frontend");
        }
        if let Some(parent) = dir.parent() {
            dir = parent.to_path_buf();
        } else {
            break;
        }
    }
    PathBuf::from("frontend")
}

fn json_response<T: serde::Serialize>(request: Request, data: &T) {
    let json = serde_json::to_string(data).unwrap_or_default();
    let _ = request.respond(
        Response::from_string(json)
            .with_header(hdr("Content-Type: application/json; charset=utf-8"))
            .with_header(hdr("Cache-Control: no-cache")),
    );
}

fn json_ok(request: Request) {
    json_response(request, &serde_json::json!({"ok": true}));
}

fn json_error(request: Request, msg: &str, status: u16) {
    let json = serde_json::json!({"error": msg}).to_string();
    let _ = request.respond(
        Response::from_string(json)
            .with_status_code(status)
            .with_header(hdr("Content-Type: application/json; charset=utf-8")),
    );
}

fn hdr(s: &str) -> Header {
    let (name, value) = s.split_once(": ").unwrap_or((s, ""));
    Header::from_bytes(name.as_bytes(), value.as_bytes()).unwrap()
}

fn read_body(request: &mut Request) -> String {
    let mut body = String::new();
    request.as_reader().read_to_string(&mut body).ok();
    body
}

fn mime_type(ext: &str) -> &'static str {
    match ext {
        "html" => "text/html; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "js" => "application/javascript; charset=utf-8",
        "json" => "application/json",
        _ => "text/plain",
    }
}

// ===== 静态文件 =====

fn serve_static(request: Request, frontend: &PathBuf) {
    let url = request.url().to_string();
    let path_str = if url == "/" {
        "/index.html"
    } else {
        let p = url.split('?').next().unwrap_or(&url);
        p
    };

    let file_path = match safe_join(frontend, path_str) {
        Some(p) => p,
        None => {
            let _ = request.respond(Response::from_string("Not Found").with_status_code(404));
            return;
        }
    };

    // SPA 回退
    let file_path = if file_path.exists() {
        file_path
    } else {
        frontend.join("index.html")
    };

    // 主题注入（index.html）
    if file_path.ends_with("index.html") {
        let mut content = fs::read_to_string(&file_path).unwrap_or_default();
        if shared::get_settings(&shared::data_root()).theme == "night" {
            content = content.replace("<html", "<html class=\" night\"");
        }
        let _ = request.respond(
            Response::from_string(content)
                .with_header(hdr("Content-Type: text/html; charset=utf-8"))
                .with_header(hdr("Cache-Control: no-cache")),
        );
        return;
    }

    match fs::read(&file_path) {
        Ok(data) => {
            let ext = file_path.extension().and_then(|e| e.to_str()).unwrap_or("");
            let _ = request.respond(
                Response::from_data(data)
                    .with_header(hdr(&format!("Content-Type: {}", mime_type(ext))))
                    .with_header(hdr("Cache-Control: no-cache")),
            );
        }
        Err(_) => {
            let _ = request.respond(Response::from_string("Not Found").with_status_code(404));
        }
    }
}

/// 只允许访问前端目录内的普通相对路径，拒绝 `..`、盘符和反斜杠。
fn safe_join(root: &Path, url_path: &str) -> Option<PathBuf> {
    let mut out = root.to_path_buf();
    for part in url_path.split('/') {
        match part {
            "" | "." => continue,
            ".." => return None,
            p if p.contains('\\') || p.contains(':') || p.contains('\0') => return None,
            p => out.push(p),
        }
    }
    Some(out)
}

// ===== API 处理 =====

#[derive(serde::Deserialize)]
struct SaveSettingsBody {
    #[serde(rename = "lastBookId")]
    last_book_id: Option<String>,
    theme: Option<String>,
    #[serde(rename = "fontSize")]
    font_size: Option<u32>,
    #[serde(rename = "lineHeight")]
    line_height: Option<f64>,
}

fn handle_save_settings(mut request: Request, base: &PathBuf) {
    let body = read_body(&mut request);
    if let Ok(b) = serde_json::from_str::<SaveSettingsBody>(&body) {
        shared::save_settings(base, b.last_book_id, b.theme, b.font_size, b.line_height);
        json_ok(request);
    } else {
        json_error(request, "无效请求", 400);
    }
}

#[derive(serde::Deserialize)]
struct ImportBody {
    name: String,
    content: String,
    #[serde(default)]
    overwrite: bool,
}

fn handle_import_book(mut request: Request, base: &PathBuf) {
    let body = read_body(&mut request);
    match serde_json::from_str::<ImportBody>(&body) {
        Ok(b) => match shared::import_book_with_overwrite(base, b.name, &b.content, b.overwrite) {
            Ok(result) => json_response(request, &result),
            Err(e) => {
                let code = if e == "同名本地书已存在" { 409 } else { 400 };
                json_error(request, &e, code);
            }
        },
        Err(_) => json_error(request, "无效请求", 400),
    }
}

#[derive(serde::Deserialize)]
struct IdBody {
    id: String,
}

fn handle_open_saved(mut request: Request, base: &PathBuf) {
    let body = read_body(&mut request);
    if let Ok(b) = serde_json::from_str::<IdBody>(&body) {
        match shared::open_saved(base, &b.id) {
            Ok(result) => json_response(request, &result),
            Err(e) => json_error(request, &e, 400),
        }
    } else {
        json_error(request, "无效请求", 400);
    }
}

#[derive(serde::Deserialize)]
struct UpdateBookBody {
    id: String,
    #[serde(rename = "lastChapter")]
    last_chapter: Option<usize>,
    #[serde(rename = "lastPosition")]
    last_position: Option<f64>,
}

fn handle_update_book(mut request: Request, base: &PathBuf) {
    let body = read_body(&mut request);
    if let Ok(b) = serde_json::from_str::<UpdateBookBody>(&body) {
        let _ = shared::update_book(base, &b.id, b.last_chapter, b.last_position);
        json_ok(request);
    } else {
        json_error(request, "无效请求", 400);
    }
}

fn handle_delete_book(mut request: Request, base: &PathBuf) {
    let body = read_body(&mut request);
    if let Ok(b) = serde_json::from_str::<IdBody>(&body) {
        let _ = shared::delete_book(base, &b.id);
        json_ok(request);
    } else {
        json_error(request, "无效请求", 400);
    }
}

#[derive(serde::Deserialize)]
struct RenameBody {
    id: String,
    #[serde(rename = "newName")]
    new_name: String,
}

fn handle_rename_book(mut request: Request, base: &PathBuf) {
    let body = read_body(&mut request);
    if let Ok(b) = serde_json::from_str::<RenameBody>(&body) {
        match shared::rename_book(base, &b.id, &b.new_name) {
            Ok(()) => json_ok(request),
            Err(e) => {
                let code = if e.contains("同名") { 409 } else { 400 };
                json_error(request, &e, code);
            }
        }
    } else {
        json_error(request, "无效请求", 400);
    }
}

#[derive(serde::Deserialize)]
struct StoreSearchBody {
    query: String,
}

fn handle_store_search(mut request: Request) {
    let body = read_body(&mut request);
    if let Ok(b) = serde_json::from_str::<StoreSearchBody>(&body) {
        match fanqie::search_app(&b.query) {
            Ok(list) => json_response(request, &list),
            Err(e) => json_error(request, &e, 502),
        }
    } else {
        json_error(request, "无效请求", 400);
    }
}

#[derive(serde::Deserialize)]
struct StoreIdBody {
    #[serde(rename = "bookId")]
    book_id: String,
}

fn handle_store_info(mut request: Request) {
    let body = read_body(&mut request);
    if let Ok(b) = serde_json::from_str::<StoreIdBody>(&body) {
        match fanqie::book_info(&b.book_id) {
            Ok(info) => json_response(request, &info),
            Err(e) => json_error(request, &e, 502),
        }
    } else {
        json_error(request, "无效请求", 400);
    }
}

fn handle_store_chapters(mut request: Request) {
    let body = read_body(&mut request);
    if let Ok(b) = serde_json::from_str::<StoreIdBody>(&body) {
        match fanqie::chapters(&b.book_id) {
            Ok(list) => json_response(request, &list),
            Err(e) => json_error(request, &e, 502),
        }
    } else {
        json_error(request, "无效请求", 400);
    }
}

#[derive(serde::Deserialize)]
struct StoreContentsBody {
    #[serde(rename = "bookId")]
    book_id: String,
    #[serde(rename = "itemIds")]
    item_ids: Vec<String>,
}

fn handle_store_contents(mut request: Request) {
    let body = read_body(&mut request);
    if let Ok(b) = serde_json::from_str::<StoreContentsBody>(&body) {
        match fanqie::fetch_contents(&b.item_ids, &b.book_id) {
            Ok(map) => json_response(request, &map),
            Err(e) => json_error(request, &e, 502),
        }
    } else {
        json_error(request, "无效请求", 400);
    }
}

fn handle_store_download_contents(mut request: Request) {
    let body = read_body(&mut request);
    if let Ok(b) = serde_json::from_str::<StoreContentsBody>(&body) {
        match fanqie::fetch_contents_download(&b.item_ids, &b.book_id) {
            Ok(map) => json_response(request, &map),
            Err(e) => json_error(request, &e, 502),
        }
    } else {
        json_error(request, "无效请求", 400);
    }
}

#[derive(serde::Deserialize)]
struct CoverBody {
    url: String,
}

fn handle_cover(mut request: Request) {
    use base64::Engine as _;
    let body = read_body(&mut request);
    if let Ok(b) = serde_json::from_str::<CoverBody>(&body) {
        match fanqie::cover_jpeg(&b.url) {
            Ok(bytes) => {
                let data = format!(
                    "data:image/jpeg;base64,{}",
                    base64::engine::general_purpose::STANDARD.encode(bytes)
                );
                json_response(request, &data);
            }
            Err(e) => json_error(request, &e, 502),
        }
    } else {
        json_error(request, "无效请求", 400);
    }
}

#[derive(serde::Deserialize)]
struct CoverBatchBody {
    urls: Vec<String>,
}

fn handle_cover_batch(mut request: Request) {
    use base64::Engine as _;
    let body = read_body(&mut request);
    if let Ok(b) = serde_json::from_str::<CoverBatchBody>(&body) {
        let covers: Vec<Option<String>> = fanqie::cover_batch(&b.urls)
            .into_iter()
            .map(|bytes| {
                bytes.map(|bytes| {
                    format!(
                        "data:image/jpeg;base64,{}",
                        base64::engine::general_purpose::STANDARD.encode(bytes)
                    )
                })
            })
            .collect();
        json_response(request, &covers);
    } else {
        json_error(request, "无效请求", 400);
    }
}

#[derive(serde::Deserialize)]
struct StoreRankBody {
    algo: String,
}

fn handle_store_rank(mut request: Request) {
    let body = read_body(&mut request);
    if let Ok(b) = serde_json::from_str::<StoreRankBody>(&body) {
        match fanqie::rank_books(&b.algo) {
            Ok(list) => json_response(request, &list),
            Err(e) => json_error(request, &e, 502),
        }
    } else {
        json_error(request, "无效请求", 400);
    }
}

#[derive(serde::Deserialize)]
struct AddStoreBookBody {
    #[serde(rename = "storeId")]
    store_id: String,
    name: String,
    #[serde(default)]
    replace: bool,
}

fn handle_add_store_book(mut request: Request, base: &PathBuf) {
    let body = read_body(&mut request);
    if let Ok(b) = serde_json::from_str::<AddStoreBookBody>(&body) {
        match shared::add_store_book_with_replace(base, &b.store_id, &b.name, b.replace) {
            Ok(book) => json_response(request, &book),
            Err(e) => json_error(request, &e, 409),
        }
    } else {
        json_error(request, "无效请求", 400);
    }
}

#[derive(serde::Deserialize)]
struct StoreProgressIdBody {
    #[serde(rename = "storeId")]
    store_id: String,
}

fn handle_get_store_progress(mut request: Request, base: &PathBuf) {
    let body = read_body(&mut request);
    if let Ok(b) = serde_json::from_str::<StoreProgressIdBody>(&body) {
        json_response(request, &shared::get_store_progress(base, &b.store_id))
    } else {
        json_error(request, "无效请求", 400);
    }
}

#[derive(serde::Deserialize)]
struct SaveStoreProgressBody {
    #[serde(rename = "storeId")]
    store_id: String,
    #[serde(rename = "lastChapter")]
    last_chapter: usize,
    #[serde(rename = "lastPosition")]
    last_position: f64,
    #[serde(default)]
    name: Option<String>,
    #[serde(rename = "lastItemId", default)]
    last_item_id: Option<String>,
}

fn handle_save_store_progress(mut request: Request, base: &PathBuf) {
    let body = read_body(&mut request);
    if let Ok(b) = serde_json::from_str::<SaveStoreProgressBody>(&body) {
        match shared::save_store_progress(
            base,
            &b.store_id,
            b.last_chapter,
            b.last_position,
            b.name.as_deref(),
            b.last_item_id.as_deref(),
        ) {
            Ok(()) => json_ok(request),
            Err(e) => json_error(request, &e, 500),
        }
    } else {
        json_error(request, "无效请求", 400);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn safe_join_blocks_traversal() {
        let root = Path::new("C:\\app\\frontend");
        assert_eq!(safe_join(root, "/../books.json"), None);
        assert_eq!(safe_join(root, "/a/../../b"), None);
        assert_eq!(safe_join(root, "/..\\books.json"), None);
        assert_eq!(safe_join(root, "/C:/Windows/win.ini"), None);
    }

    #[test]
    fn safe_join_allows_normal_paths() {
        let root = Path::new("C:\\app\\frontend");
        assert_eq!(safe_join(root, "/index.html"), Some(root.join("index.html")));
        assert_eq!(safe_join(root, "/sub/app.js"), Some(root.join("sub").join("app.js")));
        assert_eq!(safe_join(root, "/"), Some(root.to_path_buf()));
    }
}
