// 共享业务逻辑，不依赖 Tauri，浏览器和 Tauri 模式共用
#![allow(non_snake_case)]
use serde::{Deserialize, Serialize};
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
    #[serde(default = "default_font_size")]
    pub font_size: u32,
    #[serde(default = "default_line_height")]
    pub line_height: f64,
    #[serde(default = "default_theme")]
    pub theme: String,
    #[serde(default)]
    pub added_at: String,
}

fn default_font_size() -> u32 { 17 }
fn default_line_height() -> f64 { 2.0 }
fn default_theme() -> String { "day".into() }

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BooksData {
    pub books: Vec<Book>,
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

#[allow(dead_code)]
pub fn base_dir() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."))
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
    books_dir(base).join(format!("{}.chapters.json", id))
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

pub fn parse_chapters(content: &str) -> Vec<Chapter> {
    let re = chapter_re();
    let matches: Vec<_> = re.find_iter(content).collect();

    // 构建 字节→字符 映射（JS 用 UTF-16 码元偏移，需统一为字符偏移）
    let byte_to_char: Vec<usize> = {
        let mut v = vec![0usize];
        for (bi, _) in content.char_indices() {
            v.push(bi);
        }
        v.push(content.len());
        v
    };
    fn to_char(btc: &[usize], byte_pos: usize) -> usize {
        match btc.binary_search(&byte_pos) {
            Ok(i) => i,
            Err(i) => i - 1,
        }
    }

    let mut raw = Vec::new();

    if matches.is_empty() {
        let end = content.chars().count();
        raw.push(Chapter { title: "正文".into(), start: 0, end, is_standard: false });
        return chunk_large_chapters(content, &raw);
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
            content.chars().count()
        };
        raw.push(Chapter { title, start, end, is_standard: std });
    }

    chunk_large_chapters(content, &raw)
}

fn chunk_large_chapters(content: &str, chapters: &[Chapter]) -> Vec<Chapter> {
    let chars: Vec<char> = content.chars().collect();
    let total_chars = chars.len();
    let mut result = Vec::new();
    for ch in chapters {
        let len = ch.end.saturating_sub(ch.start);
        if len <= MAX_CHUNK_CHARS {
            result.push(ch.clone());
            continue;
        }
        let mut offset = ch.start;
        let mut part = 1usize;
        let total = (len + MAX_CHUNK_CHARS - 1) / MAX_CHUNK_CHARS;
        while offset < ch.end {
            let mut end = std::cmp::min(offset + MAX_CHUNK_CHARS, ch.end);
            if end < ch.end {
                // 在字符数组中找换行
                let search_end = std::cmp::min(end, total_chars);
                let slice: String = chars[..search_end].iter().collect();
                if let Some(nl) = slice.rfind('\n') {
                    if nl > offset + MAX_CHUNK_CHARS / 2 { end = nl; }
                }
            }
            let title = format!("{} ({}/{})", ch.title, part, total);
            result.push(Chapter { title, start: offset, end, is_standard: ch.is_standard && part == 1 });
            offset = end + 1;
            part += 1;
        }
    }
    result
}

fn save_chapters(base: &Path, id: &str, chapters: &[Chapter]) {
    let path = chapters_file(base, id);
    let _ = fs::write(&path, serde_json::to_string(chapters).unwrap_or_default());
}

fn load_chapters(base: &Path, id: &str) -> Option<Vec<Chapter>> {
    let path = chapters_file(base, id);
    if !path.exists() { return None; }
    let content = fs::read_to_string(&path).ok()?;
    serde_json::from_str(&content).ok()
}

// ===== 业务逻辑 =====

pub fn get_books(base: &Path) -> Vec<Book> {
    ensure_data(base);
    let content = fs::read_to_string(data_file(base)).unwrap_or_default();
    let data: BooksData = serde_json::from_str(&content).unwrap_or(BooksData { books: vec![] });
    data.books
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
    let s = Settings {
        last_book_id: last_book_id.unwrap_or_default(),
        theme: theme.unwrap_or_else(|| "day".into()),
        font_size: font_size.unwrap_or(17),
        line_height: line_height.unwrap_or(2.0),
    };
    fs::write(settings_file(base), serde_json::to_string_pretty(&s).unwrap_or_default()).ok();
}

