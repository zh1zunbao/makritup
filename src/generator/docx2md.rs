use std::io::{Cursor, Read};
use std::collections::HashMap;
use std::process::Command;
use std::path::Path;
use zip::ZipArchive;
use docx_rust::{
    document::{BodyContent, TableCellContent, TableRowContent, ParagraphContent},
    DocxFile,
};
use crate::generator::image2md::{self, ImageProcessingMode};
use crate::config::SETTINGS;

pub fn run(file_stream: &[u8]) -> Result<String, String> {
    // Check if pandoc is available
    if is_pandoc_available() {
        run_with_pandoc(file_stream)
    } else {
        run_with_images(file_stream)
    }
}

fn is_pandoc_available() -> bool {
    Command::new("pandoc")
        .arg("--version")
        .output()
        .is_ok()
}

fn run_with_pandoc(file_stream: &[u8]) -> Result<String, String> {
    let cfg = &*SETTINGS.read().unwrap();

    // Create a temporary file for the DOCX input
    let temp_dir = std::env::temp_dir();
    let input_path = temp_dir.join("temp_input.docx");
    let output_path = temp_dir.join("temp_output.md");
    
    // Write DOCX data to temporary file
    std::fs::write(&input_path, file_stream)
        .map_err(|e| format!("Failed to write temporary DOCX file: {}", e))?;
    
    // Prepare pandoc command
    let mut cmd = Command::new("pandoc");
    cmd.arg(&input_path)
        .arg("-o")
        .arg(&output_path)
        .arg("-f")
        .arg("docx")
        .arg("-t")
        .arg("markdown");
    
    // Handle image extraction based on configuration
    if !cfg.image_path.as_os_str().is_empty() {
        // Extract images to configured directory
        cmd.arg("--extract-media")
            .arg(&cfg.image_path);
    }
    
    // Execute pandoc
    let output = cmd.output()
        .map_err(|e| format!("Failed to execute pandoc: {}", e))?;
    
    if !output.status.success() {
        let error_msg = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Pandoc execution failed: {}", error_msg));
    }
    
    // Read the generated markdown
    let mut markdown = std::fs::read_to_string(&output_path)
        .map_err(|e| format!("Failed to read pandoc output: {}", e))?;
    
    // Clean up temporary files
    let _ = std::fs::remove_file(&input_path);
    let _ = std::fs::remove_file(&output_path);
    
    // Post-process images if needed
    if !cfg.image_path.as_os_str().is_empty() {
        markdown = process_pandoc_images(markdown)?;
    } else {
        // Convert image references to base64 if no image_path is configured
        markdown = convert_image_refs_to_base64(markdown)?;
    }
    
    Ok(markdown)
}

fn process_pandoc_images(markdown: String) -> Result<String, String> {
    let cfg = &*SETTINGS.read().unwrap();
    
    // If we have an output path, calculate relative paths
    if let Some(output_path) = &cfg.output_path {
        if !output_path.as_os_str().is_empty() {
            // Calculate relative path from output file's directory to image directory
            let output_dir = output_path.parent().unwrap_or(Path::new("."));
            
            // Pandoc creates a 'media' subdirectory under the specified extract-media path
            let pandoc_media_path = cfg.image_path.join("media");
            
            if let Ok(relative_path) = pandoc_media_path.strip_prefix(output_dir) {
                let relative_str = relative_path.to_string_lossy();
                
                // Replace pandoc's absolute media paths with relative paths  
                let updated = markdown.replace(
                    &format!("]({})", pandoc_media_path.to_string_lossy()),
                    &format!("](./{})", relative_str)
                );
                return Ok(updated);
            }
        }
    }
    
    // If no output path or empty output path, use absolute paths
    Ok(markdown)
}

fn convert_image_refs_to_base64(markdown: String) -> Result<String, String> {
    // This is a simplified approach - in practice, you'd need to parse the markdown
    // and find image references, read the files, and convert them to base64
    // For now, we'll return the markdown as-is since pandoc without --extract-media
    // should embed images differently
    Ok(markdown)
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
                            // Process embedded images in drawings with proper mode
                            if let Some(image_md) = process_drawing_images_with_mode(images)? {
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

fn process_drawing_images_with_mode(images: &HashMap<String, Vec<u8>>) -> Result<Option<String>, String> {
    let cfg = &*SETTINGS.read().unwrap();
    
    // Determine processing mode based on configuration
    let mode = if cfg.image_path.as_os_str().is_empty() {
        ImageProcessingMode::Base64
    } else {
        ImageProcessingMode::SaveToFile
    };
    
    // Process the first available image (simplified approach)
    for (filename, image_data) in images {
        if filename.ends_with(".png") || 
           filename.ends_with(".jpg") || 
           filename.ends_with(".jpeg") ||
           filename.ends_with(".gif") ||
           filename.ends_with(".webp") {
            
            let image_md = image2md::run_with_mode(image_data, mode)?;
            
            // Handle relative paths if needed
            let final_md = if !cfg.image_path.as_os_str().is_empty() {
                adjust_image_path_in_markdown(image_md)?
            } else {
                image_md
            };
            
            return Ok(Some(format!("\n\n{}\n\n", final_md)));
        }
    }
    Ok(None)
}

fn adjust_image_path_in_markdown(markdown: String) -> Result<String, String> {
    let cfg = &*SETTINGS.read().unwrap();
    
    // If we have an output path, try to make image paths relative
    if let Some(output_path) = &cfg.output_path {
        if !output_path.as_os_str().is_empty() {
            // Calculate relative path from output file's directory to image directory
            let output_dir = output_path.parent().unwrap_or(Path::new("."));
            
            if let Ok(relative_path) = cfg.image_path.strip_prefix(output_dir) {
                // Replace absolute image paths with relative ones
                let relative_str = relative_path.to_string_lossy();
                return Ok(markdown.replace(
                    &format!("]({})", cfg.image_path.to_string_lossy()),
                    &format!("](./{})", relative_str)
                ));
            }
        }
    }
    
    // If no output path configured or can't make relative, return as-is
    Ok(markdown)
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
