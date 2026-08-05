// ===== DOM 引用 =====
const bookshelf = document.getElementById("bookshelf");
const bookList = document.getElementById("bookList");
const reader = document.getElementById("reader");
const placeholder = document.getElementById("placeholder");
const chapterTitle = document.getElementById("chapterTitle");
const chapterContent = document.getElementById("chapterContent");
const btnPrev = document.getElementById("btnPrev");
const btnNext = document.getElementById("btnNext");
const btnPrevTop = document.getElementById("btnPrevTop");
const btnNextTop = document.getElementById("btnNextTop");
const chapterIndicator = document.getElementById("chapterIndicator");
const chapterIndicatorTop = document.getElementById("chapterIndicatorTop");

chapterIndicatorTop.addEventListener("click", () => {
  if (chapters.length > 0) btnChapters.click();
});
chapterIndicator.addEventListener("click", () => {
  if (chapters.length > 0) btnChapters.click();
});
const tocOverlay = document.getElementById("tocOverlay");
const tocPanel = document.getElementById("tocPanel");
const tocList = document.getElementById("tocList");
const btnOpen = document.getElementById("btnOpen");
const btnBookshelf = document.getElementById("btnBookshelf");
const btnBack = document.getElementById("btnBack");
const btnChapters = document.getElementById("btnChapters");
const btnTheme = document.getElementById("btnTheme");
const bookTitle = document.getElementById("bookTitle");
const fontSizeDropdown = document.getElementById("fontSizeDropdown");
const fontSizeValue = fontSizeDropdown.querySelector(".tbValue");
const fontSizeMenu = fontSizeDropdown.querySelector(".tbMenu");
const lineHeightDropdown = document.getElementById("lineHeightDropdown");
const lineHeightValue = lineHeightDropdown.querySelector(".tbValue");
const lineHeightMenu = lineHeightDropdown.querySelector(".tbMenu");

// ===== 状态 =====
let currentBook = null;   // { id, name, ... }
let fullContent = "";
let chapters = [];
let currentIndex = 0;
let currentTheme = "day";
let fontSize = 17;
let lineHeight = 2.0;

// ===== 启动：加载设置 + 恢复上次状态 =====
(async function init() {
  const settings = await api.getSettings();
  if (settings.error) { loadBookshelf(); return; }

  applyFontSize(settings.fontSize || 17);
  applyLineHeight(settings.lineHeight || 2.0);
  applyTheme(settings.theme || "day");

  if (settings.lastBookId) {
    // 恢复上次打开的书籍
    const books = await api.getBooks();
    if (books.error) { loadBookshelf(); return; }
    const book = books.find(b => b.id === settings.lastBookId);
    if (book) {
      const data = await api.openSaved(book.id);
      if (!data.error) {
        currentBook = book;
        fullContent = data.content;
        chapters = data.chapters;
        currentIndex = book.lastChapter || 0;
        if (currentIndex >= chapters.length) currentIndex = 0;
        bookTitle.textContent = book.name;
        bookTitle.style.visibility = "visible";
        showReader();
        renderChapter(currentIndex);
        history.replaceState({ view: "reader", bookId: book.id }, "", "/read");
        saveGlobalSettings();
        document.getElementById("contentArea").scrollTop = 0;
        return;
      }
    }
  }

  loadBookshelf();
})();

// ===== 书架 =====
async function loadBookshelf() {
  // 离开阅读页前保存进度
  if (currentBook) await saveProgress();
  const books = await api.getBooks();
  if (books.error) { bookList.innerHTML = '<p class="bookEmpty">加载书架失败，请重试</p>'; showBookshelf(); return; }
  if (books.length === 0) {
    bookList.innerHTML = '<p class="bookEmpty">书架空着，点击「导入」添加小说</p>';
  } else {
    bookList.innerHTML = books.map(b => `
      <div class="bookItem" data-id="${b.id}">
        <span class="bookName">${esc(b.name)}</span>
        <input class="bookRenameInput" value="${esc(b.name)}" maxlength="50" style="display:none" />
        <button class="bookRename" data-id="${b.id}">重命名</button>
        <button class="bookDelete" data-id="${b.id}">删除</button>
      </div>
    `).join("");
  }
  showBookshelf();
}

function setReadingControls(v) {
  document.getElementById("btnLineDown").style.display = v;
  document.getElementById("lineHeightDropdown").style.display = v;
  document.getElementById("btnLineUp").style.display = v;
  document.getElementById("btnFontDown").style.display = v;
  document.getElementById("fontSizeDropdown").style.display = v;
  document.getElementById("btnFontUp").style.display = v;
}

