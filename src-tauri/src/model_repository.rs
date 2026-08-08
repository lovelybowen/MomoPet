use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{BufReader, Read},
    path::{Component, Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};
use tauri::{AppHandle, Manager, Runtime, command};

const INSTALL_DIRECTORY: &str = "custom-models";
const METADATA_FILE: &str = ".momopet-model.json";

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ModelMode {
    Standard,
    Keyboard,
    Gamepad,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstalledModel {
    pub id: String,
    pub path: String,
    pub mode: ModelMode,
    pub is_builtin: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct InstalledModelMetadata {
    id: String,
    model_directory: String,
    mode: ModelMode,
    is_builtin: bool,
}

#[command]
pub fn import_live2d_model<R: Runtime>(
    app_handle: AppHandle<R>,
    source_dir: String,
) -> Result<InstalledModel, String> {
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|error| error.to_string())?;

    import_model(Path::new(&source_dir), &app_data_dir).map_err(|error| error.to_string())
}

#[command]
pub fn list_installed_live2d_models<R: Runtime>(
    app_handle: AppHandle<R>,
) -> Result<Vec<InstalledModel>, String> {
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|error| error.to_string())?;

    list_models(&app_data_dir).map_err(|error| error.to_string())
}

#[command]
pub fn remove_live2d_model<R: Runtime>(
    app_handle: AppHandle<R>,
    model_id: String,
) -> Result<(), String> {
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|error| error.to_string())?;

    remove_model(&model_id, &app_data_dir).map_err(|error| error.to_string())
}

fn import_model(source_dir: &Path, app_data_dir: &Path) -> Result<InstalledModel, ModelError> {
    let source_metadata = fs::symlink_metadata(source_dir).map_err(|error| {
        ModelError::new(format!(
            "cannot read model directory '{}': {error}",
            source_dir.display()
        ))
    })?;

    if source_metadata.file_type().is_symlink() || !source_metadata.is_dir() {
        return Err(ModelError::new(
            "the selected model source must be a real directory",
        ));
    }

    let files = collect_files(source_dir, false)?;
    let model_files: Vec<&PathBuf> = files
        .iter()
        .filter(|path| path.to_string_lossy().ends_with(".model3.json"))
        .collect();

    if model_files.len() != 1 {
        return Err(ModelError::new(format!(
            "expected exactly one .model3.json file, found {}",
            model_files.len()
        )));
    }

    let model_file = model_files[0];
    validate_model_references(source_dir, model_file)?;

    let id = hash_files(source_dir, &files)?;
    let model_directory = model_file
        .parent()
        .and_then(|path| path.strip_prefix(source_dir).ok())
        .unwrap_or_else(|| Path::new(""));
    let mode = detect_mode(model_file.parent().unwrap_or(source_dir))?;
    let models_dir = app_data_dir.join(INSTALL_DIRECTORY);
    let destination = models_dir.join(&id);

    fs::create_dir_all(&models_dir)?;

    if destination.exists() {
        return read_installed_model(&destination, &id);
    }

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| ModelError::new(error.to_string()))?
        .as_nanos();
    let staging = models_dir.join(format!(".import-{id}-{}-{timestamp}", std::process::id()));

    fs::create_dir(&staging)?;

    let install_result = (|| {
        copy_files(source_dir, &staging, &files)?;

        let metadata = InstalledModelMetadata {
            id: id.clone(),
            model_directory: path_to_portable_string(model_directory),
            mode: mode.clone(),
            is_builtin: false,
        };
        let metadata_bytes = serde_json::to_vec_pretty(&metadata)?;

        fs::write(staging.join(METADATA_FILE), metadata_bytes)?;
        fs::rename(&staging, &destination)?;

        Ok::<(), ModelError>(())
    })();

    if let Err(error) = install_result {
        let _ = fs::remove_dir_all(&staging);
        return Err(error);
    }

    Ok(InstalledModel {
        id,
        path: destination
            .join(model_directory)
            .to_string_lossy()
            .into_owned(),
        mode,
        is_builtin: false,
    })
}

fn list_models(app_data_dir: &Path) -> Result<Vec<InstalledModel>, ModelError> {
    let models_dir = app_data_dir.join(INSTALL_DIRECTORY);

    if !models_dir.exists() {
        return Ok(Vec::new());
    }

    let mut entries = fs::read_dir(models_dir)?.collect::<Result<Vec<_>, _>>()?;
    entries.sort_by_key(|entry| entry.file_name());

    entries
        .into_iter()
        .filter_map(|entry| {
            let id = entry.file_name().to_string_lossy().into_owned();

            is_model_id(&id).then_some((entry.path(), id))
        })
        .map(|(path, id)| read_installed_model(&path, &id))
        .collect()
}

