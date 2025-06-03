use infer;
mod config;
pub mod generator;
pub mod converter;

pub struct ConverterFile {
    pub file_path: Option<String>,
    pub file_stream: Vec<u8>,
}

// byte_stream -> String
pub fn convert(file: ConverterFile) -> Result<String, String> {
    let kind = infer::get(&file.file_stream)
        .ok_or_else(|| "Could not determine file type".to_string())?;

    let mime_type = kind.mime_type();

    if cfg!(debug_assertions) {
        dbg!(mime_type);
    }

    match mime_type {
        "audio/x-wav" | "audio/wav" | "audio/wave" => {
            generator::wav2md::run(&file.file_stream)
                .map_err(|e| format!("Failed to convert WAV: {}", e))
        }
        "application/vnd.openxmlformats-officedocument.wordprocessingml.document" => {
            generator::docx2md::run(&file.file_stream)
                .map_err(|e| format!("Failed to convert DOCX: {}", e))
        }
        "image/jpeg" | "image/png" | "image/gif" => {
            generator::image2md::run(&file.file_stream)
                .map_err(|e| format!("Failed to convert image: {}", e))
        }
        "application/vnd.openxmlformats-officedocument.presentationml.presentation" => {
            generator::pptx2md::run(&file.file_stream)
                .map_err(|e| format!("Failed to convert PPTX: {}", e))
        }
        "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet" => {
            let csvs = converter::xlsx2csv::xlsx_to_csv(&file.file_stream, None)
                .map_err(|e| format!("Failed to convert XLSX: {}", e))?;
            for (name, csv) in csvs.sheet_names.iter().zip(csvs.csv_data.iter()) {
                if cfg!(debug_assertions) {
                    dbg!(name);
                }
                let md = generator::csv2md::run(csv.as_bytes())
                    .map_err(|e| format!("Failed to convert CSV for sheet '{}': {}", name, e))?;
                return Ok(md);
            }
            Err("No sheets found in XLSX file".to_string())
        }
        "text/csv" | "application/csv" => {
            generator::csv2md::run(&file.file_stream)
                .map_err(|e| format!("Failed to convert CSV: {}", e))
        }
        _ => Err(format!("Unsupported file type: {}", mime_type)),
    }
}

pub fn convert_from_path(file_path: &str) -> Result<String, String> {
    let file_stream = std::fs::read(file_path)
        .map_err(|e| format!("Failed to read file {}: {}", file_path, e))?;

    let file = ConverterFile {
        file_path: Some(file_path.to_string()),
        file_stream,
    };

    convert(file)
}