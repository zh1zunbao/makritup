use infer;
mod config;
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
            converter::wav2md::run(&file.file_stream)
                .map_err(|e| format!("Failed to convert WAV: {}", e))
        }
        "application/vnd.openxmlformats-officedocument.wordprocessingml.document" => {
            converter::docx2md::run(&file.file_stream)
                .map_err(|e| format!("Failed to convert DOCX: {}", e))
        }
        "image/jpeg" | "image/png" | "image/gif" => {
            converter::image2md::run(&file.file_stream)
                .map_err(|e| format!("Failed to convert image: {}", e))
        }
        "application/vnd.openxmlformats-officedocument.presentationml.presentation" => {
            converter::pptx2md::run(&file.file_stream)
                .map_err(|e| format!("Failed to convert PPTX: {}", e))
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