use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
    Frame,
};
use uuid::Uuid;
use fido_types::Post;

use crate::app::App;
use super::super::theme::get_theme_colors;
use super::super::formatting::*;
use super::utils::centered_rect;

/// Render delete confirmation modal (matches unsaved changes modal style)
pub fn render_delete_confirmation_modal(frame: &mut Frame, app: &mut App, area: Rect) {
    let theme = get_theme_colors(app);

    // Get post detail state
    let detail_state = match &app.post_detail_state {
        Some(state) => state,
        None => return,
    };

    // Get post info
    let _post = match &detail_state.post {
        Some(p) => p,
        None => return,
    };

    // Create centered modal area (50% width, 35% height) - sized to show all content including shortcuts
    let modal_area = centered_rect(50, 35, area);

    // Clear background
    frame.render_widget(Clear, modal_area);

    // Get post being deleted info
    let post_being_deleted = detail_state.get_deletable_post();
    
    // Determine if we're deleting a reply or the main post
    let is_deleting_reply = post_being_deleted
        .map(|p| p.parent_post_id.is_some())
        .unwrap_or(false);
    
    let delete_message = if is_deleting_reply {
        "Are you sure you want to delete this reply?"
    } else {
        "Are you sure you want to delete this post?"
    };
    
    // Build content with message and styled keyboard shortcuts
    let mut content = vec![
        Line::from(""),
        Line::from(Span::styled(
            delete_message,
            Style::default().fg(theme.text),
        )),
        Line::from(""),
    ];

    // Count DIRECT replies to the post being deleted (not all descendants)
    if let Some(post) = post_being_deleted {
        let direct_reply_count = detail_state.replies.iter()
            .filter(|r| r.parent_post_id == Some(post.id))
            .count();
        
        if direct_reply_count > 0 {
            content.push(Line::from(Span::styled(
                format!(
                    "⚠ Warning: {} direct {}!",
                    direct_reply_count,
                    if direct_reply_count == 1 { "reply" } else { "replies" }
                ),
                Style::default()
                    .fg(theme.warning)
                    .add_modifier(Modifier::BOLD),
            )));
            content.push(Line::from(""));
        }
    }

    content.push(Line::from(Span::styled(
        "This action cannot be undone.",
        Style::default().fg(theme.text_dim),
    )));
    content.push(Line::from(""));
    content.push(Line::from("─".repeat(46)).style(Style::default().fg(theme.border)));
    content.push(Line::from(""));
    
    // Styled keyboard shortcuts matching unsaved changes modal
    content.push(Line::from(vec![
        Span::styled("Y", Style::default().fg(theme.error).add_modifier(Modifier::BOLD)),
        Span::styled(": Delete  ", Style::default().fg(theme.text)),
        Span::styled("N", Style::default().fg(theme.success).add_modifier(Modifier::BOLD)),
        Span::styled(": Cancel  ", Style::default().fg(theme.text)),
        Span::styled("Esc", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
        Span::styled(": Cancel", Style::default().fg(theme.text)),
    ]));

    let modal = Paragraph::new(content)
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .title(" Delete Post ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.warning).add_modifier(Modifier::BOLD))
                .style(Style::default().bg(theme.background)),
        );

    frame.render_widget(modal, modal_area);
}

/// Render full post modal (for viewing complete nested reply content with thread tree)
pub fn render_full_post_modal(frame: &mut Frame, app: &mut App, area: Rect) {
    let theme = get_theme_colors(app);

    // Get post detail state
    let detail_state = match &mut app.post_detail_state {
        Some(state) => state,
        None => return,
    };

    // Create centered modal area (90% width, 80% height to avoid overlapping header)
    let modal_area = centered_rect(90, 80, area);

    // Clear background
    frame.render_widget(Clear, modal_area);

    // Show loading spinner if loading
    if detail_state.loading {
        let loading_block = Block::default()
            .title(" Loading Thread... ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.accent).add_modifier(Modifier::BOLD))
            .style(Style::default().bg(theme.background));
        
        let inner = loading_block.inner(modal_area);
        frame.render_widget(loading_block, modal_area);
        
        let loading_text = Paragraph::new("⏳ Loading thread data...")
            .style(Style::default().fg(theme.text))
            .alignment(Alignment::Center);
        frame.render_widget(loading_text, inner);
        return;
    }

    // Find the root post to display in modal
    let modal_root_post = if let Some(modal_post_id) = detail_state.full_post_modal_id {
        // Find in replies or main post
        if let Some(post) = &detail_state.post {
            if post.id == modal_post_id {
                Some(post.clone())
            } else {
                detail_state.replies.iter().find(|r| r.id == modal_post_id).cloned()
            }
        } else {
            None
        }
    } else {
        None
    };

    let root_post = match modal_root_post {
        Some(p) => p,
        None => {
            // Show error if no post found
            let error_block = Block::default()
                .title(" Error ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.error).add_modifier(Modifier::BOLD))
                .style(Style::default().bg(theme.background));
            
            let inner = error_block.inner(modal_area);
            frame.render_widget(error_block, modal_area);
            
            let error_text = Paragraph::new("❌ Thread not found")
                .style(Style::default().fg(theme.error))
                .alignment(Alignment::Center);
            frame.render_widget(error_text, inner);
            return;
        }
    };

    // Filter replies to only show descendants of the selected post (excluding the root itself)
    let modal_replies: Vec<Post> = detail_state.replies.iter()
        .filter(|reply| {
            // Exclude the root post itself and only include its descendants
            reply.id != root_post.id && is_descendant_of(reply, &root_post.id, &detail_state.replies)
        })
        .cloned()
        .collect();

    // modal_area and Clear already rendered above
    let title_text = format!(" Thread by @{} ({} replies) ", root_post.author_username, modal_replies.len());
    
    let block = Block::default()
        .title(title_text)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.accent).add_modifier(Modifier::BOLD))
        .style(Style::default().bg(theme.background));

    let inner = block.inner(modal_area);
    frame.render_widget(block, modal_area);

    // Create modal layout
    let modal_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(0),    // Content (root post + nested replies)
            Constraint::Length(3), // Footer (needs 3 for border + text)
        ])
        .split(inner);

    // Content: Root post + nested replies tree
    let content_width = (modal_chunks[1].width as usize).saturating_sub(4);
    
    if modal_replies.is_empty() {
        // Just show the root post with no replies
        let mut content_lines = vec![];
        
        // Root post header
        content_lines.push(Line::from(vec![
            Span::styled(
                format!("@{}", root_post.author_username),
                Style::default().fg(theme.primary).add_modifier(Modifier::BOLD),
            ),
            Span::raw(" • "),
            Span::styled(
                format_timestamp(&root_post.created_at),
                Style::default().fg(theme.text_dim),
            ),
        ]));
        content_lines.push(Line::from(""));
        
        // Full post content
        let post_content_lines = format_post_content_with_width(&root_post.content, false, &theme, content_width);
        content_lines.extend(post_content_lines);
        content_lines.push(Line::from(""));
        
        // Vote counts
        let user_voted_up = root_post.user_vote.as_deref() == Some("up");
        let user_voted_down = root_post.user_vote.as_deref() == Some("down");

        let upvote_style = if user_voted_up {
            Style::default().fg(theme.success).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.text_dim)
        };

        let downvote_style = if user_voted_down {
            Style::default().fg(theme.error).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.text_dim)
        };

        content_lines.push(Line::from(vec![
            Span::styled(format!("↑ {}", root_post.upvotes), upvote_style),
            Span::raw("  "),
            Span::styled(format!("↓ {}", root_post.downvotes), downvote_style),
            Span::raw("  "),
            Span::styled(
                format!("💬 {}", root_post.reply_count),
                Style::default().fg(theme.text_dim),
            ),
        ]));
        
        // Show delete option if user owns the post
        if let Some(current_user) = &app.auth_state.current_user {
            if current_user.id == root_post.author_id {
                content_lines.push(Line::from(vec![
                    Span::styled(
                        "Your post - ",
                        Style::default().fg(theme.text_dim).add_modifier(Modifier::ITALIC),
                    ),
                    Span::styled(
                        "x: Delete",
                        Style::default().fg(theme.error),
                    ),
                ]));
            }
        }
        
        content_lines.push(Line::from(""));
        
        // Add horizontal separator
        content_lines.push(Line::from(Span::styled(
            "─".repeat(content_width),
            Style::default().fg(theme.text_dim),
        )));
        content_lines.push(Line::from(""));
        
        content_lines.push(Line::from(Span::styled(
            "No replies yet",
            Style::default().fg(theme.text_dim).add_modifier(Modifier::ITALIC),
        )));

        let content = Paragraph::new(content_lines)
            .wrap(ratatui::widgets::Wrap { trim: false });
        frame.render_widget(content, modal_chunks[0]);
    } else {
        // Build tree and render with replies
        let reply_tree = build_reply_tree_from_root(&root_post, &modal_replies);
        
        // IMPORTANT: Only show children if root is expanded
        // Check if root post is expanded to show its direct children
        let root_is_expanded = detail_state.modal_expanded_posts.get(&root_post.id).copied().unwrap_or(false);
        let flattened = if root_is_expanded {
            flatten_tree(&reply_tree, &detail_state.modal_expanded_posts)
        } else {
            vec![] // Don't show any children if root is collapsed
        };
        
        // Build list items for root post + replies
        let mut all_items = vec![];
        
        // First item: Root post (always shown, index 0)
        let root_is_selected = detail_state.modal_list_state.selected() == Some(0);
        let mut root_lines = vec![];
        
        let root_prefix = if root_is_selected { "▶ " } else { "  " };
        let root_style = if root_is_selected {
            Style::default().fg(theme.success).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.primary)
        };
        
        // Show expand/collapse indicator for root if it has children
        let has_replies = !modal_replies.is_empty();
        let expansion_indicator = if has_replies {
            if root_is_expanded { "[-] " } else { "[+] " }
        } else {
            ""
        };
        
        root_lines.push(Line::from(vec![
            Span::styled(root_prefix, root_style),
            Span::styled(expansion_indicator, root_style),
            Span::styled(format!("@{}", root_post.author_username), root_style),
            Span::raw(" • "),
            Span::styled(
                format_timestamp(&root_post.created_at),
                Style::default().fg(theme.text_dim),
            ),
        ]));
        
        let root_content_lines = format_post_content_with_width(&root_post.content, root_is_selected, &theme, content_width);
        for line in root_content_lines {
            let mut spans = vec![Span::raw("  ")];
            spans.extend(line.spans);
            root_lines.push(Line::from(spans));
        }
        
        // Vote counts for root
        let user_voted_up = root_post.user_vote.as_deref() == Some("up");
        let user_voted_down = root_post.user_vote.as_deref() == Some("down");
        
        root_lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(
                format!("↑ {}", root_post.upvotes),
                if user_voted_up { Style::default().fg(theme.success).add_modifier(Modifier::BOLD) } else { Style::default().fg(theme.text_dim) }
            ),
            Span::raw("  "),
            Span::styled(
                format!("↓ {}", root_post.downvotes),
                if user_voted_down { Style::default().fg(theme.error).add_modifier(Modifier::BOLD) } else { Style::default().fg(theme.text_dim) }
            ),
            Span::raw("  "),
            Span::styled(
                format!("💬 {}", root_post.reply_count),
                Style::default().fg(theme.text_dim),
            ),
        ]));
        
        // Show edit/delete options if user owns the post
        if let Some(current_user) = &app.auth_state.current_user {
            if current_user.id == root_post.author_id {
                root_lines.push(Line::from(vec![
                    Span::raw("  "),
                    Span::styled(
                        "Your post - ",
                        Style::default().fg(theme.text_dim).add_modifier(Modifier::ITALIC),
                    ),
                    Span::styled(
                        "x: Delete",
                        Style::default().fg(theme.error),
                    ),
                ]));
            }
        }
        
        root_lines.push(Line::from(""));
        
        // Add horizontal separator between post and replies
        root_lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(
                "─".repeat(content_width.saturating_sub(2)),
                Style::default().fg(theme.text_dim),
            ),
        ]));
        root_lines.push(Line::from(""));
        
        all_items.push(ListItem::new(root_lines));
        
        // Add reply items (indices 1+)
        for (flat_idx, (node, has_children)) in flattened.iter().enumerate() {
            let item_index = flat_idx + 1; // +1 because root is index 0
            let is_selected = detail_state.modal_list_state.selected() == Some(item_index);
            let mut reply_lines = vec![];
            
            let reply = &node.post;
            let depth = node.depth;
            let visual_depth = depth.min(5);
            let indent = "  ".repeat(visual_depth);
            
            let tree_char = if depth > 0 { "├─ " } else { "" };
            let is_expanded = detail_state.modal_expanded_posts.get(&reply.id).copied().unwrap_or(false);
            let expansion_indicator = if *has_children {
                if is_expanded { "[-] " } else { "[+] " }
            } else {
                "    "
            };
            
            let prefix = if is_selected { "▶ " } else { "  " };
            let header_style = if is_selected {
                Style::default().fg(theme.success).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.primary)
            };
            
            reply_lines.push(Line::from(vec![
                Span::styled(prefix, header_style),
                Span::styled(indent.clone(), Style::default().fg(theme.text_dim)),
                Span::styled(tree_char, Style::default().fg(theme.text_dim)),
                Span::styled(expansion_indicator, Style::default().fg(theme.accent)),
                Span::styled(format!("@{}", reply.author_username), header_style),
                Span::raw(" • "),
                Span::styled(
                    format_timestamp(&reply.created_at),
                    Style::default().fg(theme.text_dim),
                ),
            ]));
            
            // Reply content
            let reply_content_lines = format_post_content_with_width(
                &reply.content,
                is_selected,
                &theme,
                content_width.saturating_sub(2 + visual_depth * 2),
            );
            for line in reply_content_lines {
                let mut spans = vec![
                    Span::raw("  "),
                    Span::raw(indent.clone()),
                    Span::raw("   "),
                ];
                spans.extend(line.spans);
                reply_lines.push(Line::from(spans));
            }
            
            // Vote counts
            let reply_voted_up = reply.user_vote.as_deref() == Some("up");
            let reply_voted_down = reply.user_vote.as_deref() == Some("down");
            
            reply_lines.push(Line::from(vec![
                Span::raw("  "),
                Span::raw(indent.clone()),
                Span::raw("   "),
                Span::styled(
                    format!("↑ {}", reply.upvotes),
                    if reply_voted_up { Style::default().fg(theme.success).add_modifier(Modifier::BOLD) } else { Style::default().fg(theme.text_dim) }
                ),
                Span::raw("  "),
                Span::styled(
                    format!("↓ {}", reply.downvotes),
                    if reply_voted_down { Style::default().fg(theme.error).add_modifier(Modifier::BOLD) } else { Style::default().fg(theme.text_dim) }
                ),
                Span::raw("  "),
                Span::styled(
                    format!("💬 {}", reply.reply_count),
                    Style::default().fg(theme.text_dim),
                ),
            ]));
            reply_lines.push(Line::from(""));
            
            all_items.push(ListItem::new(reply_lines));
        }
        
        // Render as scrollable list
        let replies_list = List::new(all_items)
            .highlight_style(Style::default().bg(theme.highlight_bg));
        
        frame.render_stateful_widget(replies_list, modal_chunks[0], &mut detail_state.modal_list_state);
    }

    // Footer with keyboard shortcuts (context-sensitive and detailed)
    let footer_text = "↑/↓/j/k: Navigate | Space: Expand/Collapse | u/d: Vote | r: Reply | x: Delete | p: View Profile | Esc: Close";
    let footer = Paragraph::new(footer_text)
        .style(Style::default().fg(theme.text))
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.border)),
        );
    frame.render_widget(footer, modal_chunks[1]);
}

