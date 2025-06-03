use std::io::{Cursor, Read};
use std::collections::HashMap;
use zip::ZipArchive;
use docx_rust::{
    document::{BodyContent, TableCellContent, TableRowContent, ParagraphContent},
    DocxFile,
};
use crate::converter::image2md;

pub fn run(file_stream: &[u8]) -> Result<String, String> {
    run_with_images(file_stream)
}

fn run_with_images(file_stream: &[u8]) -> Result<String, String> {
    let cursor = Cursor::new(file_stream);
    let mut archive = ZipArchive::new(cursor)
        .map_err(|e| format!("Failed to open DOCX archive: {}", e))?;

    // First, extract all images from the archive
    let mut images = HashMap::new();
    for i in 0..archive.len() {
        let mut file = archive
            .by_index(i)
            .map_err(|e| format!("Failed to access file in ZIP archive: {}", e))?;
        
        if file.name().starts_with("word/media/") {
            let mut image_data = Vec::new();
            file.read_to_end(&mut image_data)
                .map_err(|e| format!("Failed to read image data: {}", e))?;
            
            let filename = file.name().to_string();
            images.insert(filename, image_data);
        }
    }

    // Reset cursor and parse DOCX with docx_rust
    let cursor = Cursor::new(file_stream);
    let docx_file = DocxFile::from_reader(cursor)
        .map_err(|e| format!("Failed to read DOCX file: {}", e))?;
    
    let doc = docx_file.parse()
        .map_err(|e| format!("Failed to parse DOCX file: {}", e))?;

    let mut markdown = String::new();
    markdown.push_str("# Document\n\n");

    for content in doc.document.body.content {
        match content {
            BodyContent::Paragraph(paragraph) => {
                let paragraph_md = process_paragraph(&paragraph, &images)?;
                if !paragraph_md.trim().is_empty() {
                    markdown.push_str(&paragraph_md);
                    markdown.push_str("\n\n");
                }
            }
            BodyContent::Table(table) => {
                let table_md = process_table(&table)?;
                if !table_md.trim().is_empty() {
                    markdown.push_str(&table_md);
                    markdown.push_str("\n\n");
                }
            }
            _ => {}
        }
    }

    Ok(markdown)
}

