// ═══════════════════════════════════════════════════════════════════════════
// VIGO — New Tab Page
// Speed dial, clock, greeting, and search
// ═══════════════════════════════════════════════════════════════════════════

const NewTabPage = (() => {
    const speedDialDefaults = [
        { name: 'Google', url: 'https://google.com', color: '#4285f4', letter: 'G' },
        { name: 'YouTube', url: 'https://youtube.com', color: '#ff0000', letter: 'Y' },
        { name: 'GitHub', url: 'https://github.com', color: '#8b5cf6', letter: 'G' },
        { name: 'Reddit', url: 'https://reddit.com', color: '#ff4500', letter: 'R' },
        { name: 'Twitter', url: 'https://x.com', color: '#1da1f2', letter: 'X' },
        { name: 'Wikipedia', url: 'https://wikipedia.org', color: '#636363', letter: 'W' },
        { name: 'Stack Overflow', url: 'https://stackoverflow.com', color: '#f48024', letter: 'S' },
        { name: 'LinkedIn', url: 'https://linkedin.com', color: '#0077b5', letter: 'L' },
    ];

    let clockInterval;

    function init() {
        renderGreeting();
        updateClock();
        clockInterval = setInterval(updateClock, 1000);
        renderSpeedDial();
        bindSearch();
    }

    function renderGreeting() {
        const hour = new Date().getHours();
        let greeting = 'Good evening';
        if (hour < 12) greeting = 'Good morning';
        else if (hour < 18) greeting = 'Good afternoon';
        document.getElementById('newtab-greeting').textContent = greeting;
    }

    function updateClock() {
        const now = new Date();
        const h = now.getHours().toString().padStart(2, '0');
        const m = now.getMinutes().toString().padStart(2, '0');
        document.getElementById('newtab-clock').textContent = `${h}:${m}`;
    }

    function renderSpeedDial() {
        const container = document.getElementById('speed-dial');
        container.innerHTML = speedDialDefaults.map(item => `
      <div class="speed-dial-item" data-url="${item.url}">
        <div class="speed-dial-icon" style="background:${item.color}20;color:${item.color}">${item.letter}</div>
        <span class="speed-dial-label">${item.name}</span>
      </div>
    `).join('');

        container.querySelectorAll('.speed-dial-item').forEach(el => {
            el.addEventListener('click', () => {
                TabManager.navigate(el.dataset.url);
            });
        });
    }

    function bindSearch() {
        const searchInput = document.getElementById('newtab-search');
        searchInput.addEventListener('keydown', (e) => {
            if (e.key === 'Enter' && searchInput.value.trim()) {
                TabManager.navigate(searchInput.value.trim());
                searchInput.value = '';
            }
        });
    }

    return { init };
})();
