use html2md::parse_html;

pub fn run(bytes: &[u8]) -> Result<String, String> {
    // Convert bytes to string
    let html_content = String::from_utf8(bytes.to_vec())
        .map_err(|e| format!("Invalid UTF-8 encoding: {}", e))?;
    
    // Parse HTML to Markdown
    let markdown = parse_html(&html_content);
    
    if markdown.trim().is_empty() {
        return Err("Empty or invalid HTML content".to_string());
    }
    
    Ok(markdown)
}
