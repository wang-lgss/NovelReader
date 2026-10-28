// Tauri / Node.js 双模式桥接层
(function() {

const _isTauri = typeof window !== "undefined" && !!(window.__TAURI__ || window.__TAURI_INTERNALS__);

async function call(cmd, args = {}) {
  if (_isTauri) {
    const ta = window.__TAURI__ || window.__TAURI_INTERNALS__;
    const invoke = (ta.core || ta).invoke;
    try {
      return await invoke(cmd, args);
    } catch(e) {
      return { error: e.message || String(e) };
    }
  } else {
    // Node.js 模式，映射到 HTTP API
    const routes = {
      get_books:     ["GET",  "/api/books"],
      get_settings:  ["GET",  "/api/settings"],
      save_settings: ["POST", "/api/settings"],
      import_book:   ["POST", "/api/import-book"],
      open_saved:    ["POST", "/api/open-saved"],
      search_store:  ["POST", "/api/store/search"],
      store_info:    ["POST", "/api/store/info"],
      store_chapters:["POST", "/api/store/chapters"],
      store_contents:["POST", "/api/store/contents"],
      store_download_contents:["POST", "/api/store/download-contents"],
      rank_books:    ["POST", "/api/store/rank"],
      cover_jpeg:    ["POST", "/api/cover"],
      cover_batch:   ["POST", "/api/cover-batch"],
      add_store_book:["POST", "/api/store/add-book"],
      get_store_progress:["POST", "/api/store/get-progress"],
      save_store_progress:["POST", "/api/store/save-progress"],
      update_book:   ["POST", "/api/update-book"],
      delete_book:   ["POST", "/api/delete-book"],
      rename_book:   ["POST", "/api/rename-book"],
    };
    const [method, url] = routes[cmd] || ["GET", `/api/${cmd}`];
    const opts = { headers: { "Content-Type": "application/json" } };
    if (method === "POST") {
      opts.method = "POST";
      opts.body = JSON.stringify(args);
    }
    const resp = await fetch(url, opts);
    return await resp.json();
  }
}

const api = {
  getBooks:      ()                  => call("get_books"),
  getSettings:   ()                  => call("get_settings"),
  saveSettings:  (s)                 => call("save_settings", s),
  importBook:    (name, content, overwrite) => call("import_book", { name, content, overwrite: Boolean(overwrite) }),
  openSaved:     (id)                => call("open_saved", { id }),
  searchStore:   (query)             => call("search_store", { query }),
  storeInfo:     (bookId)            => call("store_info", { bookId }),
  storeChapters: (bookId)            => call("store_chapters", { bookId }),
  storeContents: (bookId, itemIds)   => call("store_contents", { bookId, itemIds }),
  storeDownloadContents: (bookId, itemIds) => call("store_download_contents", { bookId, itemIds }),
  rankBooks:     (algo)              => call("rank_books", { algo }),
  coverJpeg:     (url)               => call("cover_jpeg", { url }),
  coverBatch:    (urls)              => call("cover_batch", { urls }),
  addStoreBook:  (storeId, name, replace) => call("add_store_book", { storeId, name, replace: Boolean(replace) }),
  getStoreProgress: (storeId)        => call("get_store_progress", { storeId }),
  saveStoreProgress: (storeId, lastChapter, lastPosition, name, lastItemId) => call("save_store_progress", { storeId, lastChapter, lastPosition, name, lastItemId }),
  saveTxtDialog:(fileName, content)  => call("save_txt_dialog", { fileName, content }),
  updateBook:    (id, data)          => call("update_book", { id, ...data }),
  deleteBook:    (id)                => call("delete_book", { id }),
  renameBook:    (id, newName)       => call("rename_book", { id, newName }),
};

window.api = api;
})();
