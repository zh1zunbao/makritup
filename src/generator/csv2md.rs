use csv::ReaderBuilder;
use std::io::Cursor;

pub fn run(bytes: &[u8]) -> Result<String, String> {
    let cursor = Cursor::new(bytes);
    let mut rdr = ReaderBuilder::new()
        .has_headers(true)
        .from_reader(cursor);
    
    let mut markdown = String::new();
    
    // Extract headers before iterating over records
    if let Ok(headers) = rdr.headers() {
        let header_row = headers
            .iter()
            .map(|h| h.trim())
            .collect::<Vec<&str>>()
            .join(" | ");
        markdown.push_str("| ");
        markdown.push_str(&header_row);
        markdown.push_str(" |\n");
        
        // Add separator row
        let separator = headers
            .iter()
            .map(|_| "---")
            .collect::<Vec<&str>>()
            .join(" | ");
        markdown.push_str("| ");
        markdown.push_str(&separator);
        markdown.push_str(" |\n");
    }
    
    for result in rdr.records() {
        match result {
            Ok(record) => {
                
                // Write data row
                let row = record
                    .iter()
                    .map(|cell| cell.trim())
                    .collect::<Vec<&str>>()
                    .join(" | ");
                markdown.push_str("| ");
                markdown.push_str(&row);
                markdown.push_str(" |\n");
            }
            Err(err) => {
                return Err(format!("CSV parsing error: {}", err));
            }
        }
    }
    
    if markdown.is_empty() {
        return Err("Empty or invalid CSV data".to_string());
    }
    
    Ok(markdown)
}