function showBookshelf() {
  bookshelf.style.display = "";
  reader.style.display = "none";
  bookTitle.style.visibility = "hidden";
  hideChapterNav();
  setReadingControls("none");
  document.getElementById("toolbar").classList.remove("reading");
  tocPanel.style.display = "none";
  tocOverlay.style.display = "none";
  btnBack.style.display = currentBook ? "" : "none";
}

// 点击书架上某本书
bookList.addEventListener("click", async (e) => {
  const item = e.target.closest(".bookItem");
  if (!item) return;

  // 确认/取消按钮（必须在 renaming 检查之前，因为处于 renaming 状态）
  if (e.target.classList.contains("bookRenameOk")) {
    await finishRename(item);
    return;
  }
  if (e.target.classList.contains("bookRenameCancel")) {
    cancelRename(item);
    return;
  }

  // 重命名状态下不响应其他点击，点输入框以外则取消
  if (item.classList.contains("renaming")) {
    if (!e.target.classList.contains("bookRenameInput")) {
      cancelRename(item);
    }
    return;
  }

  // 删除
  if (e.target.classList.contains("bookDelete")) {
    const delOk = await showConfirm("确定删除此书？");
    if (!delOk) return;
    await api.deleteBook(e.target.dataset.id);
    loadBookshelf();
    return;
  }

  // 重命名按钮
  if (e.target.classList.contains("bookRename")) {
    startRename(item);
    return;
  }

  // 点击书名 → 打开书籍
  if (e.target.classList.contains("bookName")) {
    // 如果正在编辑则不触发
    if (item.querySelector(".bookRenameInput").style.display !== "none") return;
  }

  const bookId = item.dataset.id;
  await openBook(bookId);
});

// ===== 导入 =====
const fileInput = document.getElementById("fileInput");

btnOpen.addEventListener("click", () => fileInput.click());

fileInput.addEventListener("change", async () => {
  const file = fileInput.files[0];
  if (!file) return;
  fileInput.value = "";

  // 1. 前端读取文件内容（自动检测编码）
  const content = await readFileContent(file);

  let bookName = file.name.replace(/\.txt$/i, "");
  if (bookName.length > 50) bookName = bookName.slice(0, 50);

  // 2. 检查重名
  const books = await api.getBooks();
  let dup = null;
  for (const b of books) {
    if (b.name === bookName) { dup = b; break; }
  }
  if (dup) {
    const ok = await showConfirm(`书架已存在「${bookName}」，是否覆盖？`);
    if (!ok) return;
  }

  // 3. 发送给服务器保存到 books/
  const data = await api.importBook(bookName, content);
  if (data.error) { await showAlert(data.error); return; }

  // 只在书架页面时刷新书架
  if (bookshelf.style.display !== "none") loadBookshelf();
  await showAlert(`「${data.book.name}」导入成功！`);
});

// ===== 重命名 =====
function startRename(item) {
  // 取消之前的重命名
  const prev = document.querySelector(".bookItem.renaming");
  if (prev && prev !== item) cancelRename(prev);

  const nameEl = item.querySelector(".bookName");
  const inputEl = item.querySelector(".bookRenameInput");
  const renameBtn = item.querySelector(".bookRename");

  nameEl.style.display = "none";
  renameBtn.style.display = "none";
  item.querySelector(".bookDelete").style.display = "none";
  item.classList.add("renaming");
  inputEl.value = nameEl.textContent;
  inputEl.style.display = "";
  inputEl.focus();
  inputEl.setSelectionRange(inputEl.value.length, inputEl.value.length);

  // 确认 / 取消按钮
  const okBtn = document.createElement("button");
  okBtn.className = "bookRenameOk";
  okBtn.textContent = "✓";
  const cancelBtn = document.createElement("button");
  cancelBtn.className = "bookRenameCancel";
  cancelBtn.textContent = "✗";

  const delBtn = item.querySelector(".bookDelete");
  delBtn.parentNode.insertBefore(cancelBtn, delBtn);
  delBtn.parentNode.insertBefore(okBtn, delBtn);

  inputEl.addEventListener("keydown", (e) => {
    if (e.key === "Enter") finishRename(item);
    if (e.key === "Escape") cancelRename(item);
  });
}

