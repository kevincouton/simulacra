//! Build the tracker site from `content/` into `site/dist/`.

use std::path::PathBuf;

fn main() {
    let root = std::env::var("SIMULACRA_ROOT").map(PathBuf::from).unwrap_or_else(|_| {
        let exe = std::env::current_exe().expect("cannot locate executable");
        // target/<profile>/deps| — walk up to the workspace root.
        exe.ancestors().nth(3).expect("cannot locate workspace root").to_path_buf()
    });
    let content = root.join("content");
    let output = root.join("site").join("dist");

    match simulacra_tracker::build_site(&content, &output) {
        Ok(written) => {
            println!("tracker site built: {} files -> {}", written.len(), output.display());
        }
        Err(e) => {
            eprintln!("tracker build failed: {e}");
            std::process::exit(1);
        }
    }
}
