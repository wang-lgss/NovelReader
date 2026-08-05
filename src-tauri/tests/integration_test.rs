use std::fs;
use std::path::PathBuf;
use std::sync::Once;

static INIT: Once = Once::new();

fn test_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("tauri_novel_test");
    fs::create_dir_all(&dir).ok();
    dir
}

fn parse_chapters_test(content: &str) -> Vec<(String, usize, usize)> {
    let re = regex::Regex::new(
        r"(?m)^\s*(第[零一二三四五六七八九十百千万\d]+[章卷节](\s|$)|楔子(\s|$)|序章(\s|$)|序言(\s|$)|终章(\s|$)|尾声(\s|$)|后记(\s|$)|番外(\s|$)|完本感言(\s|$)).*"
    ).unwrap();
    let matches: Vec<_> = re.find_iter(content).collect();
    let mut result = Vec::new();

    if matches.is_empty() {
        result.push(("正文".into(), 0, content.len()));
        return result;
    }

    if matches[0].start() > 0 {
        result.push(("前言".into(), 0, matches[0].start()));
    }

    for i in 0..matches.len() {
        let m = matches[i];
        let title = m.as_str().trim().to_string();
        let start = m.end();
        let end = if i + 1 < matches.len() { matches[i + 1].start() } else { content.len() };
        result.push((title, start, end));
    }

    result
}

#[test]
fn test_open_saved_with_imported_book() {
    let dir = test_dir();
    let books_dir = dir.join("books");
    fs::create_dir_all(&books_dir).unwrap();

    // 模拟导入
    let content = "作者前言\n\n第一章 入门\n本章正文内容。\n\n第二章 进阶\n第二章正文。";
    let id = "testbook001";
    fs::write(books_dir.join(format!("{}.txt", id)), content).unwrap();

    // 模拟 books.json
    let book = serde_json::json!({
        "id": id,
        "name": "测试书",
        "lastChapter": 0,
        "lastPosition": 0.0,
        "fontSize": 17,
        "lineHeight": 2.0,
        "theme": "day",
        "addedAt": "0"
    });
    let data = serde_json::json!({ "books": [book] });
    fs::write(dir.join("books.json"), data.to_string()).unwrap();

    // 模拟打开书籍
    let saved_content = fs::read_to_string(books_dir.join(format!("{}.txt", id))).unwrap();
    assert!(!saved_content.is_empty());

    // 解析章节
    let chapters = parse_chapters_test(&saved_content);
    println!("章节数: {}", chapters.len());
    for (i, (title, start, end)) in chapters.iter().enumerate() {
        println!("  [{}] {} ({}..{})", i, title, start, end);
        let ch_content = &saved_content[*start..*end];
        assert!(!ch_content.is_empty(), "章节内容不应为空");
    }

    assert!(chapters.len() >= 3, "至少3个章节: 前言+第一章+第二章");

    // 检查具体内容
    let ch1 = &saved_content[chapters[1].1..chapters[1].2];
    assert!(ch1.contains("本章正文内容"), "第一章应包含正文");
}

#[test]
fn test_import_large_file() {
    let mut content = String::new();
    content.push_str("简介\n\n");
    for i in 1..=50 {
        content.push_str(&format!("第{}章 标题{}\n", i, i));
        for _ in 0..1000 { content.push_str("龙"); }
        content.push_str("\n\n");
    }

    let chapters = parse_chapters_test(&content);
    println!("总章节: {}", chapters.len());
    assert_eq!(chapters.len(), 51, "前言+50章");

    // 每章内容提取
    for (i, (title, start, end)) in chapters.iter().enumerate() {
        let ch = &content[*start..*end];
        assert!(!ch.is_empty(), "章节{}内容为空", i);
    }
}
