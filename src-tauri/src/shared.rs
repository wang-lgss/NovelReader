// 共享业务逻辑，不依赖 Tauri，浏览器和 Tauri 模式共用
#![allow(non_snake_case)]
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use regex::Regex;

const MAX_CHUNK_CHARS: usize = 250000;

// ===== 数据结构 =====

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Book {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub last_chapter: usize,
    #[serde(default)]
    pub last_position: f64,
    #[serde(default)]
    pub added_at: String,
    /// 来自番茄书城的书：存 store bookId，用于在线阅读和进度同步。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub store_id: Option<String>,
}

fn default_font_size() -> u32 { 17 }
fn default_line_height() -> f64 { 2.0 }
fn default_theme() -> String { "day".into() }

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BooksData {
    pub books: Vec<Book>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct StoreProgress {
    #[serde(default)]
    pub last_chapter: usize,
    #[serde(default)]
    pub last_position: f64,
    #[serde(default)]
    pub last_item_id: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct OnlineBook {
    pub store_id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub last_chapter: usize,
    #[serde(default)]
    pub last_position: f64,
    #[serde(default)]
    pub last_item_id: String,
    #[serde(default)]
    pub in_shelf: bool,
    #[serde(default)]
    pub added_at: String,
    #[serde(default)]
    pub last_read_at: String,
}

#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct OnlineBooksData {
    #[serde(default)]
    pub books: Vec<OnlineBook>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct StoreProgressData {
    #[serde(default)]
    pub books: HashMap<String, StoreProgress>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    #[serde(default)]
    pub last_book_id: String,
    #[serde(default = "default_theme")]
    pub theme: String,
    #[serde(default = "default_font_size")]
    pub font_size: u32,
    #[serde(default = "default_line_height")]
    pub line_height: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct StoreBook {
    pub id: String,
    pub title: String,
    pub author: String,
    #[serde(rename = "abstract")]
    pub abstract_text: String,
    pub cover: String,
    pub score: String,
    pub category: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Chapter {
    pub title: String,
    pub start: usize,
    pub end: usize,
    pub is_standard: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenSavedResult {
    pub content: String,
    pub chapters: Vec<Chapter>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportResult {
    pub book: Book,
}

// ===== 路径工具 =====

pub fn base_dir() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."))
}

/// 数据根目录：桌面端与网页端共用同一份书架/设置。
///
/// 优先级：
/// 1. `NOVELREADER_DATA_DIR` 环境变量；
/// 2. 开发/源码布局：从 exe 所在目录向上查找同时包含 `package.json` 与 `src-tauri` 的目录；
/// 3. 否则使用 exe 所在目录（便携版，数据跟着程序走）。
pub fn data_root() -> PathBuf {
    if let Ok(dir) = std::env::var("NOVELREADER_DATA_DIR") {
        if !dir.trim().is_empty() {
            let p = PathBuf::from(dir);
            fs::create_dir_all(&p).ok();
            return p;
        }
    }
    discover_data_root(&base_dir())
}

fn discover_data_root(exe_dir: &Path) -> PathBuf {
    let mut dir = exe_dir;
    for _ in 0..4 {
        if is_workspace_root(dir) {
            return dir.to_path_buf();
        }
        match dir.parent() {
            Some(parent) => dir = parent,
            None => break,
        }
    }
    exe_dir.to_path_buf()
}

fn is_workspace_root(dir: &Path) -> bool {
    dir.join("package.json").is_file() && dir.join("src-tauri").is_dir()
}

/// 先写唯一临时文件再改名，避免两端同时写入时共用 `books.tmp` 互相覆盖。
fn write_atomic(path: &Path, contents: &str) -> std::io::Result<()> {
    use std::sync::atomic::{AtomicU64, Ordering};
    static SEQ: AtomicU64 = AtomicU64::new(0);
    let seq = SEQ.fetch_add(1, Ordering::Relaxed);
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("data");
    let tmp = path.with_file_name(format!(".{}.{}.{}.tmp", name, std::process::id(), seq));

    fs::write(&tmp, contents)?;
    match fs::rename(&tmp, path) {
        Ok(()) => Ok(()),
        Err(e) => {
            let _ = fs::remove_file(&tmp);
            Err(e)
        }
    }
}

pub fn books_dir(base: &Path) -> PathBuf {
    let dir = base.join("books");
    fs::create_dir_all(&dir).ok();
    dir
}

pub fn data_file(base: &Path) -> PathBuf {
    base.join("books.json")
}

pub fn settings_file(base: &Path) -> PathBuf {
    base.join("settings.json")
}

fn chapters_file(base: &Path, id: &str) -> PathBuf {
    // v2：章节边界改为 UTF-16 码元偏移，旧缓存（字符偏移）必须失效
    books_dir(base).join(format!("{}.chapters.v2.json", id))
}

pub fn ensure_data(base: &Path) {
    let f = data_file(base);
    if !f.exists() {
        fs::write(&f, "{\"books\":[]}").ok();
    }
    let s = settings_file(base);
    if !s.exists() {
        fs::write(&s, "{\"lastBookId\":\"\",\"theme\":\"day\",\"fontSize\":17,\"lineHeight\":2.0}").ok();
    }
}

// ===== 章节解析 =====

fn chapter_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"(?m)^\s*(第\s*[零一二三四五六七八九十百千万两\d]+\s*[章卷节]|楔子|序章|序言|终章|尾声|后记|番外|完本感言).*").unwrap()
    })
}

fn standard_chapter_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"^第\s*[零一二三四五六七八九十百千万两\d]+\s*章").unwrap())
}

