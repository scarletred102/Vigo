// ═══════════════════════════════════════════════════════════════════════════
// VIGO — Bookmarks Manager
// Tag-based bookmarks with search
// ═══════════════════════════════════════════════════════════════════════════

const BookmarkManager = (() => {
    let bookmarks = [];

    async function load() {
        if (window.vigo) {
            bookmarks = await window.vigo.getBookmarks();
        }
    }

    async function addBookmark(url, title, tags = []) {
        if (window.vigo) {
            bookmarks = await window.vigo.addBookmark({ url, title, tags });
        } else {
            bookmarks.unshift({
                id: Date.now().toString(36),
                url, title, tags,
                createdAt: new Date().toISOString()
            });
        }
        return bookmarks;
    }

    async function removeBookmark(id) {
        if (window.vigo) {
            bookmarks = await window.vigo.removeBookmark(id);
        } else {
            bookmarks = bookmarks.filter(b => b.id !== id);
        }
        return bookmarks;
    }

    function isBookmarked(url) {
        return bookmarks.some(b => b.url === url);
    }

    function search(query) {
        if (!query) return bookmarks;
        query = query.toLowerCase();
        return bookmarks.filter(b =>
            (b.title || '').toLowerCase().includes(query) ||
            (b.url || '').toLowerCase().includes(query) ||
            (b.tags || []).some(t => t.toLowerCase().includes(query))
        );
    }

    function render(container, query = '') {
        const filtered = search(query);
        if (filtered.length === 0) {
            container.innerHTML = `
        <div class="empty-state">
          <svg width="40" height="40" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
            <path d="M19 21l-7-5-7 5V5a2 2 0 012-2h10a2 2 0 012 2z"/>
          </svg>
          <div class="empty-state-text">No bookmarks yet.<br>Click the star icon to save a page.</div>
        </div>`;
            return;
        }

        container.innerHTML = `
      <input class="panel-search" placeholder="Search bookmarks..." value="${query || ''}">
      ${filtered.map(b => `
        <div class="panel-item" data-url="${b.url}" data-id="${b.id}">
          <div class="panel-item-icon">🔖</div>
          <div class="panel-item-info">
            <div class="panel-item-title">${escapeHtml(b.title)}</div>
            <div class="panel-item-subtitle">${escapeHtml(b.url)}</div>
            ${(b.tags || []).length ? `<div style="margin-top:4px">${b.tags.map(t => `<span class="panel-tag">${t}</span>`).join('')}</div>` : ''}
          </div>
          <button class="panel-item-action" data-remove="${b.id}" title="Remove">
            <svg width="12" height="12" viewBox="0 0 12 12"><path d="M1 1l10 10M11 1L1 11" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/></svg>
          </button>
        </div>
      `).join('')}`;

        // Bind events
        container.querySelector('.panel-search')?.addEventListener('input', (e) => {
            render(container, e.target.value);
        });
        container.querySelectorAll('.panel-item[data-url]').forEach(el => {
            el.addEventListener('click', (e) => {
                if (e.target.closest('.panel-item-action')) return;
                TabManager.navigate(el.dataset.url);
            });
        });
        container.querySelectorAll('.panel-item-action[data-remove]').forEach(el => {
            el.addEventListener('click', async () => {
                await removeBookmark(el.dataset.remove);
                render(container, container.querySelector('.panel-search')?.value || '');
            });
        });
    }

    async function toggleBookmark() {
        const tab = TabManager.getActiveTab();
        if (!tab || !tab.url) return;
        if (isBookmarked(tab.url)) {
            const bm = bookmarks.find(b => b.url === tab.url);
            if (bm) await removeBookmark(bm.id);
        } else {
            const tags = prompt('Tags (comma-separated, or leave empty):');
            const tagList = tags ? tags.split(',').map(t => t.trim()).filter(Boolean) : [];
            await addBookmark(tab.url, tab.title || tab.url, tagList);
        }
        updateBookmarkIcon();
    }

    function updateBookmarkIcon() {
        const tab = TabManager.getActiveTab();
        const btn = document.getElementById('btn-bookmark-page');
        if (btn) {
            btn.classList.toggle('active', tab && isBookmarked(tab.url));
        }
    }

    function escapeHtml(str) {
        const div = document.createElement('div');
        div.textContent = str || '';
        return div.innerHTML;
    }

    function getAll() { return bookmarks; }

    return { load, addBookmark, removeBookmark, isBookmarked, search, render, toggleBookmark, updateBookmarkIcon, getAll };
})();