async function finishRename(item) {
  try {
    const oldName = item.querySelector(".bookName").textContent;
    const inputEl = item.querySelector(".bookRenameInput");
    const newName = inputEl.value.trim();
    if (!newName || newName === oldName) { cancelRename(item); return; }
    if (newName.length > 50) { await showAlert("书名不能超过50字"); cancelRename(item); return; }

    const data = await api.renameBook(item.dataset.id, newName);
    if (data && data.error) { await showAlert(data.error); cancelRename(item); return; }
    loadBookshelf();
  } catch (e) {
    await showAlert("重命名失败: " + e.message);
  }
}

function cancelRename(item) {
  // 恢复原状
  const inputEl = item.querySelector(".bookRenameInput");
  const nameEl = item.querySelector(".bookName");
  const renameBtn = item.querySelector(".bookRename");
  const delBtn = item.querySelector(".bookDelete");

  inputEl.style.display = "none";
  nameEl.style.display = "";
  renameBtn.style.display = "";
  delBtn.style.display = "";
  item.classList.remove("renaming");
  // 移除 ✓ ✗ 按钮
  item.querySelector(".bookRenameOk").remove();
  item.querySelector(".bookRenameCancel").remove();
}

// ===== 书架按钮 =====
btnBookshelf.addEventListener("click", loadBookshelf);
btnBack.addEventListener("click", async () => {
  if (!currentBook) return;
  await openBook(currentBook.id);
});

// ===== 打开书籍 =====
async function openBook(bookId) {
  const data = await api.openSaved(bookId);
  if (data.error) { await showAlert(data.error); return; }

  const books = await api.getBooks();
  currentBook = books.find(b => b.id === bookId);

  fullContent = data.content;
  chapters = data.chapters;

  currentIndex = (currentBook && currentBook.lastChapter) || 0;
  if (currentIndex >= chapters.length) currentIndex = 0;

  bookTitle.textContent = currentBook ? currentBook.name : "";
  bookTitle.style.visibility = "visible";
  showReader();
  renderChapter(currentIndex);
  saveGlobalSettings();
  history.pushState({ view: "reader", bookId }, "", "/read");
}

// ===== 保存 =====
async function saveProgress() {
  if (!currentBook) return;
  const area = document.getElementById("contentArea");
  const ratio = area.scrollHeight > area.clientHeight
    ? area.scrollTop / (area.scrollHeight - area.clientHeight) : 0;
  await api.updateBook(currentBook.id, { lastChapter: currentIndex, lastPosition: ratio });
  currentBook.lastChapter = currentIndex;
  currentBook.lastPosition = ratio;
}

async function saveGlobalSettings() {
  await api.saveSettings({
    lastBookId: currentBook ? currentBook.id : "",
    theme: currentTheme, fontSize, lineHeight,
  });
}

// ===== 阅读区切换 =====
function showReader() {
  bookshelf.style.display = "none";
  reader.style.display = "";
  reader.classList.add("hidden-placeholder");
  showChapterNav();
  setReadingControls("");
  document.getElementById("toolbar").classList.add("reading");
  tocPanel.style.display = "none";
  tocOverlay.style.display = "none";
  btnBack.style.display = "none";
}

function showLoading(msg) {
  bookshelf.style.display = "none";
  reader.style.display = "";
  placeholder.textContent = msg;
  chapterTitle.textContent = "";
  chapterContent.textContent = "";
  hideChapterNav();
  reader.classList.remove("hidden-placeholder");
}

function showChapterNav() {
  document.getElementById("chapterNavTop").style.display = "";
  document.getElementById("chapterNavBottom").style.display = "";
  btnChapters.style.display = "";
}

function hideChapterNav() {
  document.getElementById("chapterNavTop").style.display = "none";
  document.getElementById("chapterNavBottom").style.display = "none";
  btnChapters.style.display = "none";
}

// ===== 滚动到底部/顶部 20 次滚轮 → 上/下一章 =====
let _scrollCount = 0;
let _scrollTimer = null;
let _scrollDirection = 0; // 1=向下(下一章), -1=向上(上一章)
let _wasAtBottom = false; // wheel 之前是否已到底（由 scroll 事件维护）
let _wasAtTop = false;    // wheel 之前是否已到顶

function resetScrollTrigger() {
  _scrollCount = 0;
  _scrollDirection = 0;
  if (_scrollTimer) { clearTimeout(_scrollTimer); _scrollTimer = null; }
  const area = document.getElementById("contentArea");
  _wasAtTop = area.scrollTop <= 2;
  _wasAtBottom = area.scrollTop + area.clientHeight >= area.scrollHeight - 2;
}

