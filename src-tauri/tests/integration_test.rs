// 集成测试：直接调用生产解析逻辑，避免复制实现后与 shared.rs 漂移。
use novelreader_lib::shared::{import_book, open_saved, parse_chapters};

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
fn test_parse_chapters_offsets() {
    let content = "作者前言\n\n第一章 入门\n本章正文内容。\n\n第二章 进阶\n这是第二章正文。";
    let chapters = parse_chapters(content);

    assert!(chapters.len() >= 3, "至少 3 个章节，实际: {}", chapters.len());
    assert_eq!(chapters[0].title, "前言");
    assert_eq!(slice_utf16(content, chapters[0].start, chapters[0].end).trim(), "作者前言");
    assert_eq!(chapters[1].title, "第一章 入门");
    assert_eq!(slice_utf16(content, chapters[1].start, chapters[1].end).trim(), "本章正文内容。");
    assert_eq!(chapters[2].title, "第二章 进阶");
    assert_eq!(slice_utf16(content, chapters[2].start, chapters[2].end).trim(), "这是第二章正文。");
}

#[test]
fn test_parse_large_file() {
    let mut content = String::from("简介\n\n");
    for i in 1..=50 {
        content.push_str(&format!("第{}章 标题{}\n", i, i));
        for _ in 0..1000 {
            content.push('龙');
        }
        content.push_str("\n\n");
    }

    let chapters = parse_chapters(&content);
    assert_eq!(chapters.len(), 51, "前言 + 50 章");

    for (i, ch) in chapters.iter().enumerate() {
        let text = slice_utf16(&content, ch.start, ch.end);
        assert!(!text.trim().is_empty(), "章节 {} 内容为空", i);
    }
}

#[test]
fn test_chunk_huge_chapter() {
    let mut content = String::from("第一章 很长\n");
    for _ in 0..300000 {
        content.push('龙');
    }

    let chapters = parse_chapters(&content);
    assert!(chapters.len() >= 2, "超长章节应被拆分，实际: {}", chapters.len());

    let total_utf16: usize = content.chars().map(|c| c.len_utf16()).sum();
    let expected = slice_utf16(&content, chapters[0].start, total_utf16);
    let joined: String = chapters
        .iter()
        .map(|c| slice_utf16(&content, c.start, c.end))
        .collect();
    assert_eq!(joined, expected, "拆分后不应丢失字符");
}

#[test]
fn test_import_and_open_roundtrip() {
    // 走真实的 import_book / open_saved，验证落盘、缓存读写与偏移一致
    let base = std::env::temp_dir().join(format!("novelreader_it_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    std::fs::create_dir_all(&base).unwrap();

    let content = "第一章 开始\n正文😀一\n\n第二章 继续\n正文二";
    let imported = import_book(&base, "测试书".into(), content).unwrap();
    let opened = open_saved(&base, &imported.book.id).unwrap();
    assert_eq!(opened.content, content);
    assert_eq!(opened.chapters.len(), 2);
    assert_eq!(
        slice_utf16(&opened.content, opened.chapters[0].start, opened.chapters[0].end).trim(),
        "正文😀一"
    );
    assert_eq!(
        slice_utf16(&opened.content, opened.chapters[1].start, opened.chapters[1].end).trim(),
        "正文二"
    );

    // 导入时应生成 v2 缓存，再次打开走缓存且边界不变
    let cache = base
        .join("books")
        .join(format!("{}.chapters.v2.json", imported.book.id));
    assert!(cache.exists(), "应生成 v2 章节缓存");

    let again = open_saved(&base, &imported.book.id).unwrap();
    assert_eq!(again.chapters.len(), 2);
    assert_eq!(again.chapters[0].start, opened.chapters[0].start);
    assert_eq!(again.chapters[1].end, opened.chapters[1].end);

    let _ = std::fs::remove_dir_all(&base);
}
