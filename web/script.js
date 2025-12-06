// Configuration constants
const CONFIG = {
    IFRAME_LOAD_DELAY: 50,
    RESIZE_ATTEMPTS: [50, 150, 300, 600, 1000],
    RESIZE_DEBOUNCE: 250,
    TERMINAL_URL_LOCAL: 'http://localhost:7681',
    TERMINAL_URL_PROD: '/terminal/',
};

// Utility: Safely dispatch resize event to iframe
function dispatchIframeResize(iframe) {
    try {
        if (iframe?.contentWindow) {
            iframe.contentWindow.dispatchEvent(new Event('resize'));
        }
    } catch (e) {
        // Cross-origin restriction, silently ignore
    }
}

// Utility: Check if running in production
function isProduction() {
    const hostname = window.location.hostname;
    return hostname !== 'localhost' && hostname !== '127.0.0.1';
}

// Main initialization
document.addEventListener('DOMContentLoaded', function() {
    const terminal = document.getElementById('terminal');
    const fullscreenBtn = document.getElementById('fullscreen-btn');
    
    if (!terminal || !fullscreenBtn) {
        console.error('Required DOM elements not found');
        return;
    }
    
    // Set iframe source based on environment
    setTimeout(() => {
        terminal.src = isProduction() 
            ? CONFIG.TERMINAL_URL_PROD 
            : CONFIG.TERMINAL_URL_LOCAL;
    }, CONFIG.IFRAME_LOAD_DELAY);
    
    // Handle iframe loading and terminal resizing
    terminal.addEventListener('load', function() {
        console.log('Terminal loaded successfully');
        
        // Force terminal resize after load with multiple attempts
        // This helps xterm.js inside ttyd recalculate its dimensions
        CONFIG.RESIZE_ATTEMPTS.forEach(delay => {
            setTimeout(() => {
                window.dispatchEvent(new Event('resize'));
                dispatchIframeResize(terminal);
                
                // Force reflow to ensure layout recalculation
                void terminal.offsetHeight;
            }, delay);
        });
    });
    
    terminal.addEventListener('error', function() {
        console.error('Failed to load terminal');
        const container = terminal.parentElement;
        container.innerHTML = `
            <div style="padding: 40px; text-align: center; color: #ff5f56;">
                <h3>Terminal Unavailable</h3>
                <p>The terminal service is currently unavailable. Please try again later.</p>
            </div>
        `;
    });
    
    // Fullscreen terminal
    fullscreenBtn.addEventListener('click', function(e) {
        e.preventDefault();
        const terminalUrl = terminal.src;
        window.open(terminalUrl, '_blank', 'width=1200,height=800');
    });
    
    // Add keyboard shortcut hint
    document.addEventListener('keydown', function(e) {
        // Ctrl/Cmd + K to focus terminal
        if ((e.ctrlKey || e.metaKey) && e.key === 'k') {
            e.preventDefault();
            terminal.focus();
        }
    });
    
    // Handle window resize to keep terminal properly sized
    let resizeTimeout;
    window.addEventListener('resize', function() {
        clearTimeout(resizeTimeout);
        resizeTimeout = setTimeout(() => {
            dispatchIframeResize(terminal);
            void terminal.offsetHeight; // Force reflow
        }, CONFIG.RESIZE_DEBOUNCE);
    });
});

// Console easter egg for curious developers
console.log(
    '%c _____ _     _       \n' +
    '|  ___(_) __| | ___  \n' +
    '| |_  | |/ _` |/ _ \\ \n' +
    '|  _| | | (_| | (_) |\n' +
    '|_|   |_|\\__,_|\\___/ \n',
    'font-family: monospace; color: #0ea5e9;'
);
console.log(
    '%cWelcome to Fido! 🐕\n' +
    'Built with Rust + Ratatui\n' +
    'Press Ctrl+K to focus the terminal\n\n' +
    '🔄 Resurrecting BBS culture for the modern age',
    'font-family: monospace; color: #a3a3a3;'
);