fn process_paragraph(
    paragraph: &docx_rust::document::Paragraph,
    images: &HashMap<String, Vec<u8>>
) -> Result<String, String> {
    let mut text_content = String::new();
    let mut is_heading = false;
    let mut heading_level = 1;

    // Check paragraph style for heading detection
    if let Some(property) = &paragraph.property {
        if let Some(style_id) = &property.style_id {
            if let Some((is_h, level)) = check_style_for_heading(&style_id.value) {
                is_heading = is_h;
                heading_level = level;
            }
        }
    }

    // Extract text content and check for formatting-based headings
    let mut has_bold = false;
    let mut font_size: Option<f32> = None;

    for content in &paragraph.content {
        match content {
            ParagraphContent::Run(run) => {
                // Check run properties for formatting
                if let Some(props) = &run.property {
                    if props.bold.is_some() {
                        has_bold = true;
                    }
                    if let Some(size) = &props.size {
                        font_size = Some(size.value as f32 / 2.0); // Convert half-points to points
                    }
                }

                // Extract text from run
                for run_content in &run.content {
                    match run_content {
                        docx_rust::document::RunContent::Text(text) => {
                            text_content.push_str(&text.text);
                        }
                        docx_rust::document::RunContent::Drawing(_drawing) => {
                            // Process embedded images in drawings
                            if let Some(image_md) = process_drawing_images(images)? {
                                text_content.push_str(&image_md);
                            }
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }

    // Determine final heading status
    let (final_is_heading, final_level) = determine_heading_status(
        is_heading,
        heading_level,
        has_bold,
        font_size,
        &text_content
    );

    if final_is_heading && !text_content.trim().is_empty() {
        let heading_prefix = "#".repeat(final_level.min(6));
        Ok(format!("{} {}", heading_prefix, text_content.trim()))
    } else {
        Ok(text_content)
    }
}

fn process_drawing_images(images: &HashMap<String, Vec<u8>>) -> Result<Option<String>, String> {
    // Process the first available image (simplified approach)
    // In a more sophisticated implementation, we would parse the drawing XML
    // to find the specific image reference
    for (filename, image_data) in images {
        if filename.ends_with(".png") || 
           filename.ends_with(".jpg") || 
           filename.ends_with(".jpeg") ||
           filename.ends_with(".gif") ||
           filename.ends_with(".webp") {
            
            let image_md = image2md::run(image_data)?;
            return Ok(Some(format!("\n\n{}\n\n", image_md)));
        }
    }
    Ok(None)
}

fn check_style_for_heading(style_name: &str) -> Option<(bool, usize)> {
    let style_lower = style_name.to_lowercase();
    
    // Check for various heading patterns
    if style_lower.starts_with("heading") || style_lower.starts_with("title") {
        let level = style_name.chars()
            .filter(|c| c.is_ascii_digit())
            .collect::<String>()
            .parse::<usize>()
            .unwrap_or(1);
        return Some((true, level));
    }
    
    // Check for title styles
    if style_lower == "title" {
        return Some((true, 1));
    }
    
    // Check for subtitle styles
    if style_lower.contains("subtitle") {
        return Some((true, 2));
    }
    
    // Check for other common heading patterns
    if style_lower.contains("header") {
        let level = style_name.chars()
            .filter(|c| c.is_ascii_digit())
            .collect::<String>()
            .parse::<usize>()
            .unwrap_or(3);
        return Some((true, level));
    }

    None
}

fn determine_heading_status(
    style_is_heading: bool,
    style_level: usize,
    has_bold: bool,
    font_size: Option<f32>,
    content: &str
) -> (bool, usize) {
    // If explicitly marked as heading by style, use that
    if style_is_heading {
        return (true, style_level);
    }
    
    // Check font size for heading detection
    if let Some(size) = font_size {
        let level = match size as u32 {
            s if s >= 18 => 1, // 18pt+ = H1
            s if s >= 16 => 2, // 16pt+ = H2
            s if s >= 14 => 3, // 14pt+ = H3
            s if s >= 13 => 4, // 13pt+ = H4
            s if s >= 12 => 5, // 12pt+ = H5
            _ => return (false, 1), // Normal text
        };
        
        // Additional check: short lines are more likely to be headings
        if content.trim().len() < 100 && !content.trim().ends_with('.') {
            return (true, level);
        }
    }
    
    // Heuristic: short, bold lines without periods might be headings
    let trimmed = content.trim();
    if has_bold && 
       trimmed.len() > 0 && 
       trimmed.len() < 80 && 
       !trimmed.ends_with('.') && 
       !trimmed.ends_with('!') && 
       !trimmed.ends_with('?') &&
       !trimmed.contains('\n') &&
       trimmed.chars().any(|c| c.is_alphabetic()) {
        
        // Guess level based on length
        if trimmed.len() < 30 {
            return (true, 2); // Short titles are likely H2
        } else if trimmed.len() < 50 {
            return (true, 3); // Medium titles are likely H3
        } else {
            return (true, 4); // Longer titles are likely H4
        }
    }
    
    (false, 1)
}

fn process_table(table: &docx_rust::document::Table) -> Result<String, String> {
    if table.rows.is_empty() {
        return Ok(String::new());
    }

    let mut markdown = String::new();

    // Header row
    markdown.push_str("|");
    if let Some(first_row) = table.rows.first() {
        for cell in &first_row.cells {
            match cell {
                TableRowContent::TableCell(tc) => {
                    let cell_text = extract_cell_text(tc);
                    markdown.push_str(&format!(" {} |", cell_text));
                }
                _ => {
                    markdown.push_str(" |");
                }
            }
        }
        markdown.push_str("\n");

        // Separator row
        markdown.push_str("|");
        for _ in &first_row.cells {
            markdown.push_str("---|");
        }
        markdown.push_str("\n");
    }

    // Data rows
    for row in table.rows.iter().skip(1) {
        markdown.push_str("|");
        for cell in &row.cells {
            match cell {
                TableRowContent::TableCell(tc) => {
                    let cell_text = extract_cell_text(tc);
                    markdown.push_str(&format!(" {} |", cell_text));
                }
                _ => {
                    markdown.push_str(" |");
                }
            }
        }
        markdown.push_str("\n");
    }

    Ok(markdown)
}

fn extract_cell_text(cell: &docx_rust::document::TableCell) -> String {
    let mut text = String::new();
    
    for content in &cell.content {
        match content {
            TableCellContent::Paragraph(paragraph) => {
                for para_content in &paragraph.content {
                    if let ParagraphContent::Run(run) = para_content {
                        for run_content in &run.content {
                            if let docx_rust::document::RunContent::Text(text_elem) = run_content {
                                text.push_str(&text_elem.text);
                            }
                        }
                    }
                }
                if !text.is_empty() && !text.ends_with(' ') {
                    text.push(' ');
                }
            }
        }
    }
    
    text.trim().to_string()
}
