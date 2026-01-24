use std::fs;
use std::path::PathBuf;

fn main() {
    // Get the path to the frontend dist directory
    let dist_path = PathBuf::from("../dist");

    // Check if dist directory exists, if not create a minimal placeholder
    if !dist_path.exists() {
        println!(
            "cargo:warning=Frontend dist directory not found, creating placeholder for build/test"
        );

        // Create the dist directory
        fs::create_dir_all(&dist_path).expect("Failed to create dist directory");

        // Create a minimal index.html placeholder
        let index_html = dist_path.join("index.html");
        fs::write(
            index_html,
            "<!DOCTYPE html><html><head><title>Placeholder</title></head><body></body></html>",
        )
        .expect("Failed to create placeholder index.html");

        println!("cargo:warning=Placeholder created. Run 'npm run build' for actual frontend.");
    }

    tauri_build::build()
}
