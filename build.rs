fn main() {
    // Tell rustc to look for native libraries in the vosk/ directory
    println!("cargo:rustc-link-search=native=vosk");
    // The link name vosk corresponds to libvosk.so
    println!("cargo:rustc-link-lib=vosk");
}