/// Check if a reply is a descendant of a given post
fn is_descendant_of(reply: &Post, ancestor_id: &Uuid, all_replies: &[Post]) -> bool {
    let mut current_parent = reply.parent_post_id;
    
    while let Some(parent_id) = current_parent {
        if parent_id == *ancestor_id {
            return true;
        }
        // Find the parent in all_replies and continue up the chain
        current_parent = all_replies.iter()
            .find(|r| r.id == parent_id)
            .and_then(|r| r.parent_post_id);
    }
    
    false
}

/// Reply tree node for nested rendering
#[derive(Clone)]
struct ReplyNode {
    post: Post,
    depth: usize,
    children: Vec<ReplyNode>,
    flat_index: usize,
}

/// Flatten tree for rendering with expansion state
fn flatten_tree(nodes: &[ReplyNode], expanded_posts: &std::collections::HashMap<Uuid, bool>) -> Vec<(ReplyNode, bool)> {
    let mut result = Vec::new();
    
    fn flatten_recursive(
        node: &ReplyNode,
        expanded_posts: &std::collections::HashMap<Uuid, bool>,
        result: &mut Vec<(ReplyNode, bool)>,
    ) {
        let is_expanded = expanded_posts.get(&node.post.id).copied().unwrap_or(false);
        let has_children = !node.children.is_empty();
        
        result.push((node.clone(), has_children));
        
        if is_expanded {
            for child in &node.children {
                flatten_recursive(child, expanded_posts, result);
            }
        }
    }
    
    for node in nodes {
        flatten_recursive(node, expanded_posts, &mut result);
    }
    
    result
}

