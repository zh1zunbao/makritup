use clap::{Arg, Command};
use markitup;
use std::path::PathBuf;

fn main() {
    let matches = Command::new("markitup")
        .version("1.0.0")
        .author("Your Name <your.email@example.com>")
        .about("A markup conversion tool with AI enhancement capabilities")
        .arg(
            Arg::new("input")
                .help("Input file path")
                .required(true)
                .index(1),
        )
        .arg(
            Arg::new("output")
                .short('o')
                .long("output")
                .value_name("PATH")
                .help("Output file path"),
        )
        .arg(
            Arg::new("image-path")
                .short('i')
                .long("image-path")
                .value_name("PATH")
                .help("Path for image processing"),
        )
        .arg(
            Arg::new("ai-enable")
                .short('a')
                .long("ai-enable")
                .action(clap::ArgAction::SetTrue)
                .help("Enable AI enhancement features"),
        )
        .arg(
            Arg::new("no-ai")
                .long("no-ai")
                .action(clap::ArgAction::SetTrue)
                .help("Disable AI enhancement features")
                .conflicts_with("ai-enable"),
        )
        .get_matches();

    let file_path = matches.get_one::<String>("input").unwrap();

    // 收集CLI覆盖参数
    let image_path_override = matches.get_one::<String>("image-path").map(PathBuf::from);
    let output_path_override = matches.get_one::<String>("output").map(PathBuf::from);
    let ai_enable_override = if matches.get_flag("ai-enable") {
        Some(true)
    } else if matches.get_flag("no-ai") {
        Some(false)
    } else {
        None
    };

    // 使用CLI参数更新全局配置
    markitup::config::update_settings_with_cli_args(
        image_path_override,
        output_path_override,
        ai_enable_override,
    );

    // 获取更新后的配置
    let settings = markitup::config::get_settings();

    let output = markitup::convert_from_path(file_path);
    match output {
        Ok(markup) => {
            if let Some(output_path) = &settings.output_path {
                match std::fs::write(output_path, &markup) {
                    Ok(_) => println!("Output written to: {}", output_path.display()),
                    Err(err) => {
                        eprintln!("Error writing to file: {}", err);
                        std::process::exit(1);
                    }
                }
            } else {
                println!("{}", markup);
            }
        }
        Err(err) => {
            eprintln!("Error: {}", err);
            std::process::exit(1);
        }
    }
}
