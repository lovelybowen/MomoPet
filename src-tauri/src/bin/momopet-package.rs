use momopet_lib::pet_package::{
    pack_directory, pack_sprite_sheet, validate_archive, validate_directory,
};
use std::{env, path::Path, process::ExitCode};

fn main() -> ExitCode {
    match run() {
        Ok(message) => {
            println!("{message}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("MomoPet package error: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<String, String> {
    let mut arguments = env::args().skip(1).collect::<Vec<_>>();
    if arguments.get(1).is_some_and(|argument| argument == "--") {
        arguments.remove(1);
    }

    match arguments.as_slice() {
        [command, package] if command == "validate" => {
            let validated =
                validate_archive(Path::new(package)).map_err(|error| error.to_string())?;
            Ok(format!(
                "Valid MomoPet package: {} v{} ({} actions, sha256 {})",
                validated.manifest.id,
                validated.manifest.version,
                validated.manifest.actions.len(),
                validated.content_digest
            ))
        }
        [command, source, output] if command == "pack" => {
            let validated = pack_directory(Path::new(source), Path::new(output))
                .map_err(|error| error.to_string())?;
            Ok(format!(
                "Created {}: {} v{} (sha256 {})",
                output, validated.manifest.id, validated.manifest.version, validated.content_digest
            ))
        }
        [command, source] if command == "validate-dir" => {
            let validated =
                validate_directory(Path::new(source)).map_err(|error| error.to_string())?;
            Ok(format!(
                "Valid MomoPet directory: {} v{} ({} actions, sha256 {})",
                validated.manifest.id,
                validated.manifest.version,
                validated.manifest.actions.len(),
                validated.content_digest
            ))
        }
        [command, source, output, columns] if command == "spritesheet" => {
            let columns = columns
                .parse::<u32>()
                .map_err(|_| "sprite sheet columns must be an integer".to_owned())?;
            let packed = pack_sprite_sheet(Path::new(source), Path::new(output), columns)
                .map_err(|error| error.to_string())?;
            Ok(format!(
                "Created {}: {} frames, {}x{} grid, {}x{} per frame",
                output,
                packed.frame_count,
                packed.columns,
                packed.rows,
                packed.frame_width,
                packed.frame_height
            ))
        }
        _ => Err(
            "usage: momopet-package validate <file.momopet> | validate-dir <source-dir> | pack <source-dir> <file.momopet> | spritesheet <frames-dir> <sheet.png> <columns>".to_owned(),
        ),
    }
}