// scroll 事件：追踪「滚轮之前」的边界状态，保证滚到底的那一滚不计入
// 用 rAF 延迟读取布局属性，避免在 scroll 事件中强制重排导致闪烁
let _scrollBoundPending = false;
document.getElementById("contentArea").addEventListener("scroll", () => {
  if (_scrollBoundPending) return;
  _scrollBoundPending = true;
  requestAnimationFrame(() => {
    const area = document.getElementById("contentArea");
    _wasAtBottom = area.scrollTop + area.clientHeight >= area.scrollHeight - 2;
    _wasAtTop = area.scrollTop <= 2;
    _scrollBoundPending = false;
  });
});

// wheel 事件：只有此前已到底/顶，本次滚动才算
document.getElementById("contentArea").addEventListener("wheel", (e) => {
  const area = document.getElementById("contentArea");
  const dir = e.deltaY > 0 ? 1 : -1;

  if (dir !== _scrollDirection) { resetScrollTrigger(); }

  if (dir === 1) {
    if (currentIndex >= chapters.length - 1) return;
    if (!_wasAtBottom) return;                             // 这一滚才到底 → 不计
  } else {
    if (currentIndex <= 0) return;
    if (!_wasAtTop) return;                                // 这一滚才到顶 → 不计
  }

  _scrollDirection = dir;
  _scrollCount++;
  if (_scrollCount === 1) {
    _scrollTimer = setTimeout(resetScrollTrigger, 3000);
  } else if (_scrollCount >= 7) {
    resetScrollTrigger();
    goChapter(dir);
  }
});

// ===== 章节渲染 =====
function renderChapter(index) {
  resetScrollTrigger();
  currentIndex = index;
  const ch = chapters[index];
  let title = ch.title;
  if (title.length > 50) title = title.slice(0, 50) + "…";
  chapterTitle.textContent = title;
  chapterContent.textContent = fullContent.slice(ch.start, ch.end).trim();

  btnPrev.disabled = btnPrevTop.disabled = index === 0;
  btnNext.disabled = btnNextTop.disabled = index === chapters.length - 1;
  // 指示器：标准章节独立计数
  const totalStd = chapters.filter(c => c.isStandard).length;
  let stdNum = 0;
  for (let i = 0; i <= index; i++) { if (chapters[i].isStandard) stdNum++; }
  let text;
  if (totalStd > 0 && stdNum > 0) {
    text = `第 ${stdNum} 章 / 共 ${totalStd} 章`;
  } else if (totalStd > 0) {
    text = `- / 共 ${totalStd} 章`;
  } else {
    text = `第 ${index + 1} 章 / 共 ${chapters.length} 章`;
  }
  chapterIndicator.textContent = chapterIndicatorTop.textContent = text;

  // 更新目录高亮
  [...tocList.children].forEach((li, i) => li.classList.toggle("active", i === index));

  // 在同一个 rAF 中设置滚动位置，避免先跳到顶部再跳回导致闪烁
  const restorePos = currentBook && currentBook.lastPosition !== undefined && currentBook.lastChapter === index;
  requestAnimationFrame(() => {
    const area = document.getElementById("contentArea");
    if (restorePos) {
      area.scrollTop = currentBook.lastPosition * (area.scrollHeight - area.clientHeight);
    } else {
      area.scrollTop = 0;
    }
  });

  // 保存进度（延迟防抖）
  clearTimeout(saveProgress._timer);
  saveProgress._timer = setTimeout(saveProgress, 500);
}

function goChapter(delta) {
  const idx = currentIndex + delta;
  if (idx >= 0 && idx < chapters.length) {
    renderChapter(idx);
  }
}
btnPrev.addEventListener("click", () => goChapter(-1));
btnNext.addEventListener("click", () => goChapter(1));
btnPrevTop.addEventListener("click", () => goChapter(-1));
btnNextTop.addEventListener("click", () => goChapter(1));

// ===== 目录 =====
btnChapters.addEventListener("click", () => {
  if (chapters.length === 0) return;
  // 渲染目录列表
  tocList.innerHTML = chapters.map((ch, i) => {
    let t = ch.title;
    if (t.length > 30) t = t.slice(0, 30) + "…";
    return `<li data-index="${i}" class="${i === currentIndex ? 'active' : ''}">${esc(t)}</li>`;
  }).join("");
  tocPanel.style.display = "";
  tocOverlay.style.display = "";
  // 目录变为可见后刷新滚动条
  requestAnimationFrame(() => {
    tocUpdateThumb();
    const active = tocList.querySelector("li.active");
    if (active) active.scrollIntoView({ block: "center" });
  });
});

