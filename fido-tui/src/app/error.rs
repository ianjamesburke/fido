use super::get_modifier_key_name;

/// Categorize error messages for better user feedback
pub fn categorize_error(error_str: &str) -> String {
    let error_lower = error_str.to_lowercase();

    // Rate limit errors - show the server's message directly (it includes wait time)
    if error_lower.contains("rate limit")
        || error_lower.contains("too many requests")
        || error_lower.contains("429")
    {
        // Extract the useful part of the message if it's wrapped in JSON
        if let Some(start) = error_str.find("Please wait") {
            if let Some(end) = error_str[start..].find('.') {
                return format!("⏳ {}", &error_str[start..start + end + 1]);
            }
        }
        // If the message already contains seconds info, use it directly
        if error_lower.contains("second") || error_lower.contains("wait") {
            // Clean up the message - remove JSON wrapper if present
            let clean_msg = error_str
                .replace("Rate limit exceeded: ", "")
                .replace("{\"error\":\"Too Many Requests\",\"details\":\"", "")
                .replace("\"}", "");
            return format!("⏳ {}", clean_msg);
        }
        return "⏳ Rate limit exceeded. Please wait a moment before trying again.".to_string();
    }

    // Network errors
    if error_lower.contains("connection")
        || error_lower.contains("timeout")
        || error_lower.contains("network")
    {
        let modifier = get_modifier_key_name();
        return format!("Network Error: Connection failed. Check your network and try again (Press {}+R to retry)", modifier);
    }

    // Authorization errors
    if error_lower.contains("401")
        || error_lower.contains("403")
        || error_lower.contains("unauthorized")
        || error_lower.contains("forbidden")
    {
        return "Authorization Error: Session expired or insufficient permissions. Please log in again (Press Shift+L to logout)".to_string();
    }

    // Validation errors
    if error_lower.contains("400")
        || error_lower.contains("validation")
        || error_lower.contains("invalid")
    {
        return format!("Validation Error: {}", error_str);
    }

    // Server errors
    if error_lower.contains("500") || error_lower.contains("502") || error_lower.contains("503") {
        let modifier = get_modifier_key_name();
        return format!("Server Error: The server is experiencing issues. Please try again later (Press {}+R to retry)", modifier);
    }

    // Generic error with retry instruction
    format!("Error: {}", error_str)
}
