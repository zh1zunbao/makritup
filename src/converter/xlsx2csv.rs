use ooxml;

/// Configuration for xlsx to csv conversion
pub struct Xlsx2CsvConfig {
    /// Delimiter for CSV output (default: comma)
    pub delimiter: u8,
    /// Whether to use first row as header for column sizing
    pub use_header: bool,
}

impl Default for Xlsx2CsvConfig {
    fn default() -> Self {
        Self {
            delimiter: b',',
            use_header: false,
        }
    }
}

/// Result of xlsx to csv conversion
pub struct Xlsx2CsvResult {
    /// Sheet names in order
    pub sheet_names: Vec<String>,
    /// CSV content for each sheet
    pub csv_data: Vec<String>,
}

impl Xlsx2CsvResult {
    /// Get CSV data by sheet name
    pub fn get_by_name(&self, name: &str) -> Option<&String> {
        self.sheet_names
            .iter()
            .position(|n| n == name)
            .map(|i| &self.csv_data[i])
    }
    
    /// Get the first sheet's CSV data
    pub fn first(&self) -> Option<&String> {
        self.csv_data.first()
    }
}

/// Convert xlsx byte data to CSV strings
pub fn xlsx_to_csv(data: &[u8], config: Option<Xlsx2CsvConfig>) -> Result<Xlsx2CsvResult, String> {
    let config = config.unwrap_or_default();
    
    // Write to temporary file since ooxml doesn't support reading from cursor
    let temp_dir = std::env::temp_dir();
    let temp_file = temp_dir.join(format!("temp_xlsx_{}.xlsx", std::process::id()));
    
    std::fs::write(&temp_file, data)
        .map_err(|e| format!("Failed to write temp file: {}", e))?;
    
    let xlsx = ooxml::document::SpreadsheetDocument::open(&temp_file)
        .map_err(|e| format!("Failed to open xlsx: {}", e))?;
        
    let workbook = xlsx.get_workbook();
    let sheet_names = workbook.worksheet_names();
    
    if sheet_names.is_empty() {
        let _ = std::fs::remove_file(&temp_file);
        return Err("No sheets found in xlsx file".to_string());
    }
    
    let mut csv_data = Vec::new();
    
    for sheet_name in &sheet_names {
        let csv_string = worksheet_to_csv_string(&workbook, sheet_name, &config)
            .map_err(|e| format!("Failed to convert sheet '{}': {}", sheet_name, e))?;
        csv_data.push(csv_string);
    }
    
    // Clean up temp file
    let _ = std::fs::remove_file(&temp_file);
    
    Ok(Xlsx2CsvResult {
        sheet_names,
        csv_data,
    })
}

/// Convert a single worksheet to CSV string
fn worksheet_to_csv_string(
    workbook: &ooxml::document::Workbook,
    sheet_name: &str,
    config: &Xlsx2CsvConfig,
) -> Result<String, String> {
    let worksheet = workbook
        .get_worksheet_by_name(sheet_name)
        .ok_or_else(|| format!("Sheet '{}' not found", sheet_name))?;
    
    let mut output = Vec::new();
    {
        let mut writer = csv::WriterBuilder::new()
            .delimiter(config.delimiter)
            .from_writer(&mut output);
        
        let mut rows_iter = worksheet.rows();
        
        if config.use_header {
            if let Some(header_row) = rows_iter.next() {
                let header_cells: Vec<_> = header_row.collect();
                let column_count = header_cells
                    .iter()
                    .position(|cell| cell.is_empty())
                    .unwrap_or(header_cells.len());
                
                // Write header row
                let cols: Vec<String> = header_cells
                    .iter()
                    .take(column_count)
                    .map(|cell| cell.to_string().unwrap_or_default())
                    .collect();
                writer.write_record(&cols)
                    .map_err(|e| format!("Failed to write header: {}", e))?;
                
                // Write remaining rows with fixed column count
                for row in rows_iter {
                    let row_cells: Vec<_> = row.collect();
                    let cols: Vec<String> = row_cells
                        .iter()
                        .take(column_count)
                        .map(|cell| cell.to_string().unwrap_or_default())
                        .collect();
                    writer.write_record(&cols)
                        .map_err(|e| format!("Failed to write row: {}", e))?;
                }
            }
        } else {
            // Write all rows as-is
            for row in rows_iter {
                let row_cells: Vec<_> = row.collect();
                let cols: Vec<String> = row_cells
                    .iter()
                    .map(|cell| cell.to_string().unwrap_or_default())
                    .collect();
                writer.write_record(&cols)
                    .map_err(|e| format!("Failed to write row: {}", e))?;
            }
        }
        
        writer.flush()
            .map_err(|e| format!("Failed to flush writer: {}", e))?;
    } // writer is dropped here, releasing the borrow on output
    
    String::from_utf8(output)
        .map_err(|e| format!("Failed to convert to UTF-8: {}", e))
}

/// Convenience function to convert xlsx bytes to CSV with default settings
pub fn xlsx_to_csv_simple(data: &[u8]) -> Result<Vec<String>, String> {
    let result = xlsx_to_csv(data, None)?;
    Ok(result.csv_data)
}

/// Convenience function to get just the first sheet as CSV
pub fn xlsx_to_csv_first_sheet(data: &[u8]) -> Result<String, String> {
    let result = xlsx_to_csv(data, None)?;
    result.first()
        .ok_or_else(|| "No sheets found".to_string())
        .map(|s| s.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_config_default() {
        let config = Xlsx2CsvConfig::default();
        assert_eq!(config.delimiter, b',');
        assert_eq!(config.use_header, false);
    }
}