tocList.addEventListener("click", (e) => {
  const li = e.target.closest("li");
  if (!li) return;
  const index = parseInt(li.dataset.index);
  renderChapter(index);
  tocPanel.style.display = "none";
  tocOverlay.style.display = "none";
});

document.getElementById("tocClose").addEventListener("click", () => {
  tocPanel.style.display = "none";
  tocOverlay.style.display = "none";
});

tocOverlay.addEventListener("click", () => {
  tocPanel.style.display = "none";
  tocOverlay.style.display = "none";
});

// ===== 主题 =====
function applyTheme(t) {
  currentTheme = t;
  if (t === "night") {
    document.documentElement.classList.add("night");
    btnTheme.textContent = "☀️ 日间模式";
  } else {
    document.documentElement.classList.remove("night");
    btnTheme.textContent = "🌙 夜间模式";
  }
  // 同步原生窗口标题栏
  try {
    const raw = window.__TAURI__ || window.__TAURI_INTERNALS__;
    if (raw && raw.window) {
      raw.window.getCurrentWindow().setTheme(t === "night" ? "dark" : "light");
    }
  } catch (_) {}
  // 同步 WebView 背景色，防止 resize 闪烁
  try {
    const raw = window.__TAURI__ || window.__TAURI_INTERNALS__;
    if (raw && raw.core) {
      raw.core.invoke("set_window_bg", { color: t === "night" ? "#1a1a1a" : "#f5f0e8" });
    }
  } catch (_) {}
}

btnTheme.addEventListener("click", () => {
  applyTheme(currentTheme === "day" ? "night" : "day");
  saveGlobalSettings();
});

// ===== 字体大小 =====
const FONT_SIZES = [12, 14, 16, 17, 18, 20, 22, 24, 26, 28];

function applyFontSize(size) {
  fontSize = size;
  reader.style.fontSize = size + "px";
  fontSizeValue.textContent = size;
  [...fontSizeMenu.children].forEach(li => {
    li.classList.toggle("active", parseInt(li.dataset.value) === size);
  });
}

// ===== 行间距 =====
const LINE_HEIGHTS = [1.4, 1.6, 1.8, 2.0, 2.2, 2.4, 2.6, 2.8, 3.0, 3.2, 3.4, 3.6, 3.8, 4.0];

function applyLineHeight(h) {
  lineHeight = h;
  reader.style.lineHeight = h;
  lineHeightValue.textContent = h.toFixed(1);
  [...lineHeightMenu.children].forEach(li => {
    li.classList.toggle("active", parseFloat(li.dataset.value) === h);
  });
}

// ===== 下拉框通用逻辑 =====
function setupDropdown(dropdownEl, onChange) {
  const valueEl = dropdownEl.querySelector(".tbValue");
  const menuEl = dropdownEl.querySelector(".tbMenu");
  valueEl.addEventListener("click", (e) => {
    e.stopPropagation();
    document.querySelectorAll(".tbMenu.show").forEach(m => {
      if (m !== menuEl) m.classList.remove("show");
    });
    menuEl.classList.toggle("show");
  });
  menuEl.addEventListener("click", (e) => {
    const val = e.target.dataset.value;
    if (val) { onChange(val); menuEl.classList.remove("show"); }
  });
}

setupDropdown(fontSizeDropdown, (val) => { applyFontSize(parseInt(val)); saveGlobalSettings(); });

btnFontUp.addEventListener("click", () => {
  const idx = FONT_SIZES.indexOf(fontSize);
  if (idx < FONT_SIZES.length - 1) { applyFontSize(FONT_SIZES[idx + 1]); saveGlobalSettings(); }
});
btnFontDown.addEventListener("click", () => {
  const idx = FONT_SIZES.indexOf(fontSize);
  if (idx > 0) { applyFontSize(FONT_SIZES[idx - 1]); saveGlobalSettings(); }
});

setupDropdown(lineHeightDropdown, (val) => { applyLineHeight(parseFloat(val)); saveGlobalSettings(); });

