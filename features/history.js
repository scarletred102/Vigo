// ═══════════════════════════════════════════════════════════════════════════
// VIGO — History Manager
// Visual timeline grouped by day with search
// ═══════════════════════════════════════════════════════════════════════════

const HistoryManager = (() => {
    let history = [];

    async function load() {
        if (window.vigo) {
            history = await window.vigo.getHistory();
        }
    }

    function groupByDay(entries) {
        const groups = {};
        const today = new Date().toDateString();
        const yesterday = new Date(Date.now() - 86400000).toDateString();

        entries.forEach(entry => {
            const date = new Date(entry.visitedAt);
            const dateStr = date.toDateString();
            let label = dateStr;
            if (dateStr === today) label = 'Today';
            else if (dateStr === yesterday) label = 'Yesterday';
            else label = date.toLocaleDateString('en-US', { weekday: 'long', month: 'short', day: 'numeric' });

            if (!groups[label]) groups[label] = [];
            groups[label].push(entry);
        });
        return groups;
    }

    function search(query) {
        if (!query) return history;
        query = query.toLowerCase();
        return history.filter(h =>
            (h.title || '').toLowerCase().includes(query) ||
            (h.url || '').toLowerCase().includes(query)
        );
    }

    function render(container, query = '') {
        const filtered = search(query);
        if (filtered.length === 0) {
            container.innerHTML = `
        <div class="empty-state">
          <svg width="40" height="40" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
            <circle cx="12" cy="12" r="10"/><polyline points="12 6 12 12 16 14"/>
          </svg>
          <div class="empty-state-text">No browsing history yet.</div>
        </div>`;
            return;
        }

        const groups = groupByDay(filtered);
        let html = `<input class="panel-search" placeholder="Search history..." value="${query || ''}">`;

        for (const [day, entries] of Object.entries(groups)) {
            html += `<div class="panel-section-title">${day}</div>`;
            entries.forEach(entry => {
                const time = new Date(entry.visitedAt).toLocaleTimeString('en-US', { hour: '2-digit', minute: '2-digit' });
                html += `
          <div class="panel-item" data-url="${escapeHtml(entry.url)}">
            <div class="panel-item-icon">🕐</div>
            <div class="panel-item-info">
              <div class="panel-item-title">${escapeHtml(entry.title || entry.url)}</div>
              <div class="panel-item-subtitle">${time} — ${escapeHtml(entry.url)}</div>
            </div>
          </div>`;
            });
        }

        html += `<div style="padding:12px;text-align:center">
      <button class="btn-secondary" id="btn-clear-history">Clear All History</button>
    </div>`;

        container.innerHTML = html;

        container.querySelector('.panel-search')?.addEventListener('input', (e) => {
            render(container, e.target.value);
        });
        container.querySelectorAll('.panel-item[data-url]').forEach(el => {
            el.addEventListener('click', () => TabManager.navigate(el.dataset.url));
        });
        container.querySelector('#btn-clear-history')?.addEventListener('click', async () => {
            if (window.vigo) await window.vigo.clearHistory();
            history = [];
            render(container);
        });
    }

    function escapeHtml(str) {
        const div = document.createElement('div');
        div.textContent = str || '';
        return div.innerHTML;
    }

    return { load, render, search };
})();
