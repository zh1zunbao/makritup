use markitup::converter::image2md::run;
use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    dbg!(&args);

    if args.len() < 2 {
        eprintln!("Usage: {} <file_path>", args[0]);
        std::process::exit(1);
    }

    let file_path = &args[1];

    let file_stream = std::fs::read(file_path)
        .map_err(|e| format!("Failed to read file {}: {}", file_path, e))
        .unwrap();

    let output = run(&file_stream)
        .map_err(|e| format!("Failed to convert image: {}", e))
        .unwrap();

    // Print the output to stdout
    // or handle it as needed
    println!("{}", output);
}