/// Build reply tree starting from a specific root post
fn build_reply_tree_from_root(root: &Post, replies: &[Post]) -> Vec<ReplyNode> {
    use std::collections::HashMap;
    
    // Group replies by parent
    let mut children_map: HashMap<Uuid, Vec<&Post>> = HashMap::new();
    for reply in replies {
        if let Some(parent_id) = reply.parent_post_id {
            children_map.entry(parent_id).or_default().push(reply);
        }
    }
    
    // Build tree recursively
    fn build_node(post: &Post, depth: usize, children_map: &HashMap<Uuid, Vec<&Post>>, flat_index: &mut usize) -> ReplyNode {
        let current_index = *flat_index;
        *flat_index += 1;
        
        let children = children_map
            .get(&post.id)
            .map(|kids| {
                kids.iter()
                    .map(|child| build_node(child, depth + 1, children_map, flat_index))
                    .collect()
            })
            .unwrap_or_default();
        
        ReplyNode {
            post: post.clone(),
            depth,
            children,
            flat_index: current_index,
        }
    }
    
    // Get direct children of root
    let mut flat_index = 0;
    children_map
        .get(&root.id)
        .map(|kids| {
            kids.iter()
                .map(|child| build_node(child, 0, &children_map, &mut flat_index))
                .collect()
        })
        .unwrap_or_default()
}