fn remove_model(model_id: &str, app_data_dir: &Path) -> Result<(), ModelError> {
    if !is_model_id(model_id) {
        return Err(ModelError::new("invalid installed model ID"));
    }

    let destination = app_data_dir.join(INSTALL_DIRECTORY).join(model_id);
    let installed = read_installed_model(&destination, model_id)?;

    if installed.is_builtin {
        return Err(ModelError::new("built-in models cannot be removed"));
    }

    fs::remove_dir_all(destination)?;
    Ok(())
}

fn read_installed_model(path: &Path, expected_id: &str) -> Result<InstalledModel, ModelError> {
    let directory_metadata = fs::symlink_metadata(path)?;

    if directory_metadata.file_type().is_symlink() || !directory_metadata.is_dir() {
        return Err(ModelError::new(
            "installed model path is not a real directory",
        ));
    }

    let metadata: InstalledModelMetadata =
        serde_json::from_slice(&fs::read(path.join(METADATA_FILE))?)?;

    if metadata.id != expected_id || !is_model_id(&metadata.id) {
        return Err(ModelError::new(
            "installed model metadata has an invalid ID",
        ));
    }

    let relative_model_dir = Path::new(&metadata.model_directory);

    validate_relative_path(relative_model_dir, true)?;

    let model_path = path.join(relative_model_dir);
    let model_files = collect_files(&model_path, model_path == path)?
        .into_iter()
        .filter(|candidate| candidate.to_string_lossy().ends_with(".model3.json"))
        .count();

    if model_files != 1 {
        return Err(ModelError::new(
            "installed model no longer contains exactly one .model3.json file",
        ));
    }

    Ok(InstalledModel {
        id: metadata.id,
        path: model_path.to_string_lossy().into_owned(),
        mode: metadata.mode,
        is_builtin: metadata.is_builtin,
    })
}

fn collect_files(root: &Path, allow_root_metadata: bool) -> Result<Vec<PathBuf>, ModelError> {
    let mut files = Vec::new();
    collect_files_from(root, root, allow_root_metadata, &mut files)?;
    files.sort();
    Ok(files)
}

fn collect_files_from(
    root: &Path,
    directory: &Path,
    allow_root_metadata: bool,
    files: &mut Vec<PathBuf>,
) -> Result<(), ModelError> {
    let mut entries = fs::read_dir(directory)?.collect::<Result<Vec<_>, _>>()?;
    entries.sort_by_key(|entry| entry.file_name());

    for entry in entries {
        let path = entry.path();
        let metadata = fs::symlink_metadata(&path)?;

        if metadata.file_type().is_symlink() {
            return Err(ModelError::new(format!(
                "symbolic links are not allowed: {}",
                path.display()
            )));
        }

        if metadata.is_dir() {
            collect_files_from(root, &path, allow_root_metadata, files)?;
        } else if metadata.is_file() {
            let relative = path
                .strip_prefix(root)
                .map_err(|_| ModelError::new("model file escaped its source directory"))?;

            if relative == Path::new(METADATA_FILE) {
                if allow_root_metadata {
                    continue;
                }

                return Err(ModelError::new(format!(
                    "{METADATA_FILE} is reserved for MomoPet metadata"
                )));
            }

            files.push(path);
        } else {
            return Err(ModelError::new(format!(
                "unsupported filesystem entry: {}",
                path.display()
            )));
        }
    }

    Ok(())
}

fn validate_model_references(source_dir: &Path, model_file: &Path) -> Result<(), ModelError> {
    let model_json: Value = serde_json::from_slice(&fs::read(model_file)?)?;
    let file_references = model_json
        .get("FileReferences")
        .ok_or_else(|| ModelError::new("model is missing FileReferences"))?;
    let mut references = Vec::new();

    collect_reference_paths(file_references, None, &mut references)?;

    if references.is_empty() {
        return Err(ModelError::new(
            "model does not reference any runtime files",
        ));
    }

    let model_directory = model_file
        .parent()
        .ok_or_else(|| ModelError::new("model configuration has no parent directory"))?;

    for reference in references {
        let relative = Path::new(&reference);
        validate_relative_path(relative, false)?;

        let referenced_path = model_directory.join(relative);

        if !referenced_path.starts_with(source_dir) {
            return Err(ModelError::new(format!(
                "model reference escapes the source directory: {reference}"
            )));
        }

        let metadata = fs::symlink_metadata(&referenced_path).map_err(|_| {
            ModelError::new(format!("referenced model file is missing: {reference}"))
        })?;

        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err(ModelError::new(format!(
                "model reference is not a regular file: {reference}"
            )));
        }
    }

    Ok(())
}

