use std::{error::Error, fs, path::{self, PathBuf}, process::{exit, Command}};

const STEP_HEADER: &str = const {
    if option_env!("GITHUB_ACTIONS").is_some() {
        "::group::"
    } else {
        "===========================\n"
    }
};
const STEP_FOOTER: &str = const {
    if option_env!("GITHUB_ACTIONS").is_some() {
        "::endgroup::"
    } else {
        "\n"
    }
};

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

    if dist_dir.exists() {
        fs::remove_dir_all(&dist_dir)?;
    }
    fs::create_dir_all(&dist_dir)?;

    for (board_name, path) in boards()? {
        println!("{STEP_HEADER}Building {board_name}");

        let status = Command::new("cargo")
            .current_dir(path)
            .arg("build")
            .arg("--release")
            .arg("-Zunstable-options")
            .arg("--artifact-dir").arg(&dist_dir)
            .status().is_ok_and(|s| s.success());

        if !status {
            failed.push(board_name);
        }

        println!("{STEP_FOOTER}");
    }

    for entry in fs::read_dir(&dist_dir)? {
        let entry = entry?;
        let orig_path = entry.path();
        let elf_path = orig_path.with_extension("elf");
        fs::rename(orig_path, &elf_path)?;

        if elf_path.file_name().unwrap().to_str().unwrap().starts_with("viking-firmware-rp") {
            let uf2_path = elf_path.with_extension("uf2");
            let picotool_success = Command::new("picotool")
                .arg("uf2")
                .arg("convert")
                .arg(&elf_path)
                .arg(&uf2_path)
                .status().is_ok_and(|s| s.success());

            if !picotool_success {
                eprintln!("Failed to run picotool for {}", elf_path.display());
                failed.push(uf2_path.file_name().unwrap().to_string_lossy().to_string());
                continue;
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
        println!("{STEP_HEADER}Cleaning {board_name}");

        let status = Command::new("cargo")
            .current_dir(path)
            .arg("clean")
            .status()?;

        println!("{STEP_FOOTER}");
    }

    Ok(())
}
