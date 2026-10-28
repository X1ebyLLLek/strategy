//! Builds into the game the portraits of `assets/portraits` (people met in the game get one each):
//! the game knows how many there are and draws them from memory, so it is one file. A new
//! portrait in the folder is picked up by itself.

use std::path::Path;
use std::{env, fs};

fn main() {
    // the game's window asks for the strong video card on a laptop with two (NVIDIA Optimus,
    // AMD PowerXpress read these exported numbers; without them the window ran on the weak one)
    if env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc") {
        println!("cargo:rustc-link-arg-bin=play=/EXPORT:NvOptimusEnablement");
        println!("cargo:rustc-link-arg-bin=play=/EXPORT:AmdPowerXpressRequestHighPerformance");
    }
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("assets").join("portraits");
    println!("cargo:rerun-if-changed=assets/portraits");
    let mut files: Vec<_> = fs::read_dir(&dir)
        .map(|d| d.flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|x| x == "jpg")).collect())
        .unwrap_or_default();
    files.sort();
    let mut s = String::from("/// The portraits, from assets/portraits (name without `.jpg`, contents).\npub static PORTRAITS: &[(&str, &[u8])] = &[\n");
    for p in &files {
        let name = p.file_stem().unwrap().to_string_lossy();
        s += &format!("    ({:?}, include_bytes!({:?})),\n", name, p.to_string_lossy());
    }
    s += "];\n";
    fs::write(Path::new(&env::var("OUT_DIR").unwrap()).join("portraits.rs"), s).unwrap();
}
