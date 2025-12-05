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
    
    // Add smooth scroll
    document.querySelectorAll('a[href^="#"]').forEach(anchor => {
        anchor.addEventListener('click', function (e) {
            e.preventDefault();
            const target = document.querySelector(this.getAttribute('href'));
            if (target) {
                target.scrollIntoView({
                    behavior: 'smooth',
                    block: 'start'
                });
            }
        });
    });
    
    // Add terminal connection status indicator
    function checkTerminalConnection() {
        // Simple visual feedback
        const terminalContainer = document.querySelector('.terminal-container');
        
        // Try to detect if iframe is responsive
        try {
            if (terminal.contentWindow) {
                terminalContainer.classList.remove('loading');
            }
        } catch {
            // Cross-origin, but that's expected
            terminalContainer.classList.remove('loading');
        }
    }
    
    // Check connection after a delay
    setTimeout(checkTerminalConnection, 2000);
    
    // Add copy button for GitHub link
    const githubLink = document.querySelector('a[href*="github"]');
    if (githubLink) {
        githubLink.addEventListener('click', function() {
            // Track click (if you add analytics later)
            console.log('GitHub link clicked');
        });
    }
    
    // Intersection Observer for scroll animations
    const observerOptions = {
        threshold: 0.1,
        rootMargin: '0px 0px -50px 0px'
    };
    
    const observer = new IntersectionObserver((entries) => {
        entries.forEach(entry => {
            if (entry.isIntersecting) {
                entry.target.style.opacity = '1';
                entry.target.style.transform = 'translateY(0)';
            }
        });
    }, observerOptions);
    
    // Observe future items
    document.querySelectorAll('.future-item').forEach((item, index) => {
        item.style.opacity = '0';
        item.style.transform = 'translateY(30px)';
        item.style.transition = `all 0.6s ease-out ${index * 0.1}s`;
        observer.observe(item);
    });
    
    // Observe tech items
    document.querySelectorAll('.tech-item').forEach((item, index) => {
        item.style.opacity = '0';
        item.style.transform = 'translateY(30px)';
        item.style.transition = `all 0.6s ease-out ${index * 0.1}s`;
        observer.observe(item);
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
