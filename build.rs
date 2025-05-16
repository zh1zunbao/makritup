use std::env;

fn main() {
    let target = env::var("TARGET").unwrap();
    let lib_dir = if target.contains("windows") {
        "vosk/lib/vosk-win64-0.3.45"
    } else if target.contains("linux") {
        "vosk/lib/vosk-linux-x86_64-0.3.45"
    } else {
        panic!("Unsupported target: {}", target);
    };
    println!("cargo:rustc-link-search=native={}", lib_dir);
    println!("cargo:rustc-link-lib=vosk");
}
