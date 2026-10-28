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
const tocRoot = document.getElementById("tocRoot");
const tocOverlay = document.getElementById("tocOverlay");
const tocList = document.getElementById("tocList");
const btnOpen = document.getElementById("btnOpen");
const btnBookshelf = document.getElementById("btnBookshelf");
const btnStore = document.getElementById("btnStore");
const store = document.getElementById("store");
const storeInput = document.getElementById("storeInput");
const storeSearchBtn = document.getElementById("storeSearchBtn");
const storeHint = document.getElementById("storeHint");
const storeList = document.getElementById("storeList");
const storeRankTabs = document.getElementById("storeRankTabs");
const storeRankList = document.getElementById("storeRankList");
const storeDetail = document.getElementById("storeDetail");
const storeDetailBack = document.getElementById("storeDetailBack");
const storeDetailCover = document.getElementById("storeDetailCover");
const storeDetailTitle = document.getElementById("storeDetailTitle");
const storeDetailAuthor = document.getElementById("storeDetailAuthor");
const storeDetailTags = document.getElementById("storeDetailTags");
const storeDetailMetaLine2 = document.getElementById("storeDetailMetaLine2");
const storeDetailAbstract = document.getElementById("storeDetailAbstract");
const storeDetailStatus = document.getElementById("storeDetailStatus");
const storeReadBtn = document.getElementById("storeReadBtn");
const storeAddBtn = document.getElementById("storeAddBtn");
const storeTocBtn = document.getElementById("storeTocBtn");
const storeDownloadBtn = document.getElementById("storeDownloadBtn");
const storePreview = document.getElementById("storePreview");
const storePreviewList = document.getElementById("storePreviewList");
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
      if (book.storeId) {
        await openBook(book.id);
        history.replaceState({ view: "reader", bookId: book.id }, "", "/read");
        return;
      }
      const data = await api.openSaved(book.id);
      if (!data.error) {
        currentBook = book;
        fullContent = data.content;
        chapters = data.chapters;
        currentIndex = book.lastChapter || 0;
        if (currentIndex >= chapters.length) currentIndex = 0;
        bookTitle.textContent = book.name;
        bookTitle.style.display = "";
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
  if (onlineReading) {
    if (onlineReading.from === "store") currentBook = null;
    onlineReading = null;
  }
  const books = await api.getBooks();
  if (books.error) { bookList.innerHTML = '<p class="bookEmpty">加载书架失败，请重试</p>'; showBookshelf(); return; }
  if (books.length === 0) {
    bookList.innerHTML = '<p class="bookEmpty">书架空着，点击「导入」添加小说</p>';
  } else {
    bookList.innerHTML = books.map(b => `
      <div class="bookItem" data-id="${b.id}">
        <span class="bookName">${esc(b.name)}</span>${b.storeId ? '<span class="bookBadge onlineBadge">在线</span>' : '<span class="bookBadge localBadge">本地</span>'}
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
  const toolbar = document.getElementById("toolbar");
  toolbar.classList.remove("store", "reading");
  toolbar.classList.add("shelf");
  toolbar.classList.toggle("has-recent", Boolean(currentBook));
  bookshelf.style.display = "";
  reader.style.display = "none";
  store.style.display = "none";
  bookTitle.style.display = "none";
  hideChapterNav();
  setReadingControls("none");
  toolbar.classList.remove("reading");
  tocRoot.classList.remove("open");
  tocRoot.classList.remove("open");
  btnBack.textContent = "↩ 返回";
  btnBack.style.display = currentBook ? "" : "none";
}

// ===== 书城 =====
function showStore() {
  const toolbar = document.getElementById("toolbar");
  toolbar.classList.remove("shelf", "reading");
  toolbar.classList.add("store");
  toolbar.classList.toggle("has-recent", Boolean(currentBook));
  bookshelf.style.display = "none";
  reader.style.display = "none";
  store.style.display = "";
  bookTitle.style.display = "none";
  hideChapterNav();
  setReadingControls("none");
  document.getElementById("toolbar").classList.remove("reading");
  tocRoot.classList.remove("open");
  tocRoot.classList.remove("open");
  btnBack.textContent = "↩ 返回";
  btnBack.style.display = currentBook ? "" : "none";
  setStoreView("list");
  ensureStoreRank();
  storeInput.focus();
}

function storeCard(b) {
  const title = String(b.title || "未命名");
  const author = String(b.author || "未知作者");
  const category = b.category ? String(b.category) : "";
  const score = b.score ? String(b.score) : "";
  const abstract = String(b.abstract || "");
  const numericScore = Number(score);
  const scoreText = Number.isFinite(numericScore) && numericScore > 0 ? "评分 " + score : "暂无评分";
  const meta = [author, category, scoreText].filter(Boolean).map(esc).join(" · ");
  const coverText = String(b.cover || "");
  const cover = `<div class="storeCover">
      <span class="storeCoverEmpty" style="display:none">📖</span>
      <span class="storeCoverSpinner"></span>
      ${coverText ? `<img class="storeCoverImg" data-cover-id="${esc(String(b.id || ""))}" data-cover-url="${esc(coverText)}" alt="" style="display:none">` : ""}
      ${b.rank ? `<span class="storeRankNo">${esc(String(b.rank))}</span>` : ""}
    </div>`;
  return `<div class="storeCard" data-id="${esc(String(b.id || ""))}" data-score="${esc(String(b.score || ""))}">
    ${cover}
    <div class="storeBody">
      <h3 class="storeTitle">${esc(title)}</h3>
      <div class="storeMeta">${meta}</div>
      <p class="storeAbstract">${esc(abstract || "暂无简介")}</p>
    </div>
  </div>`;
}

const STORE_RANKS = [
  { algo: "101", label: "推荐榜" },
  { algo: "100", label: "完本榜" },
  { algo: "200", label: "巅峰榜" },
  { algo: "104", label: "口碑榜" },
];
const RANK_TTL_MS = 10 * 60 * 1000;
let storeRankAlgo = "101";
let storeRankToken = 0;
const storeRankCache = new Map();
const storeRankInflight = new Map();
const storeCoverCache = new Map();

function isHeicUrl(url) {
  return /\.heic(\?|$)/i.test(String(url || ""));
}

function renderStoreRankTabs() {
  [...storeRankTabs.children].forEach((btn) => {
    btn.classList.toggle("active", btn.dataset.algo === storeRankAlgo);
  });
}

function renderStoreRank(list) {
  if (!Array.isArray(list) || !list.length) {
    storeRankList.innerHTML = `<p class="storeHint">暂无榜单数据</p>`;
    return;
  }
  storeRankList.innerHTML = list.map(storeCard).join("");
}

function showCoverImage(img) {
  const cover = img && img.parentElement;
  if (!cover) return;
  img.style.display = "";
  const spinner = cover.querySelector(".storeCoverSpinner");
  const empty = cover.querySelector(".storeCoverEmpty");
  if (spinner) spinner.style.display = "none";
  if (empty) empty.style.display = "none";
}

function failCoverImage(img) {
  if (!img) return;
  img.style.display = "none";
  const cover = img.parentElement;
  const spinner = cover && cover.querySelector(".storeCoverSpinner");
  const empty = cover && cover.querySelector(".storeCoverEmpty");
  if (spinner) spinner.style.display = "none";
  if (empty) empty.style.display = "flex";
}

function showCoverEmpty(card) {
  if (!card) return;
  const spinner = card.querySelector(".storeCoverSpinner");
  const empty = card.querySelector(".storeCoverEmpty");
  if (spinner) spinner.style.display = "none";
  if (empty) empty.style.display = "flex";
}

async function retryCoverFromDetail(img) {
  if (!img || img.dataset.retried === "1") {
    failCoverImage(img);
    return;
  }
  img.dataset.retried = "1";
  try {
    const info = await api.storeInfo(img.dataset.coverId);
    let cover = info && info.cover ? String(info.cover) : "";
    if (cover && isHeicUrl(cover)) {
      cover = await api.coverJpeg(cover);
    }
    if (cover && !isHeicUrl(cover)) {
      if (img.dataset.coverId) storeCoverCache.set(img.dataset.coverId, cover);
      img.src = cover;
      return;
    }
  } catch (e) {
    // 详情兜底失败，保留占位
  }
  failCoverImage(img);
}

function bindCoverImage(img) {
  if (!img || img.dataset.bound === "1") return;
  img.dataset.bound = "1";
  img.addEventListener("load", () => showCoverImage(img));
  img.addEventListener("error", () => retryCoverFromDetail(img));
}

async function hydrateStoreCovers(books, container) {
  const cards = [...container.querySelectorAll(".storeCard")];
  const pending = [];
  cards.forEach((card, index) => {
    const book = books[index];
    const img = card.querySelector(".storeCoverImg");
    if (!img || !book) {
      if (img) failCoverImage(img);
      else showCoverEmpty(card);
      return;
    }
    const raw = String(book.cover || "");
    const id = String(book.id || "");
    if (!raw) {
      bindCoverImage(img);
      if (id) retryCoverFromDetail(img);
      else failCoverImage(img);
      return;
    }
    bindCoverImage(img);
    const cached = storeCoverCache.get(id);
    if (cached) {
      img.src = cached;
      return;
    }
    if (isHeicUrl(raw)) {
      pending.push({ img, id, url: raw });
    } else {
      img.src = raw;
    }
  });
  if (!pending.length) return;

  const urls = [...new Set(pending.map((item) => item.url))];
  let results = [];
  try {
    results = await api.coverBatch(urls);
  } catch (e) {
    results = [];
  }
  const covers = new Map();
  if (Array.isArray(results)) {
    urls.forEach((url, index) => {
      if (results[index]) covers.set(url, results[index]);
    });
  }
  for (const item of pending) {
    const cover = covers.get(item.url);
    if (cover) {
      storeCoverCache.set(item.id, cover);
      item.img.src = cover;
    } else {
      retryCoverFromDetail(item.img);
    }
  }
}

function rankEntryFresh(entry) {
  return Boolean(entry && Date.now() - entry.at < RANK_TTL_MS);
}

function fetchRankData(algo) {
  if (storeRankInflight.has(algo)) return storeRankInflight.get(algo);
  const task = (async () => {
    const raw = await api.rankBooks(algo);
    if (!Array.isArray(raw)) throw new Error(raw && raw.error ? raw.error : "榜单数据无效");
    const entry = { list: raw, at: Date.now() };
    storeRankCache.set(algo, entry);
    return entry;
  })();
  const tracked = task.finally(() => {
    if (storeRankInflight.get(algo) === tracked) storeRankInflight.delete(algo);
  });
  storeRankInflight.set(algo, tracked);
  return tracked;
}

function showStoreRankView() {
  storeRankTabs.style.display = "";
  storeRankList.style.display = "";
  storeHint.style.display = "none";
  storeList.style.display = "none";
  renderStoreRankTabs();
}

function hideStoreRankView() {
  storeRankTabs.style.display = "none";
  storeRankList.style.display = "none";
}

function rankLoadingHtml() {
  return `<div class="storeRankLoading">正在加载榜单…</div>`;
}

async function loadStoreRank(algo, force = false) {
  const token = ++storeRankToken;
  const cached = storeRankCache.get(algo);
  if (!force && rankEntryFresh(cached)) {
    if (algo === storeRankAlgo) {
      renderStoreRank(cached.list);
      hydrateStoreCovers(cached.list, storeRankList);
    }
    return;
  }
  storeRankList.innerHTML = rankLoadingHtml();
  try {
    const entry = await fetchRankData(algo);
    if (token !== storeRankToken) return;
    if (algo === storeRankAlgo) {
      renderStoreRank(entry.list);
      hydrateStoreCovers(entry.list, storeRankList);
    }
  } catch (e) {
    if (token !== storeRankToken) return;
    storeRankList.innerHTML = `<p class="storeHint">榜单加载失败：${esc(e.message || String(e))}</p>`;
  }
}

function selectStoreRank(algo) {
  if (!STORE_RANKS.some((r) => r.algo === algo)) return;
  storeRankAlgo = algo;
  renderStoreRankTabs();
  showStoreRankView();
  loadStoreRank(algo);
}

function ensureStoreRank() {
  if (!storeInput.value.trim()) {
    showStoreRankView();
    loadStoreRank(storeRankAlgo);
  }
}

let storeSearching = false;
async function doStoreSearch() {
  if (storeSearching) return;
  const q = storeInput.value.trim();
  if (!q) {
    storeList.innerHTML = "";
    storeHint.textContent = "";
    ensureStoreRank();
    return;
  }
  hideStoreRankView();
  storeHint.style.display = "";
  storeList.style.display = "";
  storeSearching = true;
  storeSearchBtn.disabled = true;
  storeHint.textContent = "搜索中…";
  storeList.innerHTML = "";
  try {
    const res = await api.searchStore(q);
    if (res && res.error) {
      storeHint.textContent = "搜索失败：" + res.error;
      return;
    }
    const list = Array.isArray(res) ? res : [];
    if (!list.length) {
      storeHint.textContent = "没有找到相关书籍";
      return;
    }
    if (q !== storeInput.value.trim()) return;
    storeHint.textContent = `找到 ${list.length} 本`;
    storeList.innerHTML = list.map(storeCard).join("");
    hydrateStoreCovers(list, storeList);
  } catch (e) {
    storeHint.textContent = "搜索失败：" + (e.message || e);
  } finally {
    storeSearching = false;
    storeSearchBtn.disabled = false;
  }
}

storeSearchBtn.addEventListener("click", doStoreSearch);
storeInput.addEventListener("keydown", (e) => {
  if (e.key === "Enter") doStoreSearch();
});
storeInput.addEventListener("input", () => {
  if (!storeInput.value.trim()) {
    storeList.innerHTML = "";
    storeHint.textContent = "";
    ensureStoreRank();
  }
});
btnStore.addEventListener("click", async () => {
  if (currentBook) await saveProgress();
  if (onlineReading) {
    currentBook = null;
    onlineReading = null;
    chapters = [];
    fullContent = "";
    clearOnlineChapterState();
  }
  showStore();
});

// ===== 书城详情 / 在线阅读 / 下载 =====
let storeBook = null;
let storeChapters = [];
let storeBusy = false;
let onlineReading = null;
const onlineChapterCache = new Map();
const onlineChapterFetching = new Map();
let onlinePrefetchTimer = null;
let onlinePrefetchToken = 0;
let onlineRenderToken = 0;
let storeProgress = null;
let storeDetailToken = 0;
let storeDetailScore = "";
let tocMode = "reader";

function setStoreView(view) {
  const isList = view === "list";
  const rankMode = isList && !storeInput.value.trim();
  document.getElementById("storeHeader").style.display = isList ? "" : "none";
  storeRankTabs.style.display = rankMode ? "" : "none";
  storeRankList.style.display = rankMode ? "" : "none";
  storeHint.style.display = isList && !rankMode ? "" : "none";
  storeList.style.display = isList && !rankMode ? "" : "none";
  storeDetail.style.display = view === "detail" ? "" : "none";
  document.getElementById("contentArea").scrollTop = 0;
}

function detailCoverLoadingHtml() {
  return `<span class="storeCoverEmpty" style="display:none">📖</span>
    <span class="storeCoverSpinner"></span>`;
}

function showDetailCoverEmpty() {
  storeDetailCover.innerHTML = `<span class="storeCoverEmpty">📖</span>`;
}

function showDetailCover(url) {
  const raw = String(url || "");
  storeDetailCover.innerHTML = detailCoverLoadingHtml();
  if (!raw) {
    showDetailCoverEmpty();
    return;
  }
  const render = (cover) => {
    if (!cover) {
      showDetailCoverEmpty();
      return;
    }
    const image = `<img class="storeCoverImg" src="${esc(String(cover))}" alt="" style="display:none"
      onload="this.style.display='';this.parentElement.querySelector('.storeCoverSpinner').style.display='none';this.parentElement.querySelector('.storeCoverEmpty').style.display='none';"
      onerror="this.style.display='none';this.parentElement.querySelector('.storeCoverSpinner').style.display='none';this.parentElement.querySelector('.storeCoverEmpty').style.display='flex';">`;
    storeDetailCover.innerHTML = `<span class="storeCoverEmpty" style="display:none">📖</span>
      <span class="storeCoverSpinner"></span>${image}`;
  };
  if (isHeicUrl(raw)) {
    api.coverJpeg(raw).then(render).catch(() => showDetailCoverEmpty());
  } else {
    render(raw);
  }
}

function formatWords(n) {
  if (!n) return "";
  if (n >= 10000) return (n / 10000).toFixed(1) + " 万字";
  return n.toLocaleString("zh-CN") + " 字";
}

function storeStatusText(info, chapterCount) {
  const label = info.status === "0" ? "已完结" : info.status === "1" ? "连载中" : "";
  if (!label) return chapterCount ? `${chapterCount} 章` : "";
  return `${label} · ${chapterCount}章`;
}

function storeScoreText(score) {
  const value = Number(score);
  return Number.isFinite(value) && value > 0 ? `评分 ${score}` : "";
}

async function loadStorePreview(bookId, list, token) {
  if (!list.length) {
    storePreview.style.display = "none";
    return;
  }
  storePreview.style.display = "";
  storePreviewList.innerHTML = '<p class="previewHint">试读加载中…</p>';
  const parts = [];
  for (let i = 0; i < list.length; i++) {
    const ch = list[i];
    let text = "";
    for (let attempt = 0; attempt < 2 && !text; attempt++) {
      try {
        const res = await api.storeContents(bookId, [ch.itemId]);
        if (res && res.error) throw new Error(res.error);
        text = res && res[ch.itemId];
      } catch (e) {
        // 自动重试一次
      }
      if (!text && attempt === 0) await new Promise((r) => setTimeout(r, 700));
    }
    if (token !== storeDetailToken) return;
    text = stripLeadingCss(text);
    text = stripChapterTitle(text, ch.title);
    parts.push(`<section class="previewChapter">
      <h4 class="previewChapterTitle">${esc(ch.title || `第 ${i + 1} 章`)}</h4>
      <div class="previewChapterText">${esc(text || "（本章加载失败，可能需付费或会员）")}</div>
    </section>`);
    storePreviewList.innerHTML = parts.join("");
  }
}

function updateStoreAbstractScroll() {
  if (!storeDetailAbstract) return;
  const scrollable = storeDetailAbstract.scrollHeight > storeDetailAbstract.clientHeight + 1;
  storeDetailAbstract.classList.toggle("scrollable", scrollable);
  if (!scrollable) storeDetailAbstract.scrollTop = 0;
}

if (window.ResizeObserver) {
  new ResizeObserver(updateStoreAbstractScroll).observe(storeDetailAbstract);
}

async function openStoreDetail(bookId, score = "") {
  const token = ++storeDetailToken;
  storeDetailScore = score;
  setStoreView("detail");
  storeBook = null;
  storeChapters = [];
  storeProgress = null;
  storeDetailTitle.textContent = "加载中…";
  storeDetailAuthor.textContent = "";
  storeDetailTags.innerHTML = "";
  storeDetailMetaLine2.textContent = "";
  storeDetailAbstract.textContent = "";
  storeDetailAbstract.classList.remove("scrollable");
  storeDetailStatus.textContent = "";
  storeDetailCover.innerHTML = detailCoverLoadingHtml();
  storePreview.style.display = "none";
  storePreviewList.innerHTML = "";
  storeReadBtn.textContent = "在线阅读";
  storeReadBtn.disabled = true;
  storeAddBtn.textContent = "加入书架";
  storeAddBtn.disabled = true;
  storeTocBtn.disabled = true;
  storeDownloadBtn.disabled = true;

  try {
    const call = async (fn) => {
      let first;
      try {
        first = await fn();
      } catch (e) {
        return await fn();
      }
      if (first && first.error) return await fn();
      return first;
    };
    const [infoResult, tocResult, booksResult, progressResult] = await Promise.allSettled([
      call(() => api.storeInfo(bookId)),
      call(() => api.storeChapters(bookId)),
      call(() => api.getBooks()),
      call(() => api.getStoreProgress(bookId)),
    ]);
    if (infoResult.status !== "fulfilled" || (infoResult.value && infoResult.value.error)) {
      throw new Error(
        infoResult.status === "rejected"
          ? infoResult.reason
          : (infoResult.value && infoResult.value.error) || "书籍详情加载失败"
      );
    }
    const info = infoResult.value;
    const toc = tocResult.status === "fulfilled" ? tocResult.value : null;
    const books = booksResult.status === "fulfilled" ? booksResult.value : [];
    const progress = progressResult.status === "fulfilled" ? progressResult.value : null;
    const tocFailed = !Array.isArray(toc) || Boolean(toc && toc.error);
    storeBook = info;
    storeChapters = Array.isArray(toc) ? toc : [];
    storeProgress = progress || null;

    storeDetailTitle.textContent = info.title || "未命名";
    storeDetailAuthor.textContent = info.author ? `作者：${info.author}` : "";
    const tags = String(info.category || "")
      .split("/")
      .map((t) => t.trim())
      .filter(Boolean);
    storeDetailTags.innerHTML = tags
      .map((t) => `<span class="storeTag">${esc(t)}</span>`)
      .join("");
    storeDetailMetaLine2.textContent = [
      formatWords(info.wordCount),
      storeStatusText(info, storeChapters.length),
      storeScoreText(storeDetailScore),
    ]
      .filter(Boolean)
      .join(" · ");
    storeDetailAbstract.textContent = info.abstract || "暂无简介";
    updateStoreAbstractScroll();
    requestAnimationFrame(updateStoreAbstractScroll);
    setTimeout(updateStoreAbstractScroll, 80);
    showDetailCover(info.cover);

    const inShelf = Array.isArray(books) && books.some((b) => b.storeId === info.id);
    storeAddBtn.textContent = inShelf ? "已在书架" : "加入书架";
    storeAddBtn.disabled = inShelf || storeChapters.length === 0;

    const hasProgress = Boolean(
      progress &&
      (
        (progress.lastItemId && storeChapters.some((c) => c.itemId === progress.lastItemId)) ||
        progress.lastChapter < storeChapters.length
      )
    );
    storeReadBtn.textContent = hasProgress ? "继续阅读" : "在线阅读";
    storeReadBtn.disabled = storeChapters.length === 0;
    storeTocBtn.disabled = storeChapters.length === 0;
    storeDownloadBtn.disabled = storeChapters.length === 0;
    if (tocFailed) storeDetailStatus.textContent = "目录加载失败，书籍详情已显示";

    loadStorePreview(bookId, storeChapters.slice(0, 2), token);
  } catch (e) {
    storeDetailStatus.textContent = "加载失败：" + (e.message || e);
  }
}

function resolveOnlineResume(list, progress, fallbackChapter, fallbackPosition) {
  const max = Math.max(list.length - 1, 0);
  let chapter = Math.min(Math.max(Number(fallbackChapter) || 0, 0), max);
  let position = Number(fallbackPosition) || 0;
  if (progress) {
    chapter = Math.min(Math.max(Number(progress.lastChapter) || 0, 0), max);
    position = Number(progress.lastPosition) || 0;
    if (progress.lastItemId) {
      const idx = list.findIndex((c) => c.itemId === progress.lastItemId);
      if (idx >= 0) {
        chapter = idx;
        position = Number(progress.lastPosition) || 0;
      } else {
        // 章节 ID 已失效时退回下标，位置归零，避免跳到错误位置
        position = 0;
      }
    }
  }
  return { chapter, position };
}

async function startOnlineReading() {
  if (!storeBook || !storeChapters.length) return;
  const progress = await api.getStoreProgress(storeBook.id);
  const resume = resolveOnlineResume(storeChapters, progress, 0, 0);
  await openOnlineReading({
    storeId: storeBook.id,
    name: storeBook.title || "未命名",
    chapters: storeChapters,
    chapter: resume.chapter,
    position: resume.position,
    from: "store",
  });
}

async function addStoreBookToShelf() {
  if (!storeBook || storeAddBtn.disabled) return;
  storeAddBtn.disabled = true;
  const title = storeBook.title || "未命名";
  const books = await api.getBooks();
  const sameOnline = Array.isArray(books)
    ? books.filter((b) => b.storeId && b.storeId !== storeBook.id && b.name === title)
    : [];
  let replace = false;
  if (sameOnline.length) {
    const ok = await showConfirm(
      `书架中已有同名在线书「${title}」。\n\n替换会将它移出书架，但保留在线阅读进度。\n\n是否替换？`
    );
    if (!ok) {
      storeAddBtn.disabled = false;
      return;
    }
    replace = true;
  }
  const res = await api.addStoreBook(storeBook.id, title, replace);
  if (res && res.error) {
    storeDetailStatus.textContent = "加入书架失败：" + res.error;
    storeAddBtn.disabled = false;
    return;
  }
  storeAddBtn.textContent = "已在书架";
  storeDetailStatus.textContent = "已加入书架，可在书架里继续阅读";
  updateReaderMoreActions();
}

function safeFileName(name) {
  return (name || "小说").replace(/[\\/:*?"<>|]/g, "_").slice(0, 60);
}

async function downloadStoreBook() {
  if (storeBusy || !storeBook || !storeChapters.length) return;
  storeBusy = true;
  storeDownloadBtn.disabled = true;

  const batchSize = 20;
  let text = `${storeBook.title || "未命名"}\n`;
  if (storeBook.author) text += `author: ${storeBook.author}\n`;
  text += "\n";
  let done = 0;
  let failed = 0;

  for (let i = 0; i < storeChapters.length; i += batchSize) {
    const chunk = storeChapters.slice(i, i + batchSize);
    storeDetailStatus.textContent = `正在下载 ${done} / ${storeChapters.length} 章…`;
    let map = null;
    for (let attempt = 0; attempt < 2 && !map; attempt++) {
      try {
        const r = await api.storeDownloadContents(storeBook.id, chunk.map((c) => c.itemId));
        if (r && r.error) throw new Error(r.error);
        map = r;
      } catch (e) {
        if (attempt === 0) await new Promise((res) => setTimeout(res, 800));
      }
    }
    if (!map) {
      failed += chunk.length;
      continue;
    }
    for (const c of chunk) {
      const t = map[c.itemId];
      if (t) {
        text += `\n${c.title || `第 ${i + 1} 章`}\n\n${stripChapterTitle(stripLeadingCss(t), c.title || "")}\n`;
        done++;
      } else {
        failed++;
      }
    }
  }

  if (done === 0) {
    storeDetailStatus.textContent = "下载失败：没有取到正文";
    storeBusy = false;
    storeDownloadBtn.disabled = false;
    return;
  }

  const fileName = safeFileName(storeBook.title) + ".txt";
  const isTauri = !!(window.__TAURI__ || window.__TAURI_INTERNALS__);
  if (isTauri) {
    const res = await api.saveTxtDialog(fileName, text);
    if (res && res.error) {
      storeDetailStatus.textContent = "导出失败：" + res.error;
    } else if (res === null || res === undefined) {
      storeDetailStatus.textContent = `已取消导出（已取到 ${done} 章）`;
    } else {
      storeDetailStatus.textContent = `已导出：${res}`;
    }
  } else {
    const blob = new Blob([text], { type: "text/plain;charset=utf-8" });
    const a = document.createElement("a");
    a.href = URL.createObjectURL(blob);
    a.download = fileName;
    a.click();
    setTimeout(() => URL.revokeObjectURL(a.href), 5000);
    storeDetailStatus.textContent = `已导出 ${fileName}`;
  }
  if (failed) storeDetailStatus.textContent += `（${failed} 章未取到）`;

  storeBusy = false;
  storeDownloadBtn.disabled = false;
}

function clearOnlineChapterState() {
  if (onlinePrefetchTimer) {
    clearTimeout(onlinePrefetchTimer);
    onlinePrefetchTimer = null;
  }
  onlinePrefetchToken++;
  onlineChapterFetching.clear();
  onlineChapterCache.clear();
}

async function ensureOnlineChapters(itemIds) {
  const unique = [...new Set(itemIds.filter(Boolean))].filter(
    (id) => !onlineChapterCache.has(id)
  );
  const newIds = unique.filter((id) => !onlineChapterFetching.has(id));
  if (newIds.length && onlineReading) {
    const storeId = onlineReading.storeId;
    const promise = (async () => {
      let remaining = newIds.slice();
      for (let attempt = 0; attempt < 2 && remaining.length; attempt++) {
        try {
          const res = await api.storeContents(storeId, remaining);
          if (res && res.error) throw new Error(res.error);
          const next = [];
          for (const id of remaining) {
            const text = res && res[id];
            if (text) onlineChapterCache.set(id, text);
            else next.push(id);
          }
          remaining = next;
        } catch (e) {
          // 自动重试一次
        }
        if (attempt === 0 && remaining.length) {
          await new Promise((r) => setTimeout(r, 900));
        }
      }
    })();
    for (const id of newIds) onlineChapterFetching.set(id, promise);
    const clean = () => {
      for (const id of newIds) {
        if (onlineChapterFetching.get(id) === promise) {
          onlineChapterFetching.delete(id);
        }
      }
    };
    promise.then(clean, clean);
  }
  const waits = unique
    .map((id) => onlineChapterFetching.get(id))
    .filter(Boolean);
  if (waits.length) await Promise.all(waits);
}

function scheduleOnlinePrefetch() {
  if (!onlineReading || reader.style.display === "none") return;
  if (onlinePrefetchTimer) clearTimeout(onlinePrefetchTimer);
  const token = ++onlinePrefetchToken;
  onlinePrefetchTimer = setTimeout(() => {
    if (token !== onlinePrefetchToken) return;
    if (!onlineReading || reader.style.display === "none") return;
    const ids = chapters
      .slice(currentIndex + 1, currentIndex + 4)
      .map((c) => c.itemId)
      .filter(Boolean);
    if (ids.length) ensureOnlineChapters(ids);
  }, 250);
}

async function openOnlineReading({ storeId, name, chapters: list, chapter, position, from, shelfId }) {
  onlineReading = { storeId, name, from };
  clearOnlineChapterState();
  chapters = list.map((c) => ({ title: c.title || "", itemId: c.itemId }));
  fullContent = "";
  currentBook = {
    id: shelfId || `store-${storeId}`,
    name,
    lastChapter: chapter || 0,
    lastPosition: position || 0,
    storeId,
  };
  currentIndex = Math.min(Math.max(chapter || 0, 0), Math.max(chapters.length - 1, 0));
  currentBook.lastItemId = (chapters[currentIndex] && chapters[currentIndex].itemId) || "";
  bookTitle.textContent = name || "";
  bookTitle.style.display = "";
  bookTitle.style.visibility = "visible";
  showReader();
  await renderChapter(currentIndex);
  saveGlobalSettings();
  updateReaderMoreActions();
}

async function loadOnlineChapter(itemId) {
  if (onlineChapterCache.has(itemId)) return onlineChapterCache.get(itemId);
  await ensureOnlineChapters([itemId]);
  if (onlineChapterCache.has(itemId)) return onlineChapterCache.get(itemId);
  return "（本章加载失败，可能需付费或会员）";
}

// 番茄正文开头常自带一行章节标题，和阅读器标题重复，去掉第一行。
function stripLeadingCss(text) {
  if (!text) return text;
  let s = String(text).replace(/^\uFEFF/, "");
  // 去掉内嵌 <style> 块
  s = s.replace(/<style\b[^>]*>[\s\S]*?<\/style>/gi, "");
  // 去掉章节开头的 CSS 规则（版权信息页常见的 .copyright-title {...}）
  s = s.replace(/^[ \t]*[.#][^{\r\n]*\{[^{}]*\}[ \t]*$/gm, "");
  return s.replace(/\n{3,}/g, "\n\n").replace(/^\s+/, "");
}

function stripChapterTitle(text, title) {
  if (!text) return text;
  const t = String(title || "").trim();
  if (!t) return text;
  const norm = (s) => s.replace(/\s+/g, "");
  const clean = text.replace(/^\s+/, "");
  const nl = clean.indexOf("\n");
  const first = (nl === -1 ? clean : clean.slice(0, nl)).trim();
  const nf = norm(first);
  const nt = norm(t);
  if (!nf) return text;
  if (nf === nt || (nt.length >= 2 && nf.startsWith(nt)) || (nt.length >= 4 && nt.startsWith(nf))) {
    return clean.slice(nl + 1).replace(/^\s*\n/, "");
  }
  return text;
}

async function leaveOnlineToDetail() {
  await saveProgress();
  onlineReading = null;
  currentBook = null;
  chapters = [];
  fullContent = "";
  clearOnlineChapterState();
  reader.style.display = "none";
  store.style.display = "";
  setStoreView("detail");
  if (storeBook) storeReadBtn.textContent = "继续阅读";
  bookTitle.style.display = "none";
  hideChapterNav();
  setReadingControls("none");
  const toolbar = document.getElementById("toolbar");
  toolbar.classList.remove("reading", "shelf", "has-recent");
  toolbar.classList.add("store");
  tocRoot.classList.remove("open");
  tocRoot.classList.remove("open");
  btnBack.style.display = "none";
  btnBack.textContent = "↩ 返回";
  updateReaderMoreActions();
}

storeRankTabs.addEventListener("click", (e) => {
  const btn = e.target.closest("button[data-algo]");
  if (btn && btn.dataset.algo) selectStoreRank(btn.dataset.algo);
});
storeRankList.addEventListener("click", (e) => {
  const card = e.target.closest(".storeCard");
  if (card && card.dataset.id) openStoreDetail(card.dataset.id, card.dataset.score || "");
});
storeList.addEventListener("click", (e) => {
  const card = e.target.closest(".storeCard");
  if (card && card.dataset.id) openStoreDetail(card.dataset.id, card.dataset.score || "");
});
storeDetailBack.addEventListener("click", () => setStoreView("list"));
storeReadBtn.addEventListener("click", startOnlineReading);
storeAddBtn.addEventListener("click", addStoreBookToShelf);
storeDownloadBtn.addEventListener("click", downloadStoreBook);

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
  let content;
  try {
    content = await readFileContent(file);
  } catch (e) {
    await showAlert(e.message || "文件读取失败");
    return;
  }

  let bookName = file.name.replace(/\.txt$/i, "");
  if (bookName.length > 50) bookName = bookName.slice(0, 50);

  // 2. 检查同名本地书；本地与在线允许同名
  const books = await api.getBooks();
  const dupLocal = Array.isArray(books)
    ? books.find((b) => !b.storeId && b.name === bookName)
    : null;
  let overwrite = false;
  if (dupLocal) {
    const ok = await showConfirm(
      `书架中已有同名本地书「${bookName}」。\n\n覆盖会删除旧书正文、章节缓存和阅读进度；在线同名书不受影响。\n\n是否覆盖？`
    );
    if (!ok) return;
    overwrite = true;
  }

  // 3. 发送给服务器保存到 books/
  const data = await api.importBook(bookName, content, overwrite);
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
  if (item.classList.contains("renaming")) return;

  const nameEl = item.querySelector(".bookName");
  const inputEl = item.querySelector(".bookRenameInput");
  const renameBtn = item.querySelector(".bookRename");

  nameEl.style.display = "none";
  renameBtn.style.display = "none";
  item.querySelector(".bookDelete").style.display = "none";
  item.classList.add("renaming");
  inputEl.value = nameEl.textContent;
  inputEl.maxLength = 50;
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

  // 用属性赋值而不是 addEventListener，避免多次重命名叠加监听器
  inputEl.oninput = () => {
    const cleaned = inputEl.value.replace(/[\r\n\t]+/g, " ");
    if (cleaned !== inputEl.value) inputEl.value = cleaned;
  };
  inputEl.onkeydown = (e) => {
    if (e.isComposing) return;
    if (e.key === "Enter") {
      e.preventDefault();
      if (e.shiftKey) return;
      finishRename(item);
    } else if (e.key === "Escape") {
      e.preventDefault();
      cancelRename(item);
    }
  };
}

async function finishRename(item) {
  if (!item || !item.classList.contains("renaming") || item.dataset.renameSaving === "1") {
    return;
  }
  item.dataset.renameSaving = "1";
  try {
    const nameEl = item.querySelector(".bookName");
    const inputEl = item.querySelector(".bookRenameInput");
    if (!nameEl || !inputEl) return;
    const oldName = nameEl.textContent;
    const newName = inputEl.value.replace(/[\r\n\t]+/g, " ").trim();
    if (!newName || newName === oldName) { cancelRename(item); return; }
    if (newName.length > 50) { await showAlert("书名不能超过50字"); cancelRename(item); return; }

    const data = await api.renameBook(item.dataset.id, newName);
    if (data && data.error) { await showAlert(data.error); cancelRename(item); return; }
    await loadBookshelf();
  } catch (e) {
    await showAlert("重命名失败: " + e.message);
    cancelRename(item);
  } finally {
    delete item.dataset.renameSaving;
  }
}

function cancelRename(item) {
  if (!item) return;
  // 恢复原状
  const inputEl = item.querySelector(".bookRenameInput");
  const nameEl = item.querySelector(".bookName");
  const renameBtn = item.querySelector(".bookRename");
  const delBtn = item.querySelector(".bookDelete");

  if (inputEl) inputEl.style.display = "none";
  if (nameEl) nameEl.style.display = "";
  if (renameBtn) renameBtn.style.display = "";
  if (delBtn) delBtn.style.display = "";
  item.classList.remove("renaming");
  // 移除 ✓ ✗ 按钮
  item.querySelector(".bookRenameOk")?.remove();
  item.querySelector(".bookRenameCancel")?.remove();
}

// ===== 书架按钮 =====
btnBookshelf.addEventListener("click", loadBookshelf);
btnBack.addEventListener("click", async () => {
  if (!currentBook) return;
  await openBook(currentBook.id);
});

// ===== 打开书籍 =====
async function openBook(bookId) {
  const books = await api.getBooks();
  const book = Array.isArray(books) ? books.find((b) => b.id === bookId) : null;

  // 书架里的在线书：抓目录走在线阅读
  if (book && book.storeId) {
    const list = await api.storeChapters(book.storeId);
    if (list && list.error) { await showAlert(list.error); return; }
    const progress = await api.getStoreProgress(book.storeId);
    const resume = resolveOnlineResume(
      Array.isArray(list) ? list : [],
      progress,
      book.lastChapter,
      book.lastPosition
    );
    await openOnlineReading({
      storeId: book.storeId,
      name: book.name,
      chapters: Array.isArray(list) ? list : [],
      chapter: resume.chapter,
      position: resume.position,
      from: "shelf",
      shelfId: book.id,
    });
    return;
  }

  const data = await api.openSaved(bookId);
  if (data.error) { await showAlert(data.error); return; }

  onlineReading = null;
  clearOnlineChapterState();
  currentBook = book;

  fullContent = data.content;
  chapters = data.chapters;

  currentIndex = (currentBook && currentBook.lastChapter) || 0;
  if (currentIndex >= chapters.length) currentIndex = 0;

  bookTitle.textContent = currentBook ? currentBook.name : "";
  bookTitle.style.display = "";
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
  if (onlineReading) {
    const itemId = chapters[currentIndex] && chapters[currentIndex].itemId;
    await api.saveStoreProgress(
      onlineReading.storeId,
      currentIndex,
      ratio,
      onlineReading.name,
      itemId || ""
    );
    storeProgress = { lastChapter: currentIndex, lastPosition: ratio, lastItemId: itemId || "" };
  } else {
    await api.updateBook(currentBook.id, { lastChapter: currentIndex, lastPosition: ratio });
  }
  currentBook.lastChapter = currentIndex;
  currentBook.lastPosition = ratio;
  if (onlineReading && chapters[currentIndex]) {
    currentBook.lastItemId = chapters[currentIndex].itemId || "";
  }
}

async function saveGlobalSettings() {
  const payload = { theme: currentTheme, fontSize, lineHeight };
  // 从书城直接在线阅读时不改写“上次打开的书”
  if (currentBook && !(onlineReading && onlineReading.from === "store")) {
    payload.lastBookId = currentBook.id;
  }
  await api.saveSettings(payload);
}

// ===== 滚动时防抖保存阅读位置 =====
// 崩溃或直接关窗时，最多丢失约 1 秒的滚动进度
let _scrollSaveTimer = null;
document.getElementById("contentArea").addEventListener("scroll", () => {
  if (!currentBook) return;
  clearTimeout(_scrollSaveTimer);
  _scrollSaveTimer = setTimeout(saveProgress, 800);
});

// ===== 阅读区切换 =====
function showReader() {
  const toolbar = document.getElementById("toolbar");
  toolbar.classList.remove("shelf", "store", "has-recent");
  toolbar.classList.add("reading");
  bookshelf.style.display = "none";
  store.style.display = "none";
  reader.style.display = "";
  reader.classList.add("hidden-placeholder");
  showChapterNav();
  setReadingControls("");
  tocRoot.classList.remove("open");
  tocRoot.classList.remove("open");
  btnBack.textContent = "↩ 返回";
  btnBack.style.display = "none";
  updateReaderMoreActions();
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
  if (reader.style.display === "none") return;
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
async function renderChapter(index) {
  resetScrollTrigger();
  if (!chapters.length) {
    chapterTitle.textContent = "";
    chapterContent.textContent = "";
    return;
  }
  currentIndex = Math.min(Math.max(index, 0), chapters.length - 1);
  const ch = chapters[currentIndex];
  let title = (ch && ch.title) || `第 ${currentIndex + 1} 章`;
  if (title.length > 50) title = title.slice(0, 50) + "…";
  chapterTitle.textContent = title;

  const token = ++onlineRenderToken;
  if (onlineReading) {
    const cached = onlineChapterCache.get(ch.itemId);
    if (cached) {
      chapterContent.textContent = stripChapterTitle(stripLeadingCss(cached), ch.title);
    } else {
      chapterContent.textContent = "加载中…";
      const text = await loadOnlineChapter(ch.itemId);
      if (token !== onlineRenderToken) return;
      chapterContent.textContent = stripChapterTitle(stripLeadingCss(text), ch.title);
    }
  } else {
    chapterContent.textContent = fullContent.slice(ch.start, ch.end).trim();
  }

  btnPrev.disabled = btnPrevTop.disabled = currentIndex === 0;
  btnNext.disabled = btnNextTop.disabled = currentIndex === chapters.length - 1;

  let text;
  if (onlineReading) {
    text = `第 ${currentIndex + 1} 章 / 共 ${chapters.length} 章`;
  } else {
    // 指示器：标准章节独立计数
    const totalStd = chapters.filter(c => c.isStandard).length;
    let stdNum = 0;
    for (let i = 0; i <= currentIndex; i++) { if (chapters[i].isStandard) stdNum++; }
    if (totalStd > 0 && stdNum > 0) {
      text = `第 ${stdNum} 章 / 共 ${totalStd} 章`;
    } else if (totalStd > 0) {
      text = `- / 共 ${totalStd} 章`;
    } else {
      text = `第 ${currentIndex + 1} 章 / 共 ${chapters.length} 章`;
    }
  }
  chapterIndicator.textContent = chapterIndicatorTop.textContent = text;

  if (onlineReading) scheduleOnlinePrefetch();

  // 更新目录高亮
  [...tocList.children].forEach((li, i) => li.classList.toggle("active", i === currentIndex));

  // 在同一个 rAF 中设置滚动位置，避免先跳到顶部再跳回导致闪烁
  const restorePos = currentBook && currentBook.lastPosition !== undefined && currentBook.lastChapter === currentIndex;
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
  tocMode = "reader";
  // 渲染目录列表
  tocList.innerHTML = chapters.map((ch, i) => {
    let t = ch.title;
    if (t.length > 30) t = t.slice(0, 30) + "…";
    return `<li data-index="${i}" class="${i === currentIndex ? 'active' : ''}">${esc(t)}</li>`;
  }).join("");
  tocRoot.classList.add("open");
  tocRoot.classList.add("open");
  // 目录变为可见后刷新滚动条
  requestAnimationFrame(() => {
    tocUpdateThumb();
    const active = tocList.querySelector("li.active");
    if (active) active.scrollIntoView({ block: "center" });
  });
});

function openDetailToc() {
  if (!storeBook || !storeChapters.length) return;
  tocMode = "detail";
  const active = storeProgress ? storeProgress.lastChapter : -1;
  tocList.innerHTML = storeChapters.map((ch, i) => {
    let t = ch.title || "未命名章节";
    if (t.length > 30) t = t.slice(0, 30) + "…";
    return `<li data-index="${i}" class="${i === active ? "active" : ""}">${esc(t)}</li>`;
  }).join("");
  tocRoot.classList.add("open");
  tocRoot.classList.add("open");
  requestAnimationFrame(() => {
    tocUpdateThumb();
    const item = tocList.querySelector("li.active");
    if (item) item.scrollIntoView({ block: "center" });
  });
}

storeTocBtn.addEventListener("click", openDetailToc);

tocList.addEventListener("click", (e) => {
  const li = e.target.closest("li");
  if (!li) return;
  const index = parseInt(li.dataset.index);
  if (tocMode === "detail") {
    tocRoot.classList.remove("open");
    tocRoot.classList.remove("open");
    if (!storeBook) return;
    openOnlineReading({
      storeId: storeBook.id,
      name: storeBook.title || "未命名",
      chapters: storeChapters,
      chapter: index,
      position: 0,
      from: "store",
    });
    return;
  }
  renderChapter(index);
  tocRoot.classList.remove("open");
  tocRoot.classList.remove("open");
});

document.getElementById("tocClose").addEventListener("click", () => {
  tocRoot.classList.remove("open");
  tocRoot.classList.remove("open");
});

tocOverlay.addEventListener("click", () => {
  tocRoot.classList.remove("open");
  tocRoot.classList.remove("open");
});

// ===== 主题 =====
function applyTheme(t) {
  currentTheme = t;
  const label = t === "night" ? "☀️ 日间模式" : "🌙 夜间模式";
  if (t === "night") {
    document.documentElement.classList.add("night");
  } else {
    document.documentElement.classList.remove("night");
  }
  btnTheme.textContent = label;
  const settingsTheme = document.getElementById("settingsTheme");
  if (settingsTheme) settingsTheme.textContent = label;
  // 同步原生 Windows 标题栏和 WebView 背景
  try {
    const raw = window.__TAURI__ || window.__TAURI_INTERNALS__;
    if (raw && raw.core) {
      raw.core.invoke("set_window_frame", { theme: t });
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
// 整本用 fatal 模式试解码：只有能完整解出、且没有大量控制字符的编码才被接受，
// 避免“采样猜错 → 用 U+FFFD 覆盖正文 → 写盘后无法恢复”。
function scoreText(text) {
  let cjk = 0, ascii = 0, garbled = 0, control = 0, total = text.length;
  for (let i = 0; i < total; i++) {
    const c = text.charCodeAt(i);
    if (c === 0xFFFD) garbled++;
    else if (c < 32 && c !== 9 && c !== 10 && c !== 13) control++;
    else if ((c >= 0x4E00 && c <= 0x9FFF) || (c >= 0x3400 && c <= 0x4DBF)) cjk++;
    else if (c < 128) ascii++;
  }
  const valid = cjk + ascii;
  const validChars = Math.max(1, total - garbled);
  return { garbled, controlRatio: control / Math.max(1, total), ratio: valid / validChars };
}

// 无 BOM 的 UTF-16 很难和 GB18030 区分：UTF-16 中文的高字节集中在 0x4E–0x9F，
// 且低字节常 >= 0x80；GBK 尾字节落在高字节区间的比例只有约 40%，ASCII 文本低字节不满足。
function looksLikeUtf16(buffer) {
  const n = Math.min(buffer.length, 4096) & ~1;
  if (n < 8) return null;
  const units = n / 2;
  let leHigh = 0, leLow = 0, beHigh = 0, beLow = 0;
  for (let i = 0; i < n; i += 2) {
    const a = buffer[i];      // LE 低字节 / BE 高字节
    const b = buffer[i + 1];  // LE 高字节 / BE 低字节
    if (b === 0 || (b >= 0x4E && b <= 0x9F)) leHigh++;
    if (a >= 0x80) leLow++;
    if (a === 0 || (a >= 0x4E && a <= 0x9F)) beHigh++;
    if (b >= 0x80) beLow++;
  }
  const leH = leHigh / units, leL = leLow / units;
  const beH = beHigh / units, beL = beLow / units;
  // 两个方向都算分，避免大端文本被小端“碰巧”命中
  if (leH >= 0.7 && leH >= beH + 0.15 && leL >= 0.4) return "utf-16le";
  if (beH >= 0.7 && beH >= leH + 0.15 && beL >= 0.4) return "utf-16be";
  return null;
}

function readFileContent(file) {
  return new Promise((resolve, reject) => {
    const fr = new FileReader();
    fr.onload = () => {
      const buffer = new Uint8Array(fr.result);
      if (buffer.length === 0) { reject(new Error("文件是空的")); return; }

      const bom = detectBOM(buffer);
      if (bom) { resolve(bom.decoder.decode(buffer.slice(bom.skip))); return; }

      const hint = looksLikeUtf16(buffer);
      const candidates = hint
        ? [hint, "utf-8", "gb18030"]
        : ["utf-8", "gb18030", "utf-16le", "utf-16be"];
      const SCORE_LIMIT = 200000;
      for (const enc of candidates) {
        let text;
        try {
          text = new TextDecoder(enc, { fatal: true }).decode(buffer);
        } catch { continue; }
        const s = scoreText(text.length > SCORE_LIMIT ? text.slice(0, SCORE_LIMIT) : text);
        // 合法的 UTF-16 文本被当作 UTF-8/GB18030 解出来，会充满 \u0000 之类的控制字符
        if (s.controlRatio < 0.01) {
          console.log(`   编码检测: ${enc} (有效比例: ${(s.ratio * 100).toFixed(0)}%)`);
          resolve(text);
          return;
        }
      }

      reject(new Error("无法识别文件编码，请先将文件另存为 UTF-8 后重试"));
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

function updateReaderMoreActions() {
  const addShelf = document.getElementById("settingsAddShelf");
  const backDetail = document.getElementById("settingsBackDetail");
  const direct = Boolean(
    onlineReading && onlineReading.from === "store" && reader.style.display !== "none"
  );
  addShelf.style.display = direct ? "" : "none";
  backDetail.style.display = direct ? "" : "none";
  if (!direct) return;
  const inShelf = Boolean(storeAddBtn && storeAddBtn.textContent.trim() === "已在书架");
  addShelf.textContent = inShelf ? "已在书架" : "加入书架";
  addShelf.classList.toggle("disabled", inShelf);
}

btnSettings.addEventListener("click", (e) => {
  e.stopPropagation();
  const visible = settingsPanel.style.display !== "none";
  if (visible) { settingsPanel.style.display = "none"; return; }
  document.getElementById("settingsLineVal").textContent = lineHeight.toFixed(1);
  document.getElementById("settingsFontVal").textContent = fontSize;
  document.getElementById("settingsTheme").textContent = currentTheme === "night" ? "☀️ 日间模式" : "🌙 夜间模式";
  updateReaderMoreActions();
  settingsPanel.style.display = "";
});

document.getElementById("settingsBookshelf").addEventListener("click", () => {
  settingsPanel.style.display = "none";
  loadBookshelf();
});

document.getElementById("settingsStore").addEventListener("click", () => {
  settingsPanel.style.display = "none";
  document.getElementById("btnStore").click();
});

document.getElementById("settingsAddShelf").addEventListener("click", async () => {
  const item = document.getElementById("settingsAddShelf");
  if (item.classList.contains("disabled")) return;
  settingsPanel.style.display = "none";
  await addStoreBookToShelf();
});

document.getElementById("settingsBackDetail").addEventListener("click", async () => {
  settingsPanel.style.display = "none";
  if (onlineReading && onlineReading.from === "store") {
    await leaveOnlineToDetail();
  }
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
  observer.observe(scrollArea, {
    childList: true,
    subtree: true,
    characterData: true,
    attributes: true,
    attributeFilter: ["style", "class"],
  });

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

// ===== Tauri 关窗钩子：先保存进度，再退出 =====
(() => {
  const raw = window.__TAURI__ || window.__TAURI_INTERNALS__;
  if (!raw || !raw.window || !raw.window.getCurrentWindow) return;
  try {
    const win = raw.window.getCurrentWindow();
    if (!win.onCloseRequested) return;
    win.onCloseRequested(async (event) => {
      if (!currentBook) return;
      event.preventDefault();
      try { await saveProgress(); } catch (e) { console.error("保存阅读进度失败", e); }
      try { await (raw.core || raw).invoke("quit_app"); } catch (e) { console.error("退出失败", e); }
    });
  } catch (e) {
    console.error("注册关窗保存失败", e);
  }
})();

// ===== 页面卸载前保存（仅浏览器模式） =====
// Tauri 模式没有 /api 路由，且 beforeunload 中无法等待异步 invoke，
// 该模式下的阅读位置由滚动防抖保存兜底。
window.addEventListener("beforeunload", () => {
  if (!currentBook) return;
  if (window.__TAURI__ || window.__TAURI_INTERNALS__) return;
  const area = document.getElementById("contentArea");
  const ratio = area.scrollHeight > area.clientHeight
    ? area.scrollTop / (area.scrollHeight - area.clientHeight) : 0;
  if (onlineReading) {
    const payload = JSON.stringify({
      storeId: onlineReading.storeId,
      lastChapter: currentIndex,
      lastPosition: ratio,
      name: onlineReading.name,
      lastItemId: (chapters[currentIndex] && chapters[currentIndex].itemId) || "",
    });
    navigator.sendBeacon("/api/store/save-progress", new Blob([payload], { type: "application/json" }));
    return;
  }
  const data = JSON.stringify({ id: currentBook.id, lastChapter: currentIndex, lastPosition: ratio });
  navigator.sendBeacon("/api/update-book", new Blob([data], { type: "application/json" }));
});

// ===== 窗口 resize 时禁用 transition，防止 WebView2 闪烁 =====
let _resizeSuppressTimer = null;
window.addEventListener("resize", () => {
  document.documentElement.classList.add("disable-transitions");
  clearTimeout(_resizeSuppressTimer);
  _resizeSuppressTimer = setTimeout(() => {
    document.documentElement.classList.remove("disable-transitions");
  }, 200);
  updateStoreAbstractScroll();
});

// 初始
hideChapterNav();