fn collect_reference_paths(
    value: &Value,
    key: Option<&str>,
    references: &mut Vec<String>,
) -> Result<(), ModelError> {
    const SINGLE_PATH_KEYS: &[&str] = &[
        "Moc",
        "Physics",
        "Pose",
        "UserData",
        "DisplayInfo",
        "File",
        "Sound",
    ];

    match value {
        Value::String(path) if key.is_some_and(|key| SINGLE_PATH_KEYS.contains(&key)) => {
            references.push(path.clone());
        }
        Value::Array(items) if key == Some("Textures") => {
            for item in items {
                let path = item
                    .as_str()
                    .ok_or_else(|| ModelError::new("Textures must contain only file paths"))?;
                references.push(path.to_owned());
            }
        }
        Value::Array(items) => {
            for item in items {
                collect_reference_paths(item, key, references)?;
            }
        }
        Value::Object(object) => {
            for (child_key, child_value) in object {
                collect_reference_paths(child_value, Some(child_key), references)?;
            }
        }
        _ => {}
    }

    Ok(())
}

fn validate_relative_path(path: &Path, allow_empty: bool) -> Result<(), ModelError> {
    let mut has_normal_component = false;

    for component in path.components() {
        match component {
            Component::Normal(_) => has_normal_component = true,
            Component::CurDir => {}
            _ => {
                return Err(ModelError::new(format!(
                    "absolute paths and path traversal are not allowed: {}",
                    path.display()
                )));
            }
        }
    }

    if !allow_empty && !has_normal_component {
        return Err(ModelError::new("model reference cannot be empty"));
    }

    Ok(())
}

fn detect_mode(model_directory: &Path) -> Result<ModelMode, ModelError> {
    let right_keys = model_directory.join("resources").join("right-keys");

    if !right_keys.is_dir() {
        return Ok(ModelMode::Standard);
    }

    let entries = fs::read_dir(right_keys)?.collect::<Result<Vec<_>, _>>()?;

    if entries.iter().any(|entry| {
        entry
            .path()
            .file_stem()
            .is_some_and(|file_stem| file_stem == "East")
    }) {
        return Ok(ModelMode::Gamepad);
    }

    if entries.is_empty() {
        Ok(ModelMode::Standard)
    } else {
        Ok(ModelMode::Keyboard)
    }
}

fn hash_files(root: &Path, files: &[PathBuf]) -> Result<String, ModelError> {
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 16 * 1024];

    for path in files {
        let relative = path
            .strip_prefix(root)
            .map_err(|_| ModelError::new("model file escaped its source directory"))?;
        let relative = path_to_portable_string(relative);

        hasher.update((relative.len() as u64).to_le_bytes());
        hasher.update(relative.as_bytes());

        let mut reader = BufReader::new(fs::File::open(path)?);

        loop {
            let read = reader.read(&mut buffer)?;

            if read == 0 {
                break;
            }

            hasher.update(&buffer[..read]);
        }
    }

    Ok(format!("{:x}", hasher.finalize()))
}

fn copy_files(source: &Path, destination: &Path, files: &[PathBuf]) -> Result<(), ModelError> {
    for source_file in files {
        let relative = source_file
            .strip_prefix(source)
            .map_err(|_| ModelError::new("model file escaped its source directory"))?;
        let destination_file = destination.join(relative);

        if let Some(parent) = destination_file.parent() {
            fs::create_dir_all(parent)?;
        }

        fs::copy(source_file, destination_file)?;
    }

    Ok(())
}

