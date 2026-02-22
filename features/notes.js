// ═══════════════════════════════════════════════════════════════════════════
// VIGO — Notes Panel
// Quick notes, per-site or global, with simple editor
// ═══════════════════════════════════════════════════════════════════════════

const NotesManager = (() => {
    let notes = [];

    async function load() {
        if (window.vigo) {
            notes = await window.vigo.getNotes();
        }
    }

    async function save() {
        if (window.vigo) {
            await window.vigo.saveNotes(notes);
        }
    }

    function addNote(content, url = '') {
        notes.unshift({
            id: Date.now().toString(36) + Math.random().toString(36).slice(2, 5),
            content,
            url,
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString()
        });
        save();
    }

    function updateNote(id, content) {
        const note = notes.find(n => n.id === id);
        if (note) {
            note.content = content;
            note.updatedAt = new Date().toISOString();
            save();
        }
    }

    function deleteNote(id) {
        notes = notes.filter(n => n.id !== id);
        save();
    }

    function render(container) {
        let html = `
      <div style="margin-bottom:12px">
        <button class="btn-primary" id="btn-add-note" style="width:100%">+ New Note</button>
      </div>`;

        if (notes.length === 0) {
            html += `
        <div class="empty-state">
          <svg width="40" height="40" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
            <path d="M14 2H6a2 2 0 00-2 2v16a2 2 0 002 2h12a2 2 0 002-2V8z"/>
            <polyline points="14 2 14 8 20 8"/>
          </svg>
          <div class="empty-state-text">No notes yet.<br>Jot something down!</div>
        </div>`;
        } else {
            html += notes.map(note => {
                const date = new Date(note.updatedAt).toLocaleDateString('en-US', { month: 'short', day: 'numeric', hour: '2-digit', minute: '2-digit' });
                const preview = (note.content || '').substring(0, 100);
                return `
          <div class="panel-item note-item" data-id="${note.id}">
            <div class="panel-item-icon">📝</div>
            <div class="panel-item-info">
              <div class="panel-item-title">${escapeHtml(preview) || 'Empty note'}</div>
              <div class="panel-item-subtitle">${date}${note.url ? ' — ' + escapeHtml(note.url) : ''}</div>
            </div>
            <button class="panel-item-action" data-delete="${note.id}" title="Delete">
              <svg width="12" height="12" viewBox="0 0 12 12"><path d="M1 1l10 10M11 1L1 11" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/></svg>
            </button>
          </div>`;
            }).join('');
        }

        container.innerHTML = html;

        container.querySelector('#btn-add-note')?.addEventListener('click', () => {
            const tab = TabManager.getActiveTab();
            addNote('', tab?.url || '');
            render(container);
            // Focus the first note for editing
            showEditor(container, notes[0]?.id);
        });

        container.querySelectorAll('.note-item').forEach(el => {
            el.addEventListener('click', (e) => {
                if (e.target.closest('.panel-item-action')) return;
                showEditor(container, el.dataset.id);
            });
        });

        container.querySelectorAll('.panel-item-action[data-delete]').forEach(el => {
            el.addEventListener('click', () => {
                deleteNote(el.dataset.delete);
                render(container);
            });
        });
    }

    function showEditor(container, noteId) {
        const note = notes.find(n => n.id === noteId);
        if (!note) return;

        container.innerHTML = `
      <div style="margin-bottom:12px">
        <button class="btn-secondary" id="btn-back-notes">← Back to Notes</button>
      </div>
      <textarea class="note-editor" id="note-editor" placeholder="Write your note...">${escapeHtml(note.content)}</textarea>
      <div style="margin-top:8px;font-size:11px;color:var(--text-tertiary)">
        Auto-saved • ${note.url ? 'Linked to: ' + note.url : 'Global note'}
      </div>`;

        container.querySelector('#btn-back-notes').addEventListener('click', () => render(container));
        const editor = container.querySelector('#note-editor');
        editor.focus();
        editor.addEventListener('input', () => {
            updateNote(noteId, editor.value);
        });
    }

    function escapeHtml(str) {
        const div = document.createElement('div');
        div.textContent = str || '';
        return div.innerHTML;
    }

    return { load, render, addNote };
})();