fn is_standard_chapter(title: &str) -> bool {
    standard_chapter_re().is_match(title)
}

/// 构建 字节偏移 → 字符下标 的映射（含末尾边界）。
fn build_byte_to_char(content: &str) -> Vec<usize> {
    let mut v = Vec::with_capacity(content.chars().count() + 1);
    v.push(0);
    for (bi, _) in content.char_indices().skip(1) {
        v.push(bi);
    }
    v.push(content.len());
    v
}

fn to_char(byte_to_char: &[usize], byte_pos: usize) -> usize {
    match byte_to_char.binary_search(&byte_pos) {
        Ok(i) => i,
        Err(i) => i.saturating_sub(1),
    }
}

/// 把按字符下标计算的边界转换为 JS `String.prototype.slice` 使用的 UTF-16 码元偏移。
fn char_to_utf16_offsets(content: &str, chapters: &mut [Chapter], total_chars: usize) {
    let mut bounds: Vec<usize> = chapters
        .iter()
        .flat_map(|c| [c.start.min(total_chars), c.end.min(total_chars)])
        .collect();
    bounds.sort_unstable();
    bounds.dedup();

    let mut map = HashMap::with_capacity(bounds.len());
    let mut byte = 0usize;
    let mut chars_seen = 0usize;
    let mut utf16_seen = 0usize;
    for bound in bounds {
        while chars_seen < bound {
            let ch = content[byte..].chars().next().expect("章节边界必须落在字符边界上");
            byte += ch.len_utf8();
            chars_seen += 1;
            utf16_seen += ch.len_utf16();
        }
        map.insert(bound, utf16_seen);
    }

    for ch in chapters.iter_mut() {
        ch.start = map[&ch.start.min(total_chars)];
        ch.end = map[&ch.end.min(total_chars)];
    }
}

pub fn parse_chapters(content: &str) -> Vec<Chapter> {
    let re = chapter_re();
    let matches: Vec<_> = re.find_iter(content).collect();
    let total_chars = content.chars().count();
    let byte_to_char = build_byte_to_char(content);

    let mut raw = Vec::new();

    if matches.is_empty() {
        raw.push(Chapter { title: "正文".into(), start: 0, end: total_chars, is_standard: false });
        let mut chapters = chunk_large_chapters(content, &raw);
        char_to_utf16_offsets(content, &mut chapters, total_chars);
        return chapters;
    }

    if matches[0].start() > 0 {
        let end = to_char(&byte_to_char, matches[0].start());
        raw.push(Chapter { title: "前言".into(), start: 0, end, is_standard: false });
    }

    for i in 0..matches.len() {
        let m = matches[i];
        let title = m.as_str().trim().to_string();
        let title = if title.chars().count() > 50 { format!("{}…", title.chars().take(50).collect::<String>()) } else { title };
        let std = is_standard_chapter(&title);
        let start = to_char(&byte_to_char, m.end());
        let end = if i + 1 < matches.len() {
            to_char(&byte_to_char, matches[i + 1].start())
        } else {
            total_chars
        };
        raw.push(Chapter { title, start, end, is_standard: std });
    }

    let mut chapters = chunk_large_chapters(content, &raw);
    char_to_utf16_offsets(content, &mut chapters, total_chars);
    chapters
}