btnLineUp.addEventListener("click", () => {
  const idx = LINE_HEIGHTS.indexOf(lineHeight);
  if (idx < LINE_HEIGHTS.length - 1) { applyLineHeight(LINE_HEIGHTS[idx + 1]); saveGlobalSettings(); }
});
btnLineDown.addEventListener("click", () => {
  const idx = LINE_HEIGHTS.indexOf(lineHeight);
  if (idx > 0) { applyLineHeight(LINE_HEIGHTS[idx - 1]); saveGlobalSettings(); }
});

// ===== 编码检测 =====
function scoreText(text) {
  let cjk = 0, ascii = 0, garbled = 0, total = text.length;
  for (let i = 0; i < total; i++) {
    const c = text.charCodeAt(i);
    if (c === 0xFFFD) garbled++;
    else if ((c >= 0x4E00 && c <= 0x9FFF) || (c >= 0x3400 && c <= 0x4DBF)) cjk++;
    else if (c < 128) ascii++;
  }
  const valid = cjk + ascii;
  const validChars = Math.max(1, total - garbled);
  return { garbled, ratio: valid / validChars };
}

function readFileContent(file) {
  return new Promise((resolve, reject) => {
    const fr = new FileReader();
    fr.onload = () => {
      const buffer = new Uint8Array(fr.result);
      const bom = detectBOM(buffer);
      if (bom) { resolve(bom.decoder.decode(buffer.slice(bom.skip))); return; }

      const SAMPLE_SIZE = 10000;
      const sample = buffer.slice(0, SAMPLE_SIZE);
      const encodings = ["utf-8", "gb18030"];
      let bestEnc = "utf-8", bestGarbled = Infinity, bestRatio = 0;

      for (const enc of encodings) {
        try {
          const text = new TextDecoder(enc, { fatal: false }).decode(sample);
          const s = scoreText(text);
          // 优先选替换字符少的；替换字符相同时选中文字符比例高的
          if (s.garbled < bestGarbled || (s.garbled === bestGarbled && s.ratio > bestRatio)) {
            bestGarbled = s.garbled; bestRatio = s.ratio; bestEnc = enc;
          }
        } catch {}
      }

      // 如果 UTF-8/GB18030 结果太差（中文字符比例 < 5%），才考虑 UTF-16
      if (bestRatio < 0.05) {
        for (const enc of ["utf-16le", "utf-16be"]) {
          try {
            const text = new TextDecoder(enc, { fatal: false }).decode(sample);
            const s = scoreText(text);
            if (s.ratio > bestRatio) {
              bestRatio = s.ratio; bestEnc = enc;
            }
          } catch {}
        }
      }

      console.log(`   编码检测: ${bestEnc} (有效比例: ${(bestRatio*100).toFixed(0)}%)`);
      resolve(new TextDecoder(bestEnc).decode(buffer));
    };
    fr.onerror = () => reject(new Error("文件读取失败"));
    fr.readAsArrayBuffer(file);
  });
}

function detectBOM(buffer) {
  if (buffer[0] === 0xEF && buffer[1] === 0xBB && buffer[2] === 0xBF)
    return { label: "utf-8", skip: 3, decoder: new TextDecoder("utf-8") };
  if (buffer[0] === 0xFF && buffer[1] === 0xFE)
    return { label: "utf-16le", skip: 2, decoder: new TextDecoder("utf-16le") };
  if (buffer[0] === 0xFE && buffer[1] === 0xFF)
    return { label: "utf-16be", skip: 2, decoder: new TextDecoder("utf-16be") };
  return null;
}

// ===== 自定义弹窗（Tauri WebView2 原生 alert/confirm 不可靠） =====
function showDialog(msg, type) {
  return new Promise(resolve => {
    document.getElementById("dialogMsg").textContent = msg;
    document.getElementById("dialogCancel").style.display = type === "confirm" ? "" : "none";
    document.getElementById("dialogOverlay").style.display = "";
    document.getElementById("dialogBox").style.display = "";
    document.getElementById("dialogOk").onclick = () => { hideDialog(); resolve(true); };
    document.getElementById("dialogCancel").onclick = () => { hideDialog(); resolve(false); };
  });
}
function hideDialog() {
  document.getElementById("dialogOverlay").style.display = "none";
  document.getElementById("dialogBox").style.display = "none";
}
const showAlert = msg => showDialog(msg, "alert");
const showConfirm = msg => showDialog(msg, "confirm");

// ===== 窄屏设置面板 =====
const btnSettings = document.getElementById("btnSettings");
const settingsPanel = document.getElementById("settingsPanel");

