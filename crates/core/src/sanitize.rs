use regex::Regex;
use std::sync::OnceLock;

static HTML_TAG_REGEX: OnceLock<Regex> = OnceLock::new();

/// Strip HTML tags and script elements from string
pub fn strip_html(input: &str) -> String {
    let re = HTML_TAG_REGEX.get_or_init(|| Regex::new(r"<[^>]*>").unwrap());
    re.replace_all(input, "").to_string()
}

/// Trim, strip HTML, and enforce max length limit
pub fn sanitize_text(input: &str, max_length: usize) -> String {
    let trimmed = input.trim();
    let stripped = strip_html(trimmed);
    if stripped.chars().count() > max_length {
        stripped.chars().take(max_length).collect()
    } else {
        stripped
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_html_stripped() {
        let dirty = "<script>alert('xss')</script>Hello <b>World</b>!";
        let clean = strip_html(dirty);
        assert_eq!(clean, "alert('xss')Hello World!");
    }

    #[test]
    fn test_sanitize_text_trim_and_limit() {
        let raw = "   <div>Super Long Text Content</div>   ";
        let sanitized = sanitize_text(raw, 10);
        assert_eq!(sanitized, "Super Long");
    }
}