fn path_to_portable_string(path: &Path) -> String {
    path.components()
        .filter_map(|component| match component {
            Component::Normal(value) => Some(value.to_string_lossy()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("/")
}

fn is_model_id(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|character| character.is_ascii_hexdigit() && !character.is_ascii_uppercase())
}

#[derive(Debug)]
struct ModelError(String);

impl ModelError {
    fn new(message: impl Into<String>) -> Self {
        Self(message.into())
    }
}

impl std::fmt::Display for ModelError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl From<std::io::Error> for ModelError {
    fn from(error: std::io::Error) -> Self {
        Self::new(error.to_string())
    }
}

impl From<serde_json::Error> for ModelError {
    fn from(error: serde_json::Error) -> Self {
        Self::new(error.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static TEMP_COUNTER: AtomicUsize = AtomicUsize::new(0);

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new(name: &str) -> Self {
            let counter = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "momopet-model-test-{}-{counter}-{name}",
                std::process::id()
            ));
            let _ = fs::remove_dir_all(&path);
            fs::create_dir_all(&path).unwrap();
            Self(path)
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn write_valid_model(path: &Path) {
        fs::create_dir_all(path.join("textures")).unwrap();
        fs::write(path.join("pet.moc3"), b"moc").unwrap();
        fs::write(path.join("textures/texture.png"), b"png").unwrap();
        fs::write(
            path.join("pet.model3.json"),
            br#"{
                "Version": 3,
                "FileReferences": {
                    "Moc": "pet.moc3",
                    "Textures": ["textures/texture.png"]
                }
            }"#,
        )
        .unwrap();
    }

    #[test]
    fn imports_lists_removes_and_deduplicates_a_valid_model() {
        let source = TestDirectory::new("valid-source");
        let app_data = TestDirectory::new("valid-app-data");
        write_valid_model(&source.0);

        let first = import_model(&source.0, &app_data.0).unwrap();
        let second = import_model(&source.0, &app_data.0).unwrap();

        assert_eq!(first, second);
        assert_eq!(first.mode, ModelMode::Standard);
        assert!(!first.is_builtin);
        assert_eq!(list_models(&app_data.0).unwrap(), vec![first.clone()]);

        remove_model(&first.id, &app_data.0).unwrap();
        assert!(list_models(&app_data.0).unwrap().is_empty());
    }

    #[test]
    fn rejects_missing_or_multiple_model_entries() {
        let empty_source = TestDirectory::new("empty-source");
        let app_data = TestDirectory::new("entry-app-data");
        let error = import_model(&empty_source.0, &app_data.0).unwrap_err();
        assert!(error.to_string().contains("found 0"));

        write_valid_model(&empty_source.0);
        fs::write(empty_source.0.join("second.model3.json"), b"{}").unwrap();
        let error = import_model(&empty_source.0, &app_data.0).unwrap_err();
        assert!(error.to_string().contains("found 2"));
    }

    #[test]
    fn rejects_missing_references_and_path_traversal() {
        let missing_source = TestDirectory::new("missing-reference");
        let app_data = TestDirectory::new("reference-app-data");
        write_valid_model(&missing_source.0);
        fs::remove_file(missing_source.0.join("pet.moc3")).unwrap();
        let error = import_model(&missing_source.0, &app_data.0).unwrap_err();
        assert!(error.to_string().contains("is missing"));

        let traversal_source = TestDirectory::new("traversal-source");
        write_valid_model(&traversal_source.0);
        fs::write(
            traversal_source.0.join("pet.model3.json"),
            br#"{"FileReferences":{"Moc":"../outside.moc3"}}"#,
        )
        .unwrap();
        let error = import_model(&traversal_source.0, &app_data.0).unwrap_err();
        assert!(error.to_string().contains("path traversal"));
    }

    #[cfg(unix)]
    #[test]
    fn rejects_symbolic_links() {
        use std::os::unix::fs::symlink;

        let source = TestDirectory::new("symlink-source");
        let app_data = TestDirectory::new("symlink-app-data");
        write_valid_model(&source.0);
        symlink(source.0.join("pet.moc3"), source.0.join("linked.moc3")).unwrap();

        let error = import_model(&source.0, &app_data.0).unwrap_err();
        assert!(error.to_string().contains("symbolic links"));
    }

    #[test]
    fn detects_keyboard_and_gamepad_profiles() {
        let keyboard_source = TestDirectory::new("keyboard-source");
        let keyboard_data = TestDirectory::new("keyboard-data");
        write_valid_model(&keyboard_source.0);
        fs::create_dir_all(keyboard_source.0.join("resources/right-keys")).unwrap();
        fs::write(
            keyboard_source.0.join("resources/right-keys/Arrow.png"),
            b"png",
        )
        .unwrap();
        let keyboard = import_model(&keyboard_source.0, &keyboard_data.0).unwrap();
        assert_eq!(keyboard.mode, ModelMode::Keyboard);

        let gamepad_source = TestDirectory::new("gamepad-source");
        let gamepad_data = TestDirectory::new("gamepad-data");
        write_valid_model(&gamepad_source.0);
        fs::create_dir_all(gamepad_source.0.join("resources/right-keys")).unwrap();
        fs::write(
            gamepad_source.0.join("resources/right-keys/East.png"),
            b"png",
        )
        .unwrap();
        let gamepad = import_model(&gamepad_source.0, &gamepad_data.0).unwrap();
        assert_eq!(gamepad.mode, ModelMode::Gamepad);
    }

    #[test]
    fn removal_cannot_escape_the_installed_model_directory() {
        let app_data = TestDirectory::new("remove-app-data");

        let error = remove_model("../outside", &app_data.0).unwrap_err();
        assert!(error.to_string().contains("invalid installed model ID"));
    }
}