pub fn import_book(base: &Path, name: String, content: &str) -> Result<ImportResult, String> {
    ensure_data(base);
    let mut data: BooksData = {
        let c = fs::read_to_string(data_file(base)).unwrap_or_default();
        serde_json::from_str(&c).unwrap_or(BooksData { books: vec![] })
    };

    if let Some(old) = data.books.iter().find(|b| b.name == name) {
        let old_id = old.id.clone();
        fs::remove_file(books_dir(base).join(format!("{}.txt", old_id))).ok();
        fs::remove_file(chapters_file(base, &old_id)).ok();
        data.books.retain(|b| b.name != name);
    }

    use rand::Rng;
    let id = format!("{:016x}", rand::thread_rng().gen::<u64>());

    fs::write(books_dir(base).join(format!("{}.txt", &id)), content).map_err(|e| e.to_string())?;

    // 导入时拆分章节并存盘
    let chapters = parse_chapters(content);
    save_chapters(base, &id, &chapters);

    let book = Book {
        id,
        name,
        last_chapter: 0,
        last_position: 0.0,
        font_size: 17,
        line_height: 2.0,
        theme: "day".into(),
        added_at: chrono_now(),
    };
    data.books.push(book.clone());
    fs::write(data_file(base), serde_json::to_string_pretty(&data).unwrap_or_default()).ok();

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

pub fn update_book(base: &Path, id: &str, lastChapter: Option<usize>, lastPosition: Option<f64>, fontSize: Option<u32>, lineHeight: Option<f64>, theme: Option<String>) -> Result<(), String> {
    ensure_data(base);
    let mut data: BooksData = {
        let c = fs::read_to_string(data_file(base)).unwrap_or_default();
        serde_json::from_str(&c).unwrap_or(BooksData { books: vec![] })
    };
    if let Some(book) = data.books.iter_mut().find(|b| b.id == id) {
        if let Some(v) = lastChapter { book.last_chapter = v; }
        if let Some(v) = lastPosition { book.last_position = v; }
        if let Some(v) = fontSize { book.font_size = v; }
        if let Some(v) = lineHeight { book.line_height = v; }
        if let Some(v) = theme { book.theme = v.clone(); }
        fs::write(data_file(base), serde_json::to_string_pretty(&data).unwrap_or_default()).ok();
    }
    Ok(())
}

pub fn delete_book(base: &Path, id: &str) -> Result<(), String> {
    ensure_data(base);
    let mut data: BooksData = {
        let c = fs::read_to_string(data_file(base)).unwrap_or_default();
        serde_json::from_str(&c).unwrap_or(BooksData { books: vec![] })
    };
    data.books.retain(|b| b.id != id);
    fs::write(data_file(base), serde_json::to_string_pretty(&data).unwrap_or_default()).ok();
    fs::remove_file(books_dir(base).join(format!("{}.txt", id))).ok();
    fs::remove_file(chapters_file(base, id)).ok();
    Ok(())
}

pub fn rename_book(base: &Path, id: &str, newName: &str) -> Result<(), String> {
    if newName.is_empty() || newName.len() > 50 {
        return Err("书名需在1-50字之间".into());
    }
    ensure_data(base);
    let mut data: BooksData = {
        let c = fs::read_to_string(data_file(base)).unwrap_or_default();
        serde_json::from_str(&c).unwrap_or(BooksData { books: vec![] })
    };
    if data.books.iter().any(|b| b.name == newName) {
        return Err("同名书籍已存在".into());
    }
    if let Some(book) = data.books.iter_mut().find(|b| b.id == id) {
        book.name = newName.to_string();
        fs::write(data_file(base), serde_json::to_string_pretty(&data).unwrap_or_default()).ok();
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
}
