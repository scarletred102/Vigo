// ═══════════════════════════════════════════════════════════════════════════
// VIGO — Reader Mode
// Clean reading view with adjustable typography
// ═══════════════════════════════════════════════════════════════════════════

const ReaderMode = (() => {
    let active = false;

    function toggle() {
        active = !active;
        const container = document.getElementById('webview-container');
        let overlay = container.querySelector('.reader-overlay');

        if (active) {
            if (!overlay) {
                overlay = document.createElement('div');
                overlay.className = 'reader-overlay';
                container.appendChild(overlay);
            }
            extractContent(overlay);
            overlay.classList.add('visible');
        } else {
            if (overlay) overlay.classList.remove('visible');
        }

        document.getElementById('btn-reader-mode').classList.toggle('active', active);
    }

    function extractContent(overlay) {
        const wv = document.getElementById(`wv-${TabManager.getActiveTabId()}`);
        if (!wv) {
            overlay.innerHTML = '<div class="reader-content"><h1>No content available</h1><p>Navigate to a page first, then enable reader mode.</p></div>';
            return;
        }

        wv.executeJavaScript(`
      (function() {
        let title = document.title || '';
        let content = '';
        // Try article tag first
        let article = document.querySelector('article');
        if (!article) article = document.querySelector('[role="main"]');
        if (!article) article = document.querySelector('main');
        if (!article) article = document.querySelector('.post-content, .entry-content, .article-body, .story-body');

        if (article) {
          content = article.innerHTML;
        } else {
          // Fallback: collect all paragraphs
          const paras = document.querySelectorAll('p');
          content = Array.from(paras).map(p => '<p>' + p.textContent + '</p>').join('');
        }
        return JSON.stringify({ title, content });
      })()
    `).then(result => {
            try {
                const data = JSON.parse(result);
                overlay.innerHTML = `
          <div class="reader-content">
            <h1>${data.title}</h1>
            <div>${data.content || '<p>Could not extract readable content from this page.</p>'}</div>
          </div>`;
            } catch {
                overlay.innerHTML = '<div class="reader-content"><h1>Reader Mode</h1><p>Could not parse page content.</p></div>';
            }
        }).catch(() => {
            overlay.innerHTML = '<div class="reader-content"><h1>Reader Mode</h1><p>Could not access page content.</p></div>';
        });
    }

    function isActive() { return active; }

    return { toggle, isActive };
})();
