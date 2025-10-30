use base64::prelude::*;
use std::{env, fs, path::Path};

fn ensure_icon_png() {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let icons_dir = Path::new(&manifest_dir).join("icons");
    let icon_path = icons_dir.join("icon.png");
    if icon_path.exists() {
        return;
    }
    let _ = fs::create_dir_all(&icons_dir);
    // 1x1 transparent PNG (base64)
    const PNG_B64: &str = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR4nGNgYAAAAAMAASsJTYQAAAAASUVORK5CYII=";
    let bytes = BASE64_STANDARD.decode(PNG_B64).expect("decode icon png");
    fs::write(&icon_path, bytes).expect("write icon png");
}

fn main() {
    ensure_icon_png();
    tauri_build::build();
}