btnSettings.addEventListener("click", (e) => {
  e.stopPropagation();
  const visible = settingsPanel.style.display !== "none";
  if (visible) { settingsPanel.style.display = "none"; return; }
  document.getElementById("settingsLineVal").textContent = lineHeight.toFixed(1);
  document.getElementById("settingsFontVal").textContent = fontSize;
  document.getElementById("settingsTheme").textContent = currentTheme === "night" ? "☀️ 日间模式" : "🌙 夜间模式";
  settingsPanel.style.display = "";
});

document.getElementById("settingsBookshelf").addEventListener("click", () => {
  settingsPanel.style.display = "none";
  loadBookshelf();
});

function updateSettingsDisplay() {
  document.getElementById("settingsLineVal").textContent = lineHeight.toFixed(1);
  document.getElementById("settingsFontVal").textContent = fontSize;
  // 更新 active
  document.querySelectorAll("#settingsLineMenu li").forEach(li => {
    li.classList.toggle("active", parseFloat(li.dataset.value) === lineHeight);
  });
  document.querySelectorAll("#settingsFontMenu li").forEach(li => {
    li.classList.toggle("active", parseInt(li.dataset.value) === fontSize);
  });
}

// 设置面板行间距
document.getElementById("settingsLineDown").addEventListener("click", (e) => {
  e.stopPropagation();
  const idx = LINE_HEIGHTS.indexOf(lineHeight);
  if (idx > 0) { applyLineHeight(LINE_HEIGHTS[idx - 1]); saveGlobalSettings(); updateSettingsDisplay(); }
});
document.getElementById("settingsLineUp").addEventListener("click", (e) => {
  e.stopPropagation();
  const idx = LINE_HEIGHTS.indexOf(lineHeight);
  if (idx < LINE_HEIGHTS.length - 1) { applyLineHeight(LINE_HEIGHTS[idx + 1]); saveGlobalSettings(); updateSettingsDisplay(); }
});
document.getElementById("settingsLineVal").addEventListener("click", (e) => {
  e.stopPropagation();
  document.getElementById("settingsLineMenu").classList.toggle("show");
});

// 设置面板字号
document.getElementById("settingsFontDown").addEventListener("click", (e) => {
  e.stopPropagation();
  const idx = FONT_SIZES.indexOf(fontSize);
  if (idx > 0) { applyFontSize(FONT_SIZES[idx - 1]); saveGlobalSettings(); updateSettingsDisplay(); }
});
document.getElementById("settingsFontUp").addEventListener("click", (e) => {
  e.stopPropagation();
  const idx = FONT_SIZES.indexOf(fontSize);
  if (idx < FONT_SIZES.length - 1) { applyFontSize(FONT_SIZES[idx + 1]); saveGlobalSettings(); updateSettingsDisplay(); }
});
document.getElementById("settingsFontVal").addEventListener("click", (e) => {
  e.stopPropagation();
  document.getElementById("settingsFontMenu").classList.toggle("show");
});

// 下拉菜单选项
document.getElementById("settingsLineMenu").addEventListener("click", (e) => {
  const val = parseFloat(e.target.dataset.value);
  if (val) { applyLineHeight(val); saveGlobalSettings(); updateSettingsDisplay(); }
  e.target.parentElement.classList.remove("show");
});
document.getElementById("settingsFontMenu").addEventListener("click", (e) => {
  const val = parseInt(e.target.dataset.value);
  if (val) { applyFontSize(val); saveGlobalSettings(); updateSettingsDisplay(); }
  e.target.parentElement.classList.remove("show");
});

document.getElementById("settingsTheme").addEventListener("click", () => {
  applyTheme(currentTheme === "day" ? "night" : "day");
  saveGlobalSettings();
});

// 点击面板外关闭
document.addEventListener("click", (e) => {
  // Close dropdown menus
  document.querySelectorAll(".tbMenu.show, .settingsCtrl .settingsMenu.show").forEach(m => m.classList.remove("show"));
  // Close rename if clicking outside
  const renaming = document.querySelector(".bookItem.renaming");
  if (renaming) {
    if (!e.target.classList.contains("bookRenameInput") &&
        !e.target.classList.contains("bookRenameOk") &&
        !e.target.classList.contains("bookRenameCancel") &&
        !e.target.classList.contains("bookRename")) {
      cancelRename(renaming);
    }
  }
  // Close settings panel
  if (settingsPanel.style.display !== "none" && !settingsPanel.contains(e.target) && e.target !== btnSettings) {
    settingsPanel.style.display = "none";
  }
});

