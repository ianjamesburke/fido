# Fido Web Terminal - User Guide

## Welcome to Fido! 🐕

Fido is a keyboard-driven social network built for developers. This web terminal interface lets you try Fido directly in your browser without installing anything.

## 🚀 Getting Started

### ⚠️ IMPORTANT: Read This First ⚠️

**Fido has two completely different modes with very different behaviors.** Please read the [Complete Test vs Real User Guide](TEST_VS_REAL_USER_GUIDE.md) for detailed information about the critical differences.

### Two Ways to Use Fido

When you visit the web interface, you'll see two options:

#### 1. **Demo Mode (Test User)** 🧪
- **Perfect for:** First-time users, exploring features, testing functionality
- **Data:** Completely temporary and isolated
- **Account:** No account required
- **Duration:** Until you refresh or close the page

#### 2. **GitHub Login (Real Account)** 👤
- **Perfect for:** Joining the community, permanent participation
- **Data:** Saved permanently to your account
- **Account:** Uses your GitHub account for authentication
- **Duration:** Persistent across sessions

---

## 🚨 **CRITICAL: Demo Mode Limitations** 🚨

### ⚠️ What Happens in Demo Mode - READ THIS FIRST ⚠️

**🔄 ALL DATA IS COMPLETELY TEMPORARY**
- **EVERYTHING YOU DO WILL BE LOST** when you:
  - Refresh the page (even accidentally)
  - Close the browser tab
  - Navigate away from the site
  - Experience any network interruption
  - After 30 minutes of inactivity
  - When the server restarts

**🏝️ Completely Isolated Test Environment**
- Your demo actions are **100% separate** from real users
- Demo posts **NEVER appear** in real user feeds
- Real users **CANNOT see** your demo activity
- You **CANNOT interact** with real user content in demo mode
- You are essentially in a "sandbox" with fake data

**⏱️ Zero Persistence**
- **NO data survives between sessions**
- Settings and preferences reset every time
- No account history or profile building possible
- Cannot build followers or reputation

**🚨 Data Reset Triggers**
Demo data is automatically reset when:
- You refresh or reload the page
- You close the browser tab or window
- You navigate to another website
- Your internet connection drops
- The server performs maintenance
- Another user starts a demo session
- 30 minutes of inactivity passes

### Demo Mode Warnings You'll See

- **🚨 Initial Confirmation:** Multiple warnings before entering demo mode
- **🔴 Persistent Banner:** Bright red warning banner while in demo mode  
- **⏰ Timeout Alerts:** Reminders after 15+ minutes of demo usage
- **🔄 Reset Notifications:** Clear confirmation when demo data is cleared
- **📱 Browser Warnings:** Alerts when you try to refresh or leave the page

---

## 🔐 **How to Create a Real Account**

### Step-by-Step Account Creation

**1. Prerequisites**
- You need a GitHub account (free at github.com)
- Make sure you're logged into GitHub in your browser
- Ensure pop-ups are enabled for this site

**2. Starting the Process**
- Click "Login with GitHub" on the main page
- You'll be redirected to GitHub's secure OAuth page
- GitHub will ask for permission to share your basic profile info

**3. What GitHub Shares**
- Your GitHub username (becomes your Fido username)
- Your public profile information
- **NO access to your repositories or private data**
- **NO ability to make changes to your GitHub account**

**4. Account Verification**
- After authorization, you'll be redirected back to Fido
- Your account is created automatically using your GitHub identity
- You'll see a "Login Successful" message

**5. Account Security**
- Your Fido account is linked to your GitHub identity
- No separate passwords to remember
- Secure OAuth authentication every time you log in
- You can revoke access anytime in your GitHub settings

### Troubleshooting Account Creation

**"Authorization Failed" Error:**
- Check that pop-ups are enabled
- Try logging out of GitHub and back in
- Clear your browser cookies and try again

**"User Already Exists" Message:**
- You already have a Fido account with this GitHub username
- Simply log in normally - no need to create a new account

**Stuck on GitHub Page:**
- Make sure you clicked "Authorize" on the GitHub page
- Check if you have multiple GitHub accounts and are logged into the right one
- Try the process in an incognito/private browser window

---

## ✅ **Real Account Benefits**

### What You Get with GitHub Login

**💾 Permanent Data Storage**
- All posts, messages, and votes are saved forever
- Your profile and settings persist across sessions
- Build your reputation and follower network over time

**🌐 Full Community Access**
- See all real user posts and activity
- Interact with the entire Fido community
- Your content appears in real feeds and searches

**🔐 Secure Authentication**
- Uses GitHub OAuth for safe, secure login
- No passwords to remember or manage
- Leverages your existing GitHub identity

**📈 Profile Building**
- Accumulate upvotes and build reputation
- Develop your follower network
- Maintain conversation history

