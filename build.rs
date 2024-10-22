use std::env;
use std::fs;
use std::path::Path;

fn main() {
    let boards_dir = Path::new("src/board");

    let supported_boards: Vec<String> = fs::read_dir(boards_dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .collect();

    let board_features: Vec<String> = env::vars()
        .filter_map(|(k, _)| k.strip_prefix("CARGO_FEATURE_BOARD_").map(|s| s.to_owned()))
        .collect();

    if board_features.len() != 1 {
        panic!("Must specify exactly 1 `board-` feature (got {board_features:?})");
    }

    let board_env = board_features.into_iter().next().unwrap();

    let Some(board) = supported_boards.iter().find(|b| b.to_ascii_uppercase().replace("-", "_") == board_env) else {
        panic!("No board directory found for {board_env} (supported: {supported_boards:?})");
    };

    println!("cargo::rustc-env=VIKING_BOARD_RS={board}/{board}.rs");
    println!("cargo:rustc-link-search=src/board/{board}");
    println!("cargo:rerun-if-changed=src/board/{board}/memory.x");
}