// ===== 自定义滚动条（复用） =====
let _scrollDrag = null;

function setupCustomScrollbar(scrollArea, bar, thumb) {
  let _thumbRafPending = false;

  function updateThumb() {
    if (_thumbRafPending) return;
    _thumbRafPending = true;
    requestAnimationFrame(() => {
      const ratio = scrollArea.clientHeight / scrollArea.scrollHeight;
      const thumbH = Math.max(20, ratio * scrollArea.clientHeight);
      const scrollRatio = scrollArea.scrollTop / (scrollArea.scrollHeight - scrollArea.clientHeight);
      thumb.style.height = thumbH + "px";
      thumb.style.top = scrollRatio * (scrollArea.clientHeight - thumbH) + "px";
      bar.style.display = ratio >= 1 ? "none" : "";
      _thumbRafPending = false;
    });
  }

  scrollArea.addEventListener("scroll", updateThumb);
  window.addEventListener("resize", updateThumb);

  thumb.addEventListener("mousedown", (e) => {
    _scrollDrag = { scrollArea, thumb, startY: e.clientY, startScroll: scrollArea.scrollTop };
    document.body.style.userSelect = "none";
  });

  setTimeout(updateThumb, 100);
  const observer = new MutationObserver(updateThumb);
  observer.observe(scrollArea, { childList: true, subtree: true, characterData: true });

  return updateThumb;
}

// 全局拖拽事件（共用，只绑定一次）
document.addEventListener("mousemove", (e) => {
  if (!_scrollDrag) return;
  const { scrollArea, thumb, startY, startScroll } = _scrollDrag;
  const dy = e.clientY - startY;
  const ratio = (scrollArea.scrollHeight - scrollArea.clientHeight) / (scrollArea.clientHeight - thumb.clientHeight);
  scrollArea.scrollTop = startScroll + dy * ratio;
});
document.addEventListener("mouseup", () => {
  _scrollDrag = null;
  document.body.style.userSelect = "";
});

// 初始化正文滚动条
setupCustomScrollbar(
  document.getElementById("contentArea"),
  document.getElementById("customScrollbar"),
  document.getElementById("customScrollThumb")
);

// 初始化目录滚动条，保存更新函数供打开目录时调用
const tocUpdateThumb = setupCustomScrollbar(
  document.getElementById("tocScrollArea"),
  document.getElementById("tocScrollbar"),
  document.getElementById("tocScrollThumb")
);

// ===== 工具 =====
function esc(s) { return s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/"/g, "&quot;"); }

// ===== 浏览器前进/后退 =====
window.addEventListener("popstate", (e) => {
  if (e.state && e.state.view === "bookshelf") {
    loadBookshelf();
  } else if (e.state && e.state.view === "reader" && e.state.bookId) {
    // 恢复到阅读状态
    openBook(e.state.bookId);
  } else {
    loadBookshelf();
  }
});

// ===== 页面卸载前强制保存 =====
window.addEventListener("beforeunload", () => {
  if (!currentBook) return;
  const area = document.getElementById("contentArea");
  const ratio = area.scrollHeight > area.clientHeight
    ? area.scrollTop / (area.scrollHeight - area.clientHeight) : 0;
  if (typeof window.__TAURI__ !== "undefined") {
    // Tauri 模式，同步 invoke（beforeunload 期间只能用同步方式，用 sendBeacon 等效）
    const data = JSON.stringify({ id: currentBook.id, lastChapter: currentIndex, lastPosition: ratio });
    navigator.sendBeacon("/api/update-book", new Blob([data], { type: "application/json" }));
  } else {
    const data = JSON.stringify({ id: currentBook.id, lastChapter: currentIndex, lastPosition: ratio });
    navigator.sendBeacon("/api/update-book", new Blob([data], { type: "application/json" }));
  }
});

// ===== 窗口 resize 时禁用 transition，防止 WebView2 闪烁 =====
let _resizeSuppressTimer = null;
window.addEventListener("resize", () => {
  document.documentElement.classList.add("disable-transitions");
  clearTimeout(_resizeSuppressTimer);
  _resizeSuppressTimer = setTimeout(() => {
    document.documentElement.classList.remove("disable-transitions");
  }, 200);
});

// 初始
hideChapterNav();
