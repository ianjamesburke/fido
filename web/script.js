// Update iframe src based on environment
document.addEventListener('DOMContentLoaded', function() {
    const terminal = document.getElementById('terminal');
    const fullscreenBtn = document.getElementById('fullscreen-btn');
    
    // Detect if we're in production or local
    const isProduction = window.location.hostname !== 'localhost' && window.location.hostname !== '127.0.0.1';
    
    if (isProduction) {
        // In production, use relative path (nginx will proxy)
        terminal.src = '/terminal/';
    } else {
        // Local development
        terminal.src = 'http://localhost:7681';
    }
    
    // Handle iframe loading
    terminal.addEventListener('load', function() {
        console.log('Terminal loaded successfully');
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
    

});

// Add console easter egg
console.log(`
 _____ _     _       
|  ___(_) __| | ___  
| |_  | |/ _\` |/ _ \\ 
|  _| | | (_| | (_) |
|_|   |_|\\__,_|\\___/ 

Welcome to Fido! 🐕
Built with Rust + Ratatui
Press Ctrl+K to focus the terminal

🔄 Resurrecting BBS culture for the modern age
`);