fn chunk_large_chapters(content: &str, chapters: &[Chapter]) -> Vec<Chapter> {
    let chars: Vec<char> = content.chars().collect();
    let mut result = Vec::new();
    for ch in chapters {
        let len = ch.end.saturating_sub(ch.start);
        if len <= MAX_CHUNK_CHARS {
            result.push(ch.clone());
            continue;
        }
        let total = (len + MAX_CHUNK_CHARS - 1) / MAX_CHUNK_CHARS;
        let mut offset = ch.start;
        let mut part = 1usize;
        while offset < ch.end {
            let mut end = std::cmp::min(offset + MAX_CHUNK_CHARS, ch.end);
            let mut next = end;
            if end < ch.end {
                // 在 [offset + MAX/2, end) 内向前找换行，避免把长段落从中间切断
                let lower = offset + MAX_CHUNK_CHARS / 2;
                let mut split = end;
                while split > lower {
                    if chars[split - 1] == '\n' {
                        end = split - 1;
                        next = split;
                        break;
                    }
                    split -= 1;
                }
            }
            let title = format!("{} ({}/{})", ch.title, part, total);
            result.push(Chapter { title, start: offset, end, is_standard: ch.is_standard && part == 1 });
            offset = next;
            part += 1;
        }
    }
    result
}

fn save_chapters(base: &Path, id: &str, chapters: &[Chapter]) {
    let json = serde_json::to_string(chapters).unwrap_or_default();
    let _ = write_atomic(&chapters_file(base, id), &json);
}

fn load_chapters(base: &Path, id: &str) -> Option<Vec<Chapter>> {
    let path = chapters_file(base, id);
    if !path.exists() { return None; }
    let content = fs::read_to_string(&path).ok()?;
    serde_json::from_str(&content).ok()
}

// ===== 业务逻辑 =====

fn load_books(base: &Path) -> BooksData {
    let content = fs::read_to_string(data_file(base)).unwrap_or_default();
    serde_json::from_str(&content).unwrap_or(BooksData { books: vec![] })
}

fn store_progress_file(base: &Path) -> PathBuf {
    base.join("store-progress.json")
}

fn online_books_file(base: &Path) -> PathBuf {
    base.join("online-books.json")
}