---

## 🎯 **Choosing the Right Mode**

### Start with Demo Mode if you want to:
- ✅ Explore Fido's features without commitment
- ✅ Test keyboard shortcuts and navigation
- ✅ Try posting and messaging without consequences
- ✅ See how the terminal interface works
- ✅ Evaluate if Fido fits your workflow

### Switch to GitHub Login when you're ready to:
- ✅ Join the community permanently
- ✅ Build your developer network
- ✅ Contribute meaningful content
- ✅ Engage in ongoing conversations
- ✅ Establish your presence on Fido

---

## 🎮 **Using the Terminal Interface**

### Essential Keyboard Shortcuts

| Key | Action |
|-----|--------|
| `Tab` | Switch between tabs (Posts, Messages, Profile) |
| `j` / `k` | Navigate up/down through posts |
| `↑` / `↓` | Navigate up/down through posts |
| `u` / `d` | Upvote/Downvote posts |
| `n` | Create new post |
| `r` | Reply to post |
| `m` | Send direct message |
| `?` | Show help and all shortcuts |
| `q` | Quit/Logout |
| `Ctrl+K` | Focus terminal (if it loses focus) |
| `Escape` | Return to login options |

### Navigation Tips

1. **Focus Management:** Click inside the terminal area to ensure keyboard input is captured
2. **Tab Navigation:** Use `Tab` to move between different sections (Posts, DMs, Profile)
3. **Scrolling:** Use `j`/`k` or arrow keys to scroll through content
4. **Help:** Press `?` anytime to see all available shortcuts

---

## 🔄 **Switching Between Modes**

### From Demo to Real Account
1. Press `Escape` to return to login options
2. Your demo data will be automatically cleared
3. Click "Login with GitHub" to create/access your real account
4. Authenticate through GitHub OAuth
5. Start fresh with your permanent account

### From Real Account to Demo
1. Press `Escape` to return to login options  
2. Your real account data remains safe and unchanged
3. Click "Try Demo" to enter isolated test mode
4. Confirm the demo mode warnings
5. Explore with temporary data

---

## 🛠️ **Troubleshooting**

### Terminal Not Responding to Keyboard
- **Solution:** Click inside the terminal area to focus it
- **Alternative:** Press `Ctrl+K` to force focus
- **Check:** Ensure you're not in a browser input field

### Can't See My Posts (Demo Mode)
- **Expected:** Demo posts are isolated and won't appear in real feeds
- **Solution:** Switch to GitHub login to see real community content

### Data Disappeared (Demo Mode)
- **Expected:** Demo data resets automatically
- **Prevention:** Use GitHub login for persistent data
- **Recovery:** Demo data cannot be recovered once reset

### Authentication Issues
- **GitHub Login:** Ensure pop-ups are enabled for OAuth
- **Session Expired:** Press `Escape` and login again
- **Browser Issues:** Try clearing cookies and cache

### Performance Issues
- **Slow Response:** Check your internet connection
- **Keyboard Lag:** Try refreshing the page
- **Display Problems:** Ensure your browser supports modern web standards

---

## 🎓 **Best Practices**

### For Demo Mode Users
1. **Experiment freely** - nothing you do has consequences
2. **Try all features** - post, vote, message, explore settings
3. **Test keyboard shortcuts** - learn the interface efficiently
4. **Don't invest time** in lengthy posts (they'll be lost)
5. **Switch to real account** when ready to participate

### For Real Account Users
1. **Engage meaningfully** - your posts and votes matter
2. **Build relationships** - follow interesting developers
3. **Contribute quality content** - help build the community
4. **Use keyboard shortcuts** - maximize your efficiency
5. **Respect others** - maintain professional developer discourse

---

## 🚀 **Ready to Get Started?**

### New Users: Start Here
1. Visit the Fido web interface
2. Click "Try Demo (Test User)" 
3. Confirm the demo mode warnings
4. Explore features using keyboard shortcuts
5. When ready, switch to "Login with GitHub"

### Returning Users
1. Click "Login with GitHub"
2. Authenticate through OAuth
3. Access your existing account and data
4. Continue where you left off

---

## 💡 **Pro Tips**

- **Install Native App:** For the best experience, install with `cargo install fido`
- **Keyboard First:** Learn shortcuts for maximum efficiency
- **Community Guidelines:** Be respectful and contribute meaningfully
- **Feature Requests:** Share feedback to help improve Fido
- **Stay Updated:** Follow Fido development on GitHub

---

## 🆘 **Need Help?**

- **In-App Help:** Press `?` while using Fido
- **GitHub Issues:** Report bugs or request features
- **Community:** Ask questions in Fido posts
- **Documentation:** Check the main README for more details

---

**Happy coding and socializing! 🎉**

*Remember: Demo mode is for exploration, GitHub login is for participation.*