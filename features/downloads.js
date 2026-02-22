// ═══════════════════════════════════════════════════════════════════════════
// VIGO — Downloads Manager
// Download tracking with progress bars
// ═══════════════════════════════════════════════════════════════════════════

const DownloadManager = (() => {
    let downloads = [];

    function render(container) {
        if (downloads.length === 0) {
            container.innerHTML = `
        <div class="empty-state">
          <svg width="40" height="40" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
            <path d="M21 15v4a2 2 0 01-2 2H5a2 2 0 01-2-2v-4"/>
            <polyline points="7 10 12 15 17 10"/>
            <line x1="12" y1="15" x2="12" y2="3"/>
          </svg>
          <div class="empty-state-text">No downloads yet.<br>Downloads will appear here.</div>
        </div>`;
            return;
        }

        container.innerHTML = downloads.map(dl => `
      <div class="panel-item">
        <div class="panel-item-icon">📥</div>
        <div class="panel-item-info">
          <div class="panel-item-title">${dl.filename}</div>
          <div class="panel-item-subtitle">${dl.status === 'complete' ? 'Completed' : dl.status}</div>
          ${dl.status === 'downloading' ? `
            <div class="download-progress">
              <div class="download-progress-bar" style="width:${dl.progress}%"></div>
            </div>` : ''}
        </div>
      </div>
    `).join('');
    }

    function addDownload(filename, url) {
        downloads.unshift({
            id: Date.now().toString(36),
            filename,
            url,
            status: 'complete',
            progress: 100,
            startedAt: new Date().toISOString()
        });
    }

    return { render, addDownload };
})();
