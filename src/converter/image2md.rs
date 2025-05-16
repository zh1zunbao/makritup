use crate::config::SETTINGS;
use base64::Engine;


pub fn run(file_stream: &[u8]) -> Result<String, String> {
    let cfg = &SETTINGS;

    if file_stream.is_empty() {
        return Err("Input stream is empty".to_string());
    }

    // Encode the image data to base64
    let encoded = base64::engine::general_purpose::STANDARD.encode(file_stream);

    // Determine the MIME type of the image
    // Use the infer crate to get the MIME type
    let mime_type = infer::get(file_stream)
        .map(|kind| kind.mime_type().to_string())
        .unwrap_or_else(|| "application/octet-stream".to_string());

    let image_name = if cfg.is_ai_enpower {
        ai_generate_name(encoded.clone(), &mime_type)
    } else {
        // generate a timestamp-based name
        // name: pic-{timestamp}
        let timestamp = chrono::Utc::now().timestamp();
        format!("pic-{}", timestamp)
    };

    // Create the Markdown image syntax
    let md_content = format!("![{}](data:{};base64,{})", image_name, mime_type, encoded);

    Ok(md_content)
}


fn ai_generate_name(encoded: String, mime_type: &str) -> String {
    "AI-Generated-Image".to_string()
}
