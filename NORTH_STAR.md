# North Star

Fido is the fastest way to connect with a developer.

Fast in both senses: ergonomics (fewest keystrokes from "who wrote this?" to talking with them) and system speed (a TUI should never make you wait). Every feature, fix, and refactor gets measured against that sentence. If it doesn't make connecting with a developer faster, it waits.

## The Five Commandments

### 1. Keystrokes to a conversation is the metric

Count the keypresses from launching Fido to sending a message to a specific developer. That number only goes down. Any new feature that adds a step to that path needs a reason.

### 2. Never make the user wait

Render first, fetch in the background, fill in when data arrives. No spinner where a cached value could be. If an interaction feels slower than vim, it's a bug.

### 3. The repo is the room

Communities are GitHub repositories, not invented spaces. Identity is your GitHub identity. Context (commits, READMEs, roles) comes from the repo. Anything that drifts toward a generic social network gets cut.

### 4. Every screen earns its keybinding

One consistent grammar across the app: if `p` opens a profile anywhere, it opens a profile everywhere. A key that does nothing is a broken promise; wire it or remove it from the footer.

### 5. Ship small, learn from real users

A working rough feature in users' hands beats a polished one on a branch. Stabilize, release, ask what hurt, repeat.