fn load_store_progress(base: &Path) -> StoreProgressData {
    fs::read_to_string(store_progress_file(base))
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn load_online_books(base: &Path) -> OnlineBooksData {
    fs::read_to_string(online_books_file(base))
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn write_online_books(base: &Path, data: &OnlineBooksData) -> Result<(), String> {
    write_atomic(
        &online_books_file(base),
        &serde_json::to_string_pretty(data).unwrap_or_default(),
    )
    .map_err(|e| format!("保存在线书籍失败：{}", e))
}

fn backup_once(src: &Path, dst: &Path) {
    if src.exists() && !dst.exists() {
        let _ = fs::copy(src, dst);
    }
}

pub fn ensure_online_books(base: &Path) {
    ensure_data(base);
    if online_books_file(base).exists() {
        return;
    }
    let old_books = load_books(base);
    let old_progress = load_store_progress(base);
    let mut online: Vec<OnlineBook> = Vec::new();

    for b in old_books.books.iter().filter(|b| b.store_id.is_some()) {
        let sid = b.store_id.clone().unwrap_or_default();
        if online.iter().any(|x| x.store_id == sid) {
            continue;
        }
        let progress = old_progress.books.get(&sid);
        online.push(OnlineBook {
            store_id: sid,
            name: b.name.clone(),
            last_chapter: progress.map(|p| p.last_chapter).unwrap_or(b.last_chapter),
            last_position: progress.map(|p| p.last_position).unwrap_or(b.last_position),
            last_item_id: progress.map(|p| p.last_item_id.clone()).unwrap_or_default(),
            in_shelf: true,
            added_at: b.added_at.clone(),
            last_read_at: String::new(),
        });
    }
    for (sid, p) in old_progress.books.iter() {
        if online.iter().any(|x| &x.store_id == sid) {
            continue;
        }
        online.push(OnlineBook {
            store_id: sid.clone(),
            name: String::new(),
            last_chapter: p.last_chapter,
            last_position: p.last_position,
            last_item_id: p.last_item_id.clone(),
            in_shelf: false,
            added_at: String::new(),
            last_read_at: String::new(),
        });
    }

    backup_once(&data_file(base), &base.join("books.pre-online-v1.json"));
    backup_once(&store_progress_file(base), &base.join("store-progress.pre-online-v1.json"));

    // 先写在线上，再移除 books.json 里的在线条目。
    if write_online_books(base, &OnlineBooksData { books: online }).is_err() {
        return;
    }
    let mut local_only = old_books;
    local_only.books.retain(|b| b.store_id.is_none());
    let _ = write_atomic(
        &data_file(base),
        &serde_json::to_string_pretty(&local_only).unwrap_or_default(),
    );
}

pub fn get_store_progress(base: &Path, store_id: &str) -> Option<StoreProgress> {
    ensure_online_books(base);
    let data = load_online_books(base);
    data.books.iter().find(|b| b.store_id == store_id).map(|b| StoreProgress {
        last_chapter: b.last_chapter,
        last_position: b.last_position,
        last_item_id: b.last_item_id.clone(),
    })
}

pub fn save_store_progress(
    base: &Path,
    store_id: &str,
    last_chapter: usize,
    last_position: f64,
    name: Option<&str>,
    last_item_id: Option<&str>,
) -> Result<(), String> {
    ensure_online_books(base);
    let mut data = load_online_books(base);
    let title = name.map(str::trim).unwrap_or("").to_string();
    if let Some(book) = data.books.iter_mut().find(|b| b.store_id == store_id) {
        book.last_chapter = last_chapter;
        book.last_position = last_position;
        if !title.is_empty() {
            book.name = title;
        }
        if let Some(item_id) = last_item_id {
            if !item_id.is_empty() {
                book.last_item_id = item_id.to_string();
            }
        }
        book.last_read_at = chrono_now();
    } else {
        data.books.push(OnlineBook {
            store_id: store_id.to_string(),
            name: if title.is_empty() { "未命名".into() } else { title },
            last_chapter,
            last_position,
            last_item_id: last_item_id.unwrap_or("").to_string(),
            in_shelf: false,
            added_at: String::new(),
            last_read_at: chrono_now(),
        });
    }
    write_online_books(base, &data)
}

/// 把番茄书城的书加入书架（只存元数据，正文在线阅读）。
pub fn add_store_book(base: &Path, store_id: &str, name: &str) -> Result<Book, String> {
    add_store_book_with_replace(base, store_id, name, false)
}

pub fn add_store_book_with_replace(
    base: &Path,
    store_id: &str,
    name: &str,
    replace: bool,
) -> Result<Book, String> {
    ensure_online_books(base);
    let mut data = load_online_books(base);
    let title = {
        let t = name.trim();
        if t.is_empty() { "未命名".to_string() } else { t.to_string() }
    };

    if let Some(existing) = data.books.iter().find(|b| b.store_id == store_id) {
        if existing.in_shelf {
            return Err("该书已在书架".into());
        }
    }

    let same_online: Vec<usize> = data
        .books
        .iter()
        .enumerate()
        .filter(|(_, b)| b.in_shelf && b.store_id != store_id && b.name == title)
        .map(|(i, _)| i)
        .collect();
    if !same_online.is_empty() {
        if !replace {
            return Err("同名在线书已存在".into());
        }
        for i in same_online {
            data.books[i].in_shelf = false;
        }
    }

    let now = chrono_now();
    let index = if let Some(i) = data.books.iter().position(|b| b.store_id == store_id) {
        let book = &mut data.books[i];
        book.name = title;
        book.in_shelf = true;
        if book.added_at.is_empty() {
            book.added_at = now.clone();
        }
        i
    } else {
        data.books.push(OnlineBook {
            store_id: store_id.to_string(),
            name: title,
            last_chapter: 0,
            last_position: 0.0,
            last_item_id: String::new(),
            in_shelf: true,
            added_at: now,
            last_read_at: String::new(),
        });
        data.books.len() - 1
    };
    let book = data.books[index].clone();
    write_online_books(base, &data)?;
    Ok(Book {
        id: format!("store-{}", book.store_id),
        name: book.name,
        last_chapter: book.last_chapter,
        last_position: book.last_position,
        added_at: book.added_at,
        store_id: Some(book.store_id),
    })
}

pub fn get_books(base: &Path) -> Vec<Book> {
    ensure_online_books(base);
    let mut books = load_books(base).books;
    books.retain(|b| b.store_id.is_none());
    let online = load_online_books(base);
    for b in online.books.into_iter().filter(|b| b.in_shelf) {
        books.push(Book {
            id: format!("store-{}", b.store_id),
            name: if b.name.is_empty() { "未命名".into() } else { b.name },
            last_chapter: b.last_chapter,
            last_position: b.last_position,
            added_at: b.added_at,
            store_id: Some(b.store_id),
        });
    }
    books
}

pub fn get_settings(base: &Path) -> Settings {
    let content = fs::read_to_string(settings_file(base)).unwrap_or_default();
    serde_json::from_str(&content).unwrap_or(Settings {
        last_book_id: String::new(),
        theme: "day".into(),
        font_size: 17,
        line_height: 2.0,
    })
}

pub fn save_settings(base: &Path, last_book_id: Option<String>, theme: Option<String>, font_size: Option<u32>, line_height: Option<f64>) {
    // 只覆盖显式传入的字段，避免“只改主题”把字号/行距重置回默认值
    let mut s = get_settings(base);
    if let Some(v) = last_book_id { s.last_book_id = v; }
    if let Some(v) = theme { s.theme = v; }
    if let Some(v) = font_size { s.font_size = v; }
    if let Some(v) = line_height { s.line_height = v; }
    write_atomic(&settings_file(base), &serde_json::to_string_pretty(&s).unwrap_or_default()).ok();
}

pub fn import_book(base: &Path, name: String, content: &str) -> Result<ImportResult, String> {
    import_book_with_overwrite(base, name, content, false)
}

pub fn import_book_with_overwrite(
    base: &Path,
    name: String,
    content: &str,
    overwrite: bool,
) -> Result<ImportResult, String> {
    ensure_data(base);
    let mut data = load_books(base);
    let book_name = name.trim().to_string();
    if book_name.is_empty() {
        return Err("书名不能为空".into());
    }
    let old_ids: Vec<String> = data
        .books
        .iter()
        .filter(|b| b.store_id.is_none() && b.name == book_name)
        .map(|b| b.id.clone())
        .collect();
    if !old_ids.is_empty() && !overwrite {
        return Err("同名本地书已存在".into());
    }

    use rand::Rng;
    let id = format!("{:016x}", rand::thread_rng().gen::<u64>());
    let txt_path = books_dir(base).join(format!("{}.txt", &id));
    fs::write(&txt_path, content).map_err(|e| format!("保存书籍失败：{}", e))?;

    // 导入时拆分章节并存盘（缓存写失败不致命，打开时会重新解析）
    let chapters = parse_chapters(content);
    save_chapters(base, &id, &chapters);

    let book = Book {
        id: id.clone(),
        name: book_name.clone(),
        last_chapter: 0,
        last_position: 0.0,
        added_at: chrono_now(),
        store_id: None,
    };
    // 只替换同名本地书；允许与在线书同名。
    data.books
        .retain(|b| b.store_id.is_some() || b.name != book_name);
    data.books.push(book.clone());

    if let Err(e) = write_atomic(&data_file(base), &serde_json::to_string_pretty(&data).unwrap_or_default()) {
        // 书架写入失败：回滚刚写入的文件，避免留下孤儿书籍
        let _ = fs::remove_file(&txt_path);
        let _ = fs::remove_file(chapters_file(base, &id));
        return Err(format!("保存书架失败：{}", e));
    }

    // 新数据落盘成功后再删除被覆盖的旧本地版本
    for old_id in old_ids {
        let _ = fs::remove_file(books_dir(base).join(format!("{}.txt", old_id)));
        let _ = fs::remove_file(chapters_file(base, &old_id));
    }

    Ok(ImportResult { book })
}

pub fn open_saved(base: &Path, id: &str) -> Result<OpenSavedResult, String> {
    let path = books_dir(base).join(format!("{}.txt", id));
    let content = fs::read_to_string(&path).map_err(|_| "书籍文件丢失，请重新导入".to_string())?;

    // 有章节缓存则直接读，否则现场解析（兼容旧数据）
    let chapters = load_chapters(base, id).unwrap_or_else(|| {
        let chs = parse_chapters(&content);
        save_chapters(base, id, &chs);
        chs
    });

    Ok(OpenSavedResult { content, chapters })
}

pub fn update_book(base: &Path, id: &str, lastChapter: Option<usize>, lastPosition: Option<f64>) -> Result<(), String> {
    ensure_data(base);
    let mut data = load_books(base);
    let updated = if let Some(book) = data.books.iter_mut().find(|b| b.id == id) {
        if let Some(v) = lastChapter { book.last_chapter = v; }
        if let Some(v) = lastPosition { book.last_position = v; }
        true
    } else {
        false
    };
    if updated {
        write_atomic(&data_file(base), &serde_json::to_string_pretty(&data).unwrap_or_default())
            .map_err(|e| format!("保存阅读进度失败：{}", e))?;
    }
    Ok(())
}

pub fn delete_book(base: &Path, id: &str) -> Result<(), String> {
    ensure_online_books(base);
    if let Some(store_id) = id.strip_prefix("store-") {
        let mut data = load_online_books(base);
        if let Some(book) = data.books.iter_mut().find(|b| b.store_id == store_id) {
            book.in_shelf = false;
            write_online_books(base, &data)?;
        }
        return Ok(());
    }
    let mut data = load_books(base);
    data.books.retain(|b| b.id != id);
    write_atomic(&data_file(base), &serde_json::to_string_pretty(&data).unwrap_or_default())
        .map_err(|e| format!("保存书架失败：{}", e))?;
    fs::remove_file(books_dir(base).join(format!("{}.txt", id))).ok();
    fs::remove_file(chapters_file(base, id)).ok();
    Ok(())
}

pub fn rename_book(base: &Path, id: &str, newName: &str) -> Result<(), String> {
    let new_name = newName.trim();
    if new_name.is_empty() || new_name.chars().count() > 50 {
        return Err("书名需在1-50字之间".into());
    }
    ensure_online_books(base);
    if let Some(store_id) = id.strip_prefix("store-") {
        let mut data = load_online_books(base);
        if !data.books.iter().any(|b| b.store_id == store_id) {
            return Err("书籍不存在".into());
        }
        if data
            .books
            .iter()
            .any(|b| b.in_shelf && b.store_id != store_id && b.name == new_name)
        {
            return Err("同名在线书已存在".into());
        }
        if let Some(book) = data.books.iter_mut().find(|b| b.store_id == store_id) {
            book.name = new_name.to_string();
            write_online_books(base, &data)?;
        }
        return Ok(());
    }

    let mut data = load_books(base);
    if !data.books.iter().any(|b| b.id == id) {
        return Err("书籍不存在".into());
    }
    if data
        .books
        .iter()
        .any(|b| b.id != id && b.name == new_name)
    {
        return Err("同名本地书已存在".into());
    }
    if let Some(book) = data.books.iter_mut().find(|b| b.id == id) {
        book.name = new_name.to_string();
        write_atomic(&data_file(base), &serde_json::to_string_pretty(&data).unwrap_or_default())
            .map_err(|e| format!("保存书名失败：{}", e))?;
    }
    Ok(())
}

fn chrono_now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let d = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default();
    format!("{}", d.as_secs())
}

// ===== 单元测试 =====

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_chapters_with_chapters() {
        let content = "前言文字\n\n第一章 开始\n这是第一章内容\n\n第二章 继续\n这是第二章内容";
        let chs = parse_chapters(content);
        assert!(chs.len() >= 3, "应有至少3个章节，实际: {}", chs.len());
        assert_eq!(chs[0].title, "前言");
        assert_eq!(chs[1].title, "第一章 开始");
        assert_eq!(chs[2].title, "第二章 继续");
    }

    #[test]
    fn test_parse_chapters_no_chapters() {
        let content = "没有章节的纯文本";
        let chs = parse_chapters(content);
        assert_eq!(chs.len(), 1);
        assert_eq!(chs[0].title, "正文");
    }

    #[test]
    fn test_parse_chapters_with_xuzi() {
        let content = "楔子 一切的开始\n楔子内容\n\n第一章 入门\n第一章内容";
        let chs = parse_chapters(content);
        assert_eq!(chs.len(), 4, "楔子+楔子内容+第一章+第一章内容, 实际: {}", chs.len());
        assert_eq!(chs[0].title, "楔子 一切的开始");
        assert_eq!(chs[1].title, "楔子内容");
        assert_eq!(chs[2].title, "第一章 入门");
        assert_eq!(chs[3].title, "第一章内容");
    }

    #[test]
    fn test_parse_chapters_is_standard() {
        let content = "第一章 开始\n内容\n\n第2章 继续\n内容\n尾声 结束\n内容";
        let chs = parse_chapters(content);
        assert!(chs[0].is_standard, "第一章应为标准章节");
        assert!(chs[1].is_standard, "第2章应为标准章节");
        assert!(!chs[2].is_standard, "尾声不应为标准章节");
    }

    #[test]
    fn test_parse_chapters_func_body() {
        // 简化正则后，行首为短关键词的正文行也会被当成章节
        let content = "正文前面\n\n番外 特别篇\n番外内容\n\n番外里面的一些故事\n不是番外标题";
        let chs = parse_chapters(content);
        assert_eq!(chs.len(), 4, "前言+番外特别篇+番外内容+番外里面的故事, 实际: {}", chs.len());
        assert_eq!(chs[0].title, "前言");
        assert_eq!(chs[1].title, "番外 特别篇");
        assert_eq!(chs[2].title, "番外内容");
        assert_eq!(chs[3].title, "番外里面的一些故事");
    }

    /// 按 UTF-16 码元偏移取子串，模拟前端 `String.prototype.slice`。
    fn slice_utf16(s: &str, start: usize, end: usize) -> String {
        let mut out = String::new();
        let mut u = 0usize;
        for ch in s.chars() {
            let next = u + ch.len_utf16();
            if u >= start && next <= end {
                out.push(ch);
            }
            u = next;
            if u >= end {
                break;
            }
        }
        out
    }

    #[test]
    fn test_offsets_adjacent_titles() {
        // 标题之间没有空行时，上一章不能吞掉下一章的标题首字
        let content = "第一章 开始\n正文一\n第二章 继续\n正文二";
        let chs = parse_chapters(content);
        assert_eq!(chs.len(), 2);
        assert_eq!(slice_utf16(content, chs[0].start, chs[0].end).trim(), "正文一");
        assert_eq!(slice_utf16(content, chs[1].start, chs[1].end).trim(), "正文二");
    }

    #[test]
    fn test_offsets_with_astral_chars() {
        // 星平面字符在 UTF-16 中占两个码元，边界必须与 JS slice 一致
        let content = "第一章 开始\n正文😀内容\n\n第二章 继续\n尾";
        let chs = parse_chapters(content);
        assert_eq!(chs.len(), 2);
        assert_eq!(slice_utf16(content, chs[0].start, chs[0].end).trim(), "正文😀内容");
        assert_eq!(slice_utf16(content, chs[1].start, chs[1].end).trim(), "尾");
    }

    #[test]
    fn test_preface_ends_before_first_title() {
        let content = "前言文字\n\n第一章 开始\n正文";
        let chs = parse_chapters(content);
        assert_eq!(chs[0].title, "前言");
        assert_eq!(slice_utf16(content, chs[0].start, chs[0].end).trim(), "前言文字");
    }

    #[test]
    fn test_chunk_does_not_drop_chars() {
        // 超长章节拆分时，未找到换行处的边界不能跳过字符
        let mut content = String::from("第一章 很长\n");
        for _ in 0..300000 {
            content.push('龙');
        }
        let chs = parse_chapters(&content);
        assert!(chs.len() >= 2, "超长章节应被拆分，实际: {}", chs.len());
        let total_utf16: usize = content.chars().map(|c| c.len_utf16()).sum();
        let expected = slice_utf16(&content, chs[0].start, total_utf16);
        let joined: String = chs
            .iter()
            .map(|c| slice_utf16(&content, c.start, c.end))
            .collect();
        assert_eq!(joined, expected, "拆分后不应丢失字符");
    }

    #[test]
    fn test_discover_data_root_workspace() {
        let base = std::env::temp_dir().join(format!("nr_discover_{}", std::process::id()));
        let exe_dir = base.join("src-tauri").join("target").join("debug");
        std::fs::create_dir_all(&exe_dir).unwrap();
        std::fs::write(base.join("package.json"), "{}").unwrap();

        assert_eq!(discover_data_root(&exe_dir), base);

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn test_discover_data_root_portable_fallback() {
        let base = std::env::temp_dir().join(format!("nr_portable_{}", std::process::id()));
        let exe_dir = base.join("NovelReader");
        std::fs::create_dir_all(&exe_dir).unwrap();

        assert_eq!(discover_data_root(&exe_dir), exe_dir);

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn test_write_atomic_replaces_existing() {
        let base = std::env::temp_dir().join(format!("nr_atomic_{}", std::process::id()));
        std::fs::create_dir_all(&base).unwrap();
        let path = base.join("books.json");

        write_atomic(&path, "first").unwrap();
        write_atomic(&path, "second").unwrap();

        assert_eq!(std::fs::read_to_string(&path).unwrap(), "second");
        let leftovers: Vec<_> = std::fs::read_dir(&base)
            .unwrap()
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|n| n.ends_with(".tmp"))
            .collect();
        assert!(leftovers.is_empty(), "临时文件应已被改名: {:?}", leftovers);
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn test_store_progress_syncs_shelf_book_and_survives_delete() {
        let base = std::env::temp_dir().join(format!("nr_store_progress_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(&base).unwrap();
        ensure_data(&base);

        let book = add_store_book(&base, "sid-test", "在线阅读测试").unwrap();
        assert_eq!(book.store_id.as_deref(), Some("sid-test"));

        save_store_progress(&base, "sid-test", 3, 0.25, Some("在线阅读测试"), Some("item-3")).unwrap();
        let shelf = get_books(&base);
        let item = shelf.iter().find(|b| b.id == book.id).unwrap();
        assert_eq!(item.last_chapter, 3);
        assert_eq!(item.last_position, 0.25);

        // 在线阅读保存进度 → 同步到书架视图
        save_store_progress(&base, "sid-test", 7, 0.6, Some("在线阅读测试"), Some("item-7")).unwrap();
        let online = get_store_progress(&base, "sid-test").unwrap();
        assert_eq!(online.last_chapter, 7);
        assert_eq!(online.last_position, 0.6);
        assert_eq!(online.last_item_id, "item-7");

        // 删除书架书后，在线阅读进度保留
        delete_book(&base, &book.id).unwrap();
        assert!(get_books(&base).is_empty());
        assert_eq!(get_store_progress(&base, "sid-test").unwrap().last_chapter, 7);

        // 重新加入书架时继承在线阅读进度
        let readded = add_store_book(&base, "sid-test", "在线阅读测试").unwrap();
        assert_eq!(readded.last_chapter, 7);
        assert_eq!(readded.last_position, 0.6);

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn test_migrate_legacy_online_books() {
        let base = std::env::temp_dir().join(format!("nr_online_migrate_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(&base).unwrap();
        ensure_data(&base);

        let local = Book {
            id: "local-1".into(),
            name: "本地书".into(),
            last_chapter: 1,
            last_position: 0.1,
            added_at: "1".into(),
            store_id: None,
        };
        let online = Book {
            id: "store-sid-m".into(),
            name: "在线书".into(),
            last_chapter: 2,
            last_position: 0.2,
            added_at: "2".into(),
            store_id: Some("sid-m".into()),
        };
        let old = BooksData { books: vec![local, online] };
        write_atomic(&data_file(&base), &serde_json::to_string(&old).unwrap()).unwrap();

        let mut progress = StoreProgressData::default();
        progress.books.insert(
            "sid-m".into(),
            StoreProgress { last_chapter: 4, last_position: 0.4, last_item_id: "item-4".into() },
        );
        progress.books.insert(
            "sid-direct".into(),
            StoreProgress { last_chapter: 1, last_position: 0.3, last_item_id: String::new() },
        );
        write_atomic(
            &store_progress_file(&base),
            &serde_json::to_string(&progress).unwrap(),
        )
        .unwrap();

        let books = get_books(&base);
        assert_eq!(books.len(), 2);
        assert!(books.iter().any(|b| b.id == "local-1"));
        assert!(books.iter().any(|b| b.store_id.as_deref() == Some("sid-m")));

        let local_only = load_books(&base);
        assert!(local_only.books.iter().all(|b| b.store_id.is_none()));
        assert!(base.join("online-books.json").exists());
        assert!(base.join("books.pre-online-v1.json").exists());
        assert!(base.join("store-progress.pre-online-v1.json").exists());

        let online_data = load_online_books(&base);
        let directed = online_data.books.iter().find(|b| b.store_id == "sid-direct").unwrap();
        assert!(!directed.in_shelf);

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn test_add_store_book_rejects_duplicate() {
        let base = std::env::temp_dir().join(format!("nr_store_dup_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(&base).unwrap();
        ensure_data(&base);

        add_store_book(&base, "sid-dup", "书").unwrap();
        let err = add_store_book(&base, "sid-dup", "书").unwrap_err();
        assert!(err.contains("已在书架"));

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn test_rename_uses_character_count() {
        let base = std::env::temp_dir().join(format!("nr_rename_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(&base).unwrap();
        ensure_data(&base);
        let imported = import_book(&base, "旧名字".into(), "第一章 开始\n正文").unwrap();
        let id = imported.book.id;

        // 17 个汉字 = 51 字节，但仍是合法的 17 个字符
        let name_17: String = std::iter::repeat('测').take(17).collect();
        assert_eq!(name_17.chars().count(), 17);
        rename_book(&base, &id, &name_17).unwrap();
        assert_eq!(get_books(&base)[0].name, name_17);

        // 51 个字符应被拒绝
        let name_51: String = std::iter::repeat('测').take(51).collect();
        assert!(rename_book(&base, &id, &name_51).is_err());

        // 纯空白应被拒绝
        assert!(rename_book(&base, &id, "   ").is_err());

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn test_import_rollback_when_books_json_unwritable() {
        let base = std::env::temp_dir().join(format!("nr_rollback_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(&base).unwrap();
        // 把 books.json 做成目录，写入必定失败
        std::fs::create_dir_all(base.join("books.json")).unwrap();

        let result = import_book(&base, "测试".into(), "第一章 开始\n正文");
        assert!(result.is_err(), "书架写入失败时应返回错误");

        let books_dir = base.join("books");
        let txt_count = std::fs::read_dir(&books_dir)
            .map(|d| d.filter_map(|e| e.ok()).count())
            .unwrap_or(0);
        assert_eq!(txt_count, 0, "失败后不应留下孤儿书籍文件");

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn test_save_settings_merges_partial_update() {
        let base = std::env::temp_dir().join(format!("nr_settings_{}", std::process::id()));
        std::fs::create_dir_all(&base).unwrap();
        ensure_data(&base);

        save_settings(&base, None, Some("night".into()), None, None);
        let s = get_settings(&base);
        assert_eq!(s.theme, "night");
        assert_eq!(s.font_size, 17, "未传字段应保留默认值");
        assert_eq!(s.line_height, 2.0);

        save_settings(&base, Some("book1".into()), None, Some(22), Some(3.0));
        let s = get_settings(&base);
        assert_eq!(s.last_book_id, "book1");
        assert_eq!(s.theme, "night", "未传的主题不应被重置");
        assert_eq!(s.font_size, 22);
        assert_eq!(s.line_height, 3.0);

        let _ = std::fs::remove_dir_all(&base);
    }
}
