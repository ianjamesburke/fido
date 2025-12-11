// Web Terminal Interface JavaScript
document.addEventListener('DOMContentLoaded', function() {
    const authContainer = document.getElementById('auth-container');
    const terminalContainer = document.getElementById('terminal-container');
    const terminal = document.getElementById('terminal');
    const demoBtn = document.getElementById('demo-btn');
    const githubBtn = document.getElementById('github-btn');

    // Check if user is already authenticated
    checkAuthStatus();

    // Demo button handler
    demoBtn.addEventListener('click', function() {
        startDemoMode();
    });

    // GitHub login button handler
    githubBtn.addEventListener('click', function() {
        startGitHubAuth();
    });

    // Terminal iframe load handler
    terminal.addEventListener('load', function() {
        console.log('Terminal loaded successfully');
        focusTerminal();
    });

    // Keyboard shortcuts
    document.addEventListener('keydown', function(e) {
        // Ctrl+K to focus terminal
        if (e.ctrlKey && e.key === 'k') {
            e.preventDefault();
            focusTerminal();
        }
        
        // Escape to show auth options (if terminal is visible)
        if (e.key === 'Escape' && terminalContainer.style.display !== 'none') {
            showAuthOptions();
        }
    });

    function checkAuthStatus() {
        // Check for existing session
        const sessionToken = localStorage.getItem('fido_session_token');
        const authMode = localStorage.getItem('fido_auth_mode');
        
        if (sessionToken && authMode) {
            showTerminal();
        }
    }

    function startDemoMode() {
        // Show confirmation dialog with clear warnings
        if (!confirmDemoMode()) {
            return;
        }

        // Set demo mode in localStorage for the TUI to detect
        localStorage.setItem('fido_auth_mode', 'demo');
        localStorage.setItem('fido_session_token', 'demo_' + Date.now());
        localStorage.setItem('fido_demo_start_time', Date.now().toString());
        
        // Update terminal URL to include demo mode
        terminal.src = '/terminal/?mode=demo';
        showTerminal();
        
        // Show demo mode notification
        showDemoModeNotification();
        
        console.log('Started demo mode - test data will reset on page reload');
    }

    function confirmDemoMode() {
        return confirm(
            "🚨 CRITICAL WARNING: DEMO MODE 🚨\n\n" +
            "⚠️ EVERYTHING YOU DO WILL BE COMPLETELY LOST ⚠️\n\n" +
            "Demo Mode Limitations:\n" +
            "• ALL DATA IS TEMPORARY - Posts, messages, votes, settings\n" +
            "• Data is LOST when you refresh, close tab, or navigate away\n" +
            "• Your actions are ISOLATED from real users\n" +
            "• Test data RESETS automatically and frequently\n" +
            "• NO account is created or saved\n" +
            "• NO way to recover lost demo data\n\n" +
            "✅ Perfect for: Exploring features, testing interface, learning shortcuts\n" +
            "❌ NOT suitable for: Real conversations, building reputation, permanent content\n\n" +
            "For permanent data and real community access, use 'Login with GitHub' instead.\n\n" +
            "Do you understand these limitations and want to continue with Demo Mode?"
        );
    }

    function showDemoModeNotification() {
        // Create and show demo mode notification
        const notification = createNotification(
            "🚨 Demo Mode Active - Data Will Be Lost",
            "⚠️ IMPORTANT: All posts, messages, votes, and settings are TEMPORARY and will be COMPLETELY RESET when you refresh, close this tab, or navigate away. This is isolated test data that doesn't affect real users. For permanent data and real community access, press 'Escape' and choose 'Login with GitHub'.",
            "warning",
            true // persistent
        );
        
        // Make it more prominent
        notification.style.borderColor = '#dc2626';
        notification.style.borderWidth = '2px';
        notification.style.boxShadow = '0 0 20px rgba(220, 38, 38, 0.3)';
        
        // Keep it visible but slightly fade after 15 seconds
        setTimeout(() => {
            if (notification && notification.parentNode) {
                notification.style.opacity = '0.9';
            }
        }, 15000);
    }

    async function startGitHubAuth() {
        try {
            // Step 1: Request device code from API
            const deviceResponse = await fetch('/auth/github/device', {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json',
                },
            });

            if (!deviceResponse.ok) {
                throw new Error(`Failed to get device code: ${deviceResponse.status}`);
            }

            const deviceData = await deviceResponse.json();
            
            // Step 2: Show user code and verification instructions
            showDeviceCodeModal(deviceData);
            
            // Step 3: Start polling for authorization
            pollForAuthorization(deviceData.device_code, deviceData.interval);
            
        } catch (error) {
            console.error('GitHub authentication error:', error);
            createNotification(
                "❌ GitHub Login Failed",
                `Failed to start GitHub authentication: ${error.message}. Please try again.`,
                "error",
                false
            );
        }
    }

    function showTerminal() {
        authContainer.style.display = 'none';
        terminalContainer.style.display = 'block';
        
        // Update terminal URL based on authentication mode
        const authMode = localStorage.getItem('fido_auth_mode');
        const sessionToken = localStorage.getItem('fido_session_token');
        
        if (authMode === 'github' && sessionToken) {
            // For GitHub auth, pass the session token to the terminal
            terminal.src = `/terminal/?session_token=${encodeURIComponent(sessionToken)}&mode=github`;
        } else if (authMode === 'demo') {
            // For demo mode, use demo mode parameter
            terminal.src = '/terminal/?mode=demo';
        } else {
            // Default fallback
            terminal.src = '/terminal/';
        }
        
        // Focus terminal after a short delay to ensure it's loaded
        setTimeout(focusTerminal, 500);
    }

    function showAuthOptions() {
        const currentMode = localStorage.getItem('fido_auth_mode');
        
        // Show data reset notification if coming from demo mode
        if (currentMode === 'demo') {
            showDataResetNotification();
        }
        
        authContainer.style.display = 'block';
        terminalContainer.style.display = 'none';
        
        // Clear session data
        localStorage.removeItem('fido_session_token');
        localStorage.removeItem('fido_auth_mode');
        localStorage.removeItem('fido_demo_start_time');
        
        // Clear any existing notifications
        clearNotifications();
    }

    function showDataResetNotification() {
        createNotification(
            "🔄 Demo Data Successfully Reset",
            "✅ All test data has been completely cleared: posts, messages, votes, settings, and preferences have been reset to provide a clean experience. This demonstrates the temporary nature of demo mode. To keep your data permanently, use 'Login with GitHub' for your next session.",
            "info",
            false // not persistent
        );
    }

    function focusTerminal() {
        if (terminalContainer.style.display !== 'none') {
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

    // Handle GitHub OAuth callback
    const urlParams = new URLSearchParams(window.location.search);
    if (urlParams.get('auth') === 'success') {
        const token = urlParams.get('token');
        if (token) {
            localStorage.setItem('fido_session_token', token);
            localStorage.setItem('fido_auth_mode', 'github');
            
            // Clean up URL
            window.history.replaceState({}, document.title, window.location.pathname);
            
            showTerminal();
            
            // Show GitHub login success notification
            createNotification(
                "✅ GitHub Login Successful",
                "Welcome to Fido! You're now connected to your real account. Your posts and messages will be saved permanently.",
                "success",
                false
            );
        }
    }

    // Notification System
    function createNotification(title, message, type = 'info', persistent = false) {
        // Remove existing notifications if not persistent
        if (!persistent) {
            clearNotifications();
        }

        const notification = document.createElement('div');
        notification.className = `notification notification-${type}`;
        
        notification.innerHTML = `
            <div class="notification-content">
                <div class="notification-title">${title}</div>
                <div class="notification-message">${message}</div>
            </div>
            <button class="notification-close" onclick="this.parentElement.remove()">×</button>
        `;

        document.body.appendChild(notification);

        // Auto-remove non-persistent notifications after 8 seconds
        if (!persistent) {
            setTimeout(() => {
                if (notification.parentNode) {
                    notification.style.opacity = '0';
                    notification.style.transform = 'translateX(100%)';
                    setTimeout(() => notification.remove(), 300);
                }
            }, 8000);
        }

        return notification;
    }

    function clearNotifications() {
        const notifications = document.querySelectorAll('.notification');
        notifications.forEach(notification => {
            if (!notification.classList.contains('notification-persistent')) {
                notification.remove();
            }
        });
    }

    // GitHub Device Flow functions
    function showDeviceCodeModal(deviceData) {
        // Create modal overlay
        const modal = document.createElement('div');
        modal.className = 'device-code-modal';
        modal.innerHTML = `
            <div class="device-code-content">
                <h3>🔐 GitHub Authentication</h3>
                <p>To complete your login, follow these steps:</p>
                <ol>
                    <li>Go to: <a href="${deviceData.verification_uri}" target="_blank" class="verification-link">${deviceData.verification_uri}</a></li>
                    <li>Enter this code: <span class="user-code">${deviceData.user_code}</span></li>
                    <li>Authorize Fido in your GitHub account</li>
                </ol>
                <div class="device-code-status">
                    <div class="spinner"></div>
                    <span>Waiting for authorization...</span>
                </div>
                <button class="cancel-auth-btn">Cancel</button>
            </div>
        `;

        // Add modal styles
        const style = document.createElement('style');
        style.textContent = `
            .device-code-modal {
                position: fixed;
                top: 0;
                left: 0;
                width: 100%;
                height: 100%;
                background: rgba(0, 0, 0, 0.8);
                display: flex;
                justify-content: center;
                align-items: center;
                z-index: 1000;
            }
            .device-code-content {
                background: #1a1a1a;
                border: 1px solid #333;
                border-radius: 8px;
                padding: 2rem;
                max-width: 500px;
                text-align: center;
                color: #fff;
            }
            .verification-link {
                color: #0ea5e9;
                text-decoration: none;
                font-weight: bold;
            }
            .verification-link:hover {
                text-decoration: underline;
            }
            .user-code {
                font-family: monospace;
                font-size: 1.5rem;
                font-weight: bold;
                color: #10b981;
                background: #0f172a;
                padding: 0.5rem 1rem;
                border-radius: 4px;
                display: inline-block;
                margin: 0.5rem;
                border: 1px solid #334155;
            }
            .device-code-status {
                margin: 1.5rem 0;
                display: flex;
                align-items: center;
                justify-content: center;
                gap: 0.5rem;
            }
            .spinner {
                width: 20px;
                height: 20px;
                border: 2px solid #333;
                border-top: 2px solid #0ea5e9;
                border-radius: 50%;
                animation: spin 1s linear infinite;
            }
            @keyframes spin {
                0% { transform: rotate(0deg); }
                100% { transform: rotate(360deg); }
            }
            .cancel-auth-btn {
                background: #dc2626;
                color: white;
                border: none;
                padding: 0.5rem 1rem;
                border-radius: 4px;
                cursor: pointer;
            }
            .cancel-auth-btn:hover {
                background: #b91c1c;
            }
        `;
        document.head.appendChild(style);

        // Add cancel functionality
        const cancelBtn = modal.querySelector('.cancel-auth-btn');
        cancelBtn.addEventListener('click', () => {
            document.body.removeChild(modal);
            document.head.removeChild(style);
            // Stop polling by setting a flag
            window.authCancelled = true;
        });

        document.body.appendChild(modal);
        
        // Store modal reference for later removal
        window.currentAuthModal = { modal, style };
    }

    async function pollForAuthorization(deviceCode, interval) {
        // Reset cancellation flag
        window.authCancelled = false;
        
        const poll = async () => {
            if (window.authCancelled) {
                return;
            }

            try {
                const response = await fetch('/auth/github/device/poll', {
                    method: 'POST',
                    headers: {
                        'Content-Type': 'application/json',
                    },
                    body: JSON.stringify({ device_code: deviceCode }),
                });

                if (response.ok) {
                    // Success! User authorized
                    const authData = await response.json();
                    
                    // Store session data
                    localStorage.setItem('fido_session_token', authData.session_token);
                    localStorage.setItem('fido_auth_mode', 'github');
                    
                    // Create web session for the terminal
                    try {
                        await createWebSession(authData.user.id, 'real');
                        
                        // Also write session info to a temporary file for the terminal to read
                        await writeSessionFile(authData.session_token, authData.user);
                    } catch (e) {
                        console.error('Failed to create web session:', e);
                    }
                    
                    // Remove modal
                    if (window.currentAuthModal) {
                        document.body.removeChild(window.currentAuthModal.modal);
                        document.head.removeChild(window.currentAuthModal.style);
                        window.currentAuthModal = null;
                    }
                    
                    // Show terminal with GitHub session
                    showTerminal();
                    
                    // Show success notification
                    createNotification(
                        "✅ GitHub Login Successful",
                        `Welcome ${authData.user.username}! You're now connected to your real account.`,
                        "success",
                        false
                    );
                    
                } else {
                    const errorData = await response.text();
                    
                    if (errorData.includes('authorization_pending')) {
                        // Still waiting for user authorization, continue polling
                        setTimeout(poll, interval * 1000);
                    } else {
                        // Real error occurred
                        throw new Error(errorData);
                    }
                }
            } catch (error) {
                console.error('Polling error:', error);
                
                // Remove modal
                if (window.currentAuthModal) {
                    document.body.removeChild(window.currentAuthModal.modal);
                    document.head.removeChild(window.currentAuthModal.style);
                    window.currentAuthModal = null;
                }
                
                createNotification(
                    "❌ GitHub Authentication Failed",
                    `Authentication failed: ${error.message}. Please try again.`,
                    "error",
                    false
                );
            }
        };

        // Start polling after the specified interval
        setTimeout(poll, interval * 1000);
    }

    // Check for demo mode timeout and show warnings
    function checkDemoModeTimeout() {
        const authMode = localStorage.getItem('fido_auth_mode');
        const startTime = localStorage.getItem('fido_demo_start_time');
        
        if (authMode === 'demo' && startTime) {
            const elapsed = Date.now() - parseInt(startTime);
            const minutes = Math.floor(elapsed / 60000);
            
            // Show escalating warnings
            if (minutes >= 10 && minutes < 11) {
                createNotification(
                    "⏰ Demo Mode: 10 Minutes Used",
                    "You've been exploring demo mode for 10 minutes. Remember: all your test data (posts, messages, settings) will be completely lost when you refresh or close this page. Consider creating a real account with 'Login with GitHub' to keep your data permanently.",
                    "warning",
                    false
                );
            } else if (minutes >= 20 && minutes < 21) {
                createNotification(
                    "⚠️ Demo Mode: 20 Minutes - Data Still Temporary",
                    "You've been using demo mode for 20 minutes. All your activity is still temporary and will be lost. If you're enjoying Fido and want to join the real community, press 'Escape' and choose 'Login with GitHub' to create a permanent account.",
                    "warning",
                    false
                );
            } else if (minutes >= 30 && minutes < 31) {
                createNotification(
                    "🚨 Demo Mode: 30 Minutes - Consider Real Account",
                    "You've been in demo mode for 30 minutes! That's a lot of temporary data that will be lost. If you're ready to join the real Fido community and keep your data permanently, press 'Escape' and 'Login with GitHub'. Your demo session may be reset soon.",
                    "warning",
                    true // Make this one persistent
                );
            }
        }
    }

    // Check demo timeout every minute
    setInterval(checkDemoModeTimeout, 60000);

    // Warn users before they leave the page in demo mode
    window.addEventListener('beforeunload', function(e) {
        const authMode = localStorage.getItem('fido_auth_mode');
        if (authMode === 'demo') {
            const message = 'You are in Demo Mode. All your test data (posts, messages, settings) will be permanently lost if you leave this page. Are you sure you want to continue?';
            e.preventDefault();
            e.returnValue = message;
            return message;
        }
    });

    // Helper function to create a web session for the terminal
    async function createWebSession(userId, userType) {
        const response = await fetch('/web/session', {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json',
            },
            body: JSON.stringify({
                user_id: userId,
                user_type: userType
            }),
        });

        if (!response.ok) {
            throw new Error(`Failed to create web session: ${response.status}`);
        }

        const sessionData = await response.json();
        
        // Store the web session token for the terminal to use
        localStorage.setItem('fido_web_session_token', sessionData.session_token);
        
        return sessionData;
    }

    // Helper function to write session info to a temporary file
    async function writeSessionFile(sessionToken, user) {
        try {
            const response = await fetch('/web/write-session', {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json',
                },
                body: JSON.stringify({
                    session_token: sessionToken,
                    user: user
                }),
            });

            if (!response.ok) {
                console.warn('Failed to write session file:', response.status);
            }
        } catch (e) {
            console.warn('Failed to write session file:', e);
        }
    }
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
    'Press Escape to return to login options\n\n' +
    '🔄 Resurrecting BBS culture for the modern age',
    'font-family: monospace; color: #a3a3a3;'
);
