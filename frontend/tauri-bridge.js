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
  importBook:    (name, content)    => call("import_book", { name, content }),
  openSaved:     (id)                => call("open_saved", { id }),
  updateBook:    (id, data)          => call("update_book", { id, ...data }),
  deleteBook:    (id)                => call("delete_book", { id }),
  renameBook:    (id, newName)       => call("rename_book", { id, newName }),
};

window.api = api;
})();
