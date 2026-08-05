// 独立 HTTP 服务端，替代 Node.js server.js
// 与 Tauri 共用 shared.rs 业务逻辑
// 编译: cargo build --bin server
#[path = "../shared.rs"]
mod shared;

use std::fs;
use std::path::PathBuf;
use tiny_http::{Header, Method, Request, Response};

const PORT: u16 = 2048;

fn main() {
    let server = tiny_http::Server::http(format!("0.0.0.0:{}", PORT)).unwrap();
    let frontend = find_frontend();
    let base = shared::base_dir();

    println!("小说阅读器 HTTP 服务已启动: http://localhost:{}", PORT);
    println!("前端目录: {}", frontend.display());
    println!("数据目录: {}", base.display());

    // 自动打开浏览器
    let _ = std::process::Command::new("cmd")
        .args(["/c", "start", &format!("http://localhost:{}", PORT)])
        .spawn();

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

    let file_path = if path_str.starts_with('/') {
        frontend.join(&path_str[1..])
    } else {
        frontend.join(path_str)
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
        if shared::get_settings(&shared::base_dir()).theme == "night" {
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
}

fn handle_import_book(mut request: Request, base: &PathBuf) {
    let body = read_body(&mut request);
    match serde_json::from_str::<ImportBody>(&body) {
        Ok(b) => match shared::import_book(base, b.name, &b.content) {
            Ok(result) => json_response(request, &result),
            Err(e) => {
                let code = if e == "同名书籍已存在" { 409 } else { 400 };
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
    #[serde(rename = "fontSize")]
    font_size: Option<u32>,
    #[serde(rename = "lineHeight")]
    line_height: Option<f64>,
    theme: Option<String>,
}

fn handle_update_book(mut request: Request, base: &PathBuf) {
    let body = read_body(&mut request);
    if let Ok(b) = serde_json::from_str::<UpdateBookBody>(&body) {
        let _ = shared::update_book(
            base, &b.id, b.last_chapter, b.last_position,
            b.font_size, b.line_height, b.theme,
        );
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
                let code = if e == "同名书籍已存在" { 409 } else { 400 };
                json_error(request, &e, code);
            }
        }
    } else {
        json_error(request, "无效请求", 400);
    }
}
