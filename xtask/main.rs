use std::{error::Error, fs, path::{self, PathBuf}, process::{exit, Command}};

fn main() {
    let args = std::env::args().collect::<Vec<_>>();
    let args = args.iter().map(|s| s.as_str()).collect::<Vec<_>>();
    match &args[1..] {
        ["dist"] => dist().unwrap(),
        ["clean"] => clean().unwrap(),
        _ => {
            println!("Usage: cargo xtask dist");
            exit(1);
        }
    }
}

fn boards() -> Result<Vec<(String, PathBuf)>, Box<dyn Error>> {
    fs::read_dir("board")?.map(|entry| {
        let board_name = entry?.file_name();
        let path = PathBuf::from("board").join(&board_name);
        let board_name_str = board_name.to_str().unwrap().to_string();
        Ok((board_name_str, path))
    }).collect()
}

fn dist() -> Result<(), Box<dyn Error>> {
    let mut failed = Vec::new();

    let dist_dir = path::absolute("dist")?;

    for (board_name, path) in boards()? {
        println!("Building {board_name}");

        let status = Command::new("cargo")
            .current_dir(path)
            .arg("build")
            .arg("--release")
            .arg("-Zunstable-options")
            .arg("--artifact-dir").arg(&dist_dir)
            .status();

        match status {
            Ok(status) if status.success() => {
            }
            _ => {
                failed.push(board_name);
            }
        }
    }

    if !failed.is_empty() {
        println!("Build failed for: {failed:?}");
        exit(1);
    } else {
        println!("Built all targets");
    }

    Ok(())
}

fn clean() -> Result<(), Box<dyn Error>> {
    for (board_name, path) in boards()? {
        println!("Cleaning {board_name}");

        let status = Command::new("cargo")
            .current_dir(path)
            .arg("clean")
            .status()?;
    }

    Ok(())
}
