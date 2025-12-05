# Fido: Developer-Centric Terminal Social Platform (MVP)

**Summary:**  
Fido is a blazing-fast, keyboard-driven social platform for developers, featuring a beautiful terminal interface—no algorithmic feeds, no ads, just control and efficiency.

---

## Core Idea

Fido offers developers a private, ad-free, highly-configurable social media experience in their terminal. Users post, upvote, and converse using instant keyboard shortcuts. All content is text/Markdown—optimized for developer workflows.

## Problem & Goal

- *Problem*: Current social networks force algorithmic feeds, are slow, ad-supported, and lack true configuration.
- *Goal*:  Maximum speed and control for developer–to–developer social interaction, via an ultra-efficient, fully keyboard-controlled TUI.

## Core MVP Features

- **GitHub Login:** Secure, instant onboarding with local credential storage.
- **Global Message Board:** One live feed for all users.
- **Markdown Support:** Posts/articles with full Markdown formatting.
- **Keyboard Navigation Everywhere:** Shortcut for every action—no mouse needed.
- **Sorting & Color Scheme Config:** Users control feed order and app appearance.
- **Mutual DMs:** Instant private messages via dashboard or CLI (`fido dm @user "msg"`).
- **Upvote/Downvote:** Posts can be upvoted or downvoted directly, influencing sort order.
- **No Ads. No Images. No Videos.** Text and community only.

## Uniqueness

- No algorithms, no ads, no distraction  
- Lightning-fast, terminal-native UI for devs  
- Configurable, privacy-first design—your network, your way

## High-Level Architecture

- **Frontend:** Ratatui and Rust–native TUI (plus command-line)
- **Backend:** Rust/Axum REST API, local SQLite database, lightweight approach
- **Testing:** Mock data and test suites for rapid iteration

---

*For more technical or sequential details, see the accompanying engineering spec sheet.*

