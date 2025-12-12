// Web Terminal Interface JavaScript - Terminal-First Approach
// The terminal handles all authentication - no web-based login flow

document.addEventListener('DOMContentLoaded', function() {
    const terminal = document.getElementById('terminal');

    // Terminal iframe load handler
    if (terminal) {
        terminal.addEventListener('load', function() {
            console.log('Terminal loaded successfully');
            focusTerminal();
        });
    }

    // Keyboard shortcuts
    document.addEventListener('keydown', function(e) {
        // Ctrl+K to focus terminal
        if (e.ctrlKey && e.key === 'k') {
            e.preventDefault();
            focusTerminal();
        }
    });

    // Simple terminal focus function
    function focusTerminal() {
        if (terminal) {
            terminal.focus();
            
            // Try to focus the content inside the iframe
            try {
                if (terminal.contentWindow) {
                    terminal.contentWindow.focus();
                }
            } catch (e) {
                // Cross-origin restrictions may prevent this
                console.log('Could not focus terminal content (cross-origin)');
            }
        }
    }

    // Auto-focus terminal on page load
    setTimeout(focusTerminal, 1000);
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
    'Press Ctrl+K to focus the terminal\n' +
    'All authentication happens in the terminal below\n\n' +
    '🔄 Terminal-first experience - same as native Fido',
    'font-family: monospace; color: #a3a3a3;'
);