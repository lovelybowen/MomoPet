use momopet_lib::pet_package::{pack_directory, validate_archive};
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
    let arguments = env::args().skip(1).collect::<Vec<_>>();

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
        _ => Err(
            "usage: momopet-package validate <file.momopet> | pack <source-dir> <file.momopet>"
                .to_owned(),
        ),
    }
}
