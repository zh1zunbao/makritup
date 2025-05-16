use std::io::Write;
use std::process::{Command, Stdio};
use crate::config::SETTINGS;

pub fn run(file_stream: &[u8]) -> Result<String, String> {

    let cfg = &SETTINGS;
    let images_path = cfg.image_path.to_str()
        .ok_or_else(|| "Failed to convert image path to string".to_string())?;
    // 1) Build the command

    let pandoc_path = "pandoc"; // Adjust this if pandoc is not in your PATH
    let mut child = Command::new(pandoc_path)
        .args(&[
            "-f",
            "docx",
            "-t",
            "gfm",
            "--extract-media",
            images_path,
            "-o",
            "-", // write output to stdout
            "-", // read input from stdin
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("failed to spawn pandoc: {}", e))?;

    // 2) Feed the DOCX bytes into pandoc’s stdin
    {
        let stdin = child
            .stdin
            .as_mut()
            .ok_or_else(|| "failed to open stdin".to_string())?;
        stdin
            .write_all(file_stream)
            .map_err(|e| format!("failed to write to pandoc stdin: {}", e))?;
    }

    // 3) Wait for pandoc to finish and collect its output
    let output = child
        .wait_with_output()
        .map_err(|e| format!("failed to read pandoc output: {}", e))?;

    // 4) Return stdout on success, or stderr on failure
    if output.status.success() {
        String::from_utf8(output.stdout).map_err(|e| format!("output was not valid UTF‐8: {}", e))
    } else {
        Err(String::from_utf8_lossy(&output.stderr).into_owned())
    }
}
