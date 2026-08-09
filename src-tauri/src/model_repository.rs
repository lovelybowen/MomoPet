use crate::pet_package::{
    INTERNAL_METADATA_FILE, ModelMode, PetAction, PetAuthor, PetInput, PetLicense, PetManifest,
    extract_and_validate, validate_installed_directory, validate_package_id,
};
use semver::Version;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::HashSet,
    fs,
    path::{Component, Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};
use tauri::{AppHandle, Manager, Runtime, command};

const INSTALL_DIRECTORY: &str = "custom-models";
const BUILTIN_DIRECTORY: &str = "assets/models";
const METADATA_FORMAT_VERSION: u32 = 1;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstalledModel {
    pub id: String,
    pub version: String,
    pub name: String,
    pub description: Option<String>,
    pub authors: Vec<PetAuthor>,
    pub license: Option<PetLicense>,
    pub path: String,
    pub entry_path: String,
    pub resource_path: String,
    pub cover_path: Option<String>,
    pub background_path: Option<String>,
    pub mode: ModelMode,
    pub input: Option<PetInput>,
    pub actions: Vec<PetAction>,
    pub is_builtin: bool,
    pub is_legacy: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct InstalledPackageMetadata {
    format_version: u32,
    content_digest: String,
    manifest: PetManifest,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct LegacyInstalledModelMetadata {
    id: String,
    model_directory: String,
    mode: ModelMode,
    is_builtin: bool,
}

enum InstalledMetadata {
    Package(Box<InstalledPackageMetadata>),
    Legacy(LegacyInstalledModelMetadata),
}

#[command]
pub fn import_pet_package<R: Runtime>(
    app_handle: AppHandle<R>,
    source_path: String,
) -> Result<InstalledModel, String> {
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|error| error.to_string())?;
    let builtin_models_dir = app_handle
        .path()
        .resource_dir()
        .map_err(|error| error.to_string())?
        .join(BUILTIN_DIRECTORY);

    import_package(Path::new(&source_path), &app_data_dir, &builtin_models_dir)
        .map_err(|error| error.to_string())
}

#[command]
pub fn list_installed_pets<R: Runtime>(
    app_handle: AppHandle<R>,
) -> Result<Vec<InstalledModel>, String> {
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|error| error.to_string())?;
    let builtin_models_dir = app_handle
        .path()
        .resource_dir()
        .map_err(|error| error.to_string())?
        .join(BUILTIN_DIRECTORY);

    list_models(&app_data_dir, &builtin_models_dir).map_err(|error| error.to_string())
}

#[command]
pub fn remove_installed_pet<R: Runtime>(
    app_handle: AppHandle<R>,
    pet_id: String,
) -> Result<(), String> {
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|error| error.to_string())?;

    remove_model(&pet_id, &app_data_dir).map_err(|error| error.to_string())
}

fn import_package(
    source: &Path,
    app_data_dir: &Path,
    builtin_models_dir: &Path,
) -> Result<InstalledModel, ModelError> {
    let models_dir = app_data_dir.join(INSTALL_DIRECTORY);
    fs::create_dir_all(&models_dir)?;

    let timestamp = unique_timestamp()?;
    let staging = models_dir.join(format!(".import-{}-{timestamp}", std::process::id()));
    fs::create_dir(&staging)?;

    let install_result = (|| {
        let validated = extract_and_validate(source, &staging)?;
        let id = validated.manifest.id.clone();
        if builtin_models_dir.join(&id).exists() {
            return Err(ModelError::new(format!(
                "package ID conflicts with a built-in pet: {id}"
            )));
        }
        let destination = models_dir.join(&id);
        let metadata = InstalledPackageMetadata {
            format_version: METADATA_FORMAT_VERSION,
            content_digest: validated.content_digest,
            manifest: validated.manifest,
        };

        if destination.exists() {
            let existing_metadata = read_metadata(&destination)?;
            let InstalledMetadata::Package(existing) = existing_metadata else {
                return Err(ModelError::new(format!(
                    "package ID conflicts with a legacy installed model: {id}"
                )));
            };

            let existing_version = Version::parse(&existing.manifest.version)?;
            let incoming_version = Version::parse(&metadata.manifest.version)?;

            if incoming_version == existing_version {
                if metadata.content_digest == existing.content_digest {
                    return read_installed_model(&destination, &id);
                }
                return Err(ModelError::new(format!(
                    "package {id} v{incoming_version} conflicts with different installed content"
                )));
            }
            if incoming_version < existing_version {
                return Err(ModelError::new(format!(
                    "package downgrade is not allowed: installed v{existing_version}, incoming v{incoming_version}"
                )));
            }
        }

        fs::write(
            staging.join(INTERNAL_METADATA_FILE),
            serde_json::to_vec_pretty(&metadata)?,
        )?;
        replace_with_rollback(&staging, &destination, &models_dir)?;
        read_installed_model(&destination, &id)
    })();

    if staging.exists() {
        let _ = fs::remove_dir_all(&staging);
    }

    install_result
}

fn replace_with_rollback(
    staging: &Path,
    destination: &Path,
    models_dir: &Path,
) -> Result<(), ModelError> {
    if !destination.exists() {
        fs::rename(staging, destination)?;
        return Ok(());
    }

    let file_name = destination
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| ModelError::new("installed package ID is not UTF-8"))?;
    let backup = models_dir.join(format!(
        ".replace-{file_name}-{}-{}",
        std::process::id(),
        unique_timestamp()?
    ));

    fs::rename(destination, &backup)?;
    if let Err(error) = fs::rename(staging, destination) {
        let _ = fs::rename(&backup, destination);
        return Err(ModelError::new(format!(
            "cannot activate package update: {error}"
        )));
    }

    let _ = fs::remove_dir_all(backup);
    Ok(())
}

fn list_models(
    app_data_dir: &Path,
    builtin_models_dir: &Path,
) -> Result<Vec<InstalledModel>, ModelError> {
    let mut models = list_builtin_models(builtin_models_dir)?;
    let builtin_ids = models
        .iter()
        .map(|model| model.id.as_str())
        .collect::<HashSet<_>>();
    let custom_models = list_custom_models(app_data_dir)?;

    if let Some(conflict) = custom_models
        .iter()
        .find(|model| builtin_ids.contains(model.id.as_str()))
    {
        return Err(ModelError::new(format!(
            "installed package ID conflicts with a built-in pet: {}",
            conflict.id
        )));
    }

    models.extend(custom_models);
    Ok(models)
}

fn list_builtin_models(models_dir: &Path) -> Result<Vec<InstalledModel>, ModelError> {
    if !models_dir.exists() {
        return Ok(Vec::new());
    }

    let mut entries = fs::read_dir(models_dir)?.collect::<Result<Vec<_>, _>>()?;
    entries.sort_by_key(|entry| entry.file_name());

    let mut models = Vec::new();
    for entry in entries {
        let id = entry.file_name().to_string_lossy().into_owned();
        let file_type = entry.file_type()?;

        if validate_package_id(&id).is_err() {
            if file_type.is_dir() {
                return Err(ModelError::new(format!(
                    "invalid built-in pet directory ID: {id}"
                )));
            }
            continue;
        }

        models.push(read_builtin_model(&entry.path(), &id)?);
    }

    Ok(models)
}

fn read_builtin_model(path: &Path, expected_id: &str) -> Result<InstalledModel, ModelError> {
    let directory_metadata = fs::symlink_metadata(path)?;
    if directory_metadata.file_type().is_symlink() || !directory_metadata.is_dir() {
        return Err(ModelError::new("built-in pet path is not a real directory"));
    }

    let validated = crate::pet_package::validate_directory(path)?;
    if validated.manifest.id != expected_id {
        return Err(ModelError::new(
            "built-in manifest ID does not match its directory",
        ));
    }

    installed_from_manifest(path, validated.manifest, true)
}

fn list_custom_models(app_data_dir: &Path) -> Result<Vec<InstalledModel>, ModelError> {
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
            is_repository_id(&id).then_some((entry.path(), id))
        })
        .map(|(path, id)| read_installed_model(&path, &id))
        .collect()
}

fn remove_model(model_id: &str, app_data_dir: &Path) -> Result<(), ModelError> {
    if !is_repository_id(model_id) {
        return Err(ModelError::new("invalid installed pet ID"));
    }

    let destination = app_data_dir.join(INSTALL_DIRECTORY).join(model_id);
    let installed = read_installed_model(&destination, model_id)?;
    if installed.is_builtin {
        return Err(ModelError::new("built-in pets cannot be removed"));
    }

    fs::remove_dir_all(destination)?;
    Ok(())
}

fn read_installed_model(path: &Path, expected_id: &str) -> Result<InstalledModel, ModelError> {
    let directory_metadata = fs::symlink_metadata(path)?;
    if directory_metadata.file_type().is_symlink() || !directory_metadata.is_dir() {
        return Err(ModelError::new(
            "installed pet path is not a real directory",
        ));
    }

    match read_metadata(path)? {
        InstalledMetadata::Package(metadata) => {
            if metadata.format_version != METADATA_FORMAT_VERSION {
                return Err(ModelError::new(format!(
                    "unsupported installed metadata version: {}",
                    metadata.format_version
                )));
            }
            if metadata.manifest.id != expected_id {
                return Err(ModelError::new(
                    "installed manifest ID does not match its directory",
                ));
            }

            let validated = validate_installed_directory(path)?;
            if validated.manifest != metadata.manifest
                || validated.content_digest != metadata.content_digest
            {
                return Err(ModelError::new(
                    "installed pet content no longer matches its metadata",
                ));
            }

            installed_from_manifest(path, metadata.manifest, false)
        }
        InstalledMetadata::Legacy(metadata) => {
            read_legacy_installed_model(path, expected_id, metadata)
        }
    }
}

fn installed_from_manifest(
    root: &Path,
    manifest: PetManifest,
    is_builtin: bool,
) -> Result<InstalledModel, ModelError> {
    let entry_path = root.join(&manifest.runtime.entry);
    let model_directory = entry_path
        .parent()
        .ok_or_else(|| ModelError::new("runtime entry has no parent directory"))?;
    let cover_path = root.join(&manifest.presentation.cover);
    let background_path = manifest
        .presentation
        .background
        .as_ref()
        .map(|value| root.join(value));
    let mode = manifest
        .input
        .as_ref()
        .map_or(ModelMode::Standard, |input| input.mode.clone());

    Ok(InstalledModel {
        id: manifest.id,
        version: manifest.version,
        name: manifest.name,
        description: manifest.description,
        authors: manifest.authors,
        license: Some(manifest.license),
        path: model_directory.to_string_lossy().into_owned(),
        entry_path: entry_path.to_string_lossy().into_owned(),
        resource_path: root.join("resources").to_string_lossy().into_owned(),
        cover_path: Some(cover_path.to_string_lossy().into_owned()),
        background_path: background_path.map(|path| path.to_string_lossy().into_owned()),
        mode,
        input: manifest.input,
        actions: manifest.actions,
        is_builtin,
        is_legacy: false,
    })
}

fn read_legacy_installed_model(
    root: &Path,
    expected_id: &str,
    metadata: LegacyInstalledModelMetadata,
) -> Result<InstalledModel, ModelError> {
    if metadata.id != expected_id || !is_legacy_id(&metadata.id) {
        return Err(ModelError::new(
            "legacy installed model metadata has an invalid ID",
        ));
    }

    let relative_model_dir = validate_legacy_relative_path(&metadata.model_directory)?;
    let model_path = root.join(relative_model_dir);
    let model_files = collect_legacy_model_entries(&model_path)?;
    if model_files.len() != 1 {
        return Err(ModelError::new(
            "legacy installed model no longer contains exactly one .model3.json file",
        ));
    }

    let entry_path = model_files[0].clone();
    let name = entry_path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("Legacy model")
        .trim_end_matches(".model3")
        .to_owned();
    let cover = model_path.join("resources/cover.png");
    let background = model_path.join("resources/background.png");

    Ok(InstalledModel {
        id: metadata.id,
        version: "0.0.0-legacy".to_owned(),
        name,
        description: None,
        authors: Vec::new(),
        license: None,
        path: model_path.to_string_lossy().into_owned(),
        entry_path: entry_path.to_string_lossy().into_owned(),
        resource_path: model_path.join("resources").to_string_lossy().into_owned(),
        cover_path: cover.exists().then(|| cover.to_string_lossy().into_owned()),
        background_path: background
            .exists()
            .then(|| background.to_string_lossy().into_owned()),
        mode: metadata.mode,
        input: None,
        actions: legacy_actions(&entry_path)?,
        is_builtin: metadata.is_builtin,
        is_legacy: true,
    })
}

fn read_metadata(path: &Path) -> Result<InstalledMetadata, ModelError> {
    let bytes = fs::read(path.join(INTERNAL_METADATA_FILE))?;
    let value: Value = serde_json::from_slice(&bytes)?;

    if value.get("formatVersion").is_some() {
        Ok(InstalledMetadata::Package(Box::new(
            serde_json::from_value(value)?,
        )))
    } else {
        Ok(InstalledMetadata::Legacy(serde_json::from_value(value)?))
    }
}

fn legacy_actions(entry_path: &Path) -> Result<Vec<PetAction>, ModelError> {
    let model_json: Value = serde_json::from_slice(&fs::read(entry_path)?)?;
    let references = model_json.get("FileReferences").and_then(Value::as_object);
    let mut actions = Vec::new();

    if let Some(motions) = references
        .and_then(|references| references.get("Motions"))
        .and_then(Value::as_object)
    {
        for (group, values) in motions {
            let Some(values) = values.as_array() else {
                continue;
            };
            for index in 0..values.len() {
                let id = if group == "Idle" && index == 0 {
                    "idle".to_owned()
                } else {
                    format!(
                        "legacy-motion-{}-{index}",
                        encode_legacy_action_component(group)
                    )
                };
                actions.push(PetAction::Motion {
                    id,
                    name: format!("{group} {}", index + 1),
                    description: None,
                    motion_group: group.clone(),
                    motion_index: index as u32,
                });
            }
        }
    }

    if let Some(expressions) = references
        .and_then(|references| references.get("Expressions"))
        .and_then(Value::as_array)
    {
        for (index, expression) in expressions.iter().enumerate() {
            let Some(name) = expression.get("Name").and_then(Value::as_str) else {
                continue;
            };
            actions.push(PetAction::Expression {
                id: format!(
                    "legacy-expression-{}-{index}",
                    encode_legacy_action_component(name)
                ),
                name: name.to_owned(),
                description: None,
                expression: name.to_owned(),
            });
        }
    }

    Ok(actions)
}

fn encode_legacy_action_component(value: &str) -> String {
    let encoded = value
        .as_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();

    if encoded.is_empty() {
        "empty".to_owned()
    } else {
        encoded
    }
}

fn collect_legacy_model_entries(root: &Path) -> Result<Vec<PathBuf>, ModelError> {
    let metadata = fs::symlink_metadata(root)?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(ModelError::new("legacy model path is not a real directory"));
    }

    let mut files = fs::read_dir(root)?
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| {
            path.is_file()
                && path
                    .file_name()
                    .and_then(|value| value.to_str())
                    .is_some_and(|value| value.ends_with(".model3.json"))
        })
        .collect::<Vec<_>>();
    files.sort();
    Ok(files)
}

fn validate_legacy_relative_path(value: &str) -> Result<PathBuf, ModelError> {
    let path = Path::new(value);
    for component in path.components() {
        match component {
            Component::Normal(_) | Component::CurDir => {}
            _ => {
                return Err(ModelError::new(
                    "legacy model path contains traversal or is absolute",
                ));
            }
        }
    }
    Ok(path.to_path_buf())
}

fn is_repository_id(value: &str) -> bool {
    is_legacy_id(value) || validate_package_id(value).is_ok()
}

fn is_legacy_id(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|character| character.is_ascii_hexdigit() && !character.is_ascii_uppercase())
}

fn unique_timestamp() -> Result<u128, ModelError> {
    Ok(SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| ModelError::new(error.to_string()))?
        .as_nanos())
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

impl From<semver::Error> for ModelError {
    fn from(error: semver::Error) -> Self {
        Self::new(error.to_string())
    }
}

impl From<crate::pet_package::PackageError> for ModelError {
    fn from(error: crate::pet_package::PackageError) -> Self {
        Self::new(error.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pet_package::{
        PROTOCOL_VERSION, PetPresentation, PetRuntime, PetRuntimeType, RUNTIME_PROFILE_VERSION,
        pack_directory,
    };
    use image::{ColorType, ImageEncoder, codecs::png::PngEncoder};
    use std::sync::atomic::{AtomicUsize, Ordering};

    static TEMP_COUNTER: AtomicUsize = AtomicUsize::new(0);

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new(name: &str) -> Self {
            let counter = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "momopet-repository-test-{}-{counter}-{name}",
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

    fn write_png(path: &Path) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        PngEncoder::new(fs::File::create(path).unwrap())
            .write_image(&[255, 255, 255, 255], 1, 1, ColorType::Rgba8.into())
            .unwrap();
    }

    fn write_package_source(root: &Path, version: &str, description: &str) {
        fs::create_dir_all(root.join("model/motions")).unwrap();
        fs::create_dir_all(root.join("model/textures")).unwrap();
        fs::write(root.join("LICENSE.txt"), "Test license").unwrap();
        fs::write(root.join("model/pet.moc3"), b"moc").unwrap();
        fs::write(root.join("model/motions/idle.motion3.json"), b"{}").unwrap();
        write_png(&root.join("model/textures/texture.png"));
        write_png(&root.join("resources/cover.png"));
        fs::write(
            root.join("model/pet.model3.json"),
            br#"{
                "Version": 3,
                "FileReferences": {
                    "Moc": "pet.moc3",
                    "Textures": ["textures/texture.png"],
                    "Motions": {"Idle": [{"File": "motions/idle.motion3.json"}]}
                }
            }"#,
        )
        .unwrap();

        let manifest = PetManifest {
            protocol_version: PROTOCOL_VERSION,
            id: "com.example.momo".to_owned(),
            version: version.to_owned(),
            name: "Momo".to_owned(),
            description: Some(description.to_owned()),
            authors: vec![PetAuthor {
                name: "Author".to_owned(),
                url: None,
            }],
            license: PetLicense {
                name: "Test License".to_owned(),
                file: "LICENSE.txt".to_owned(),
                url: None,
            },
            runtime: PetRuntime {
                runtime_type: PetRuntimeType::Live2dCubism,
                profile_version: RUNTIME_PROFILE_VERSION,
                entry: "model/pet.model3.json".to_owned(),
            },
            presentation: PetPresentation {
                cover: "resources/cover.png".to_owned(),
                background: None,
            },
            actions: vec![PetAction::Motion {
                id: "idle".to_owned(),
                name: "Idle".to_owned(),
                description: None,
                motion_group: "Idle".to_owned(),
                motion_index: 0,
            }],
            input: None,
            extensions: Default::default(),
        };
        fs::write(
            root.join("manifest.json"),
            serde_json::to_vec_pretty(&manifest).unwrap(),
        )
        .unwrap();
    }

    fn create_package(version: &str, description: &str) -> (TestDirectory, PathBuf) {
        let directory = TestDirectory::new(&format!("package-{version}-{description}"));
        let source = directory.0.join("source");
        let output = directory.0.join("output");
        fs::create_dir_all(&source).unwrap();
        fs::create_dir_all(&output).unwrap();
        write_package_source(&source, version, description);
        let package = output.join("momo.momopet");
        pack_directory(&source, &package).unwrap();
        (directory, package)
    }

    #[test]
    fn imports_upgrades_lists_and_removes_by_stable_id() {
        let app_data = TestDirectory::new("app-data");
        let builtin_models = app_data.0.join("builtins");
        let (_v1_dir, v1) = create_package("1.0.0", "first");
        let first = import_package(&v1, &app_data.0, &builtin_models).unwrap();

        assert_eq!(first.id, "com.example.momo");
        assert_eq!(first.version, "1.0.0");
        assert_eq!(first.actions[0].id(), "idle");
        assert_eq!(
            list_models(&app_data.0, &builtin_models).unwrap(),
            vec![first]
        );

        let (_v2_dir, v2) = create_package("1.1.0", "updated");
        let updated = import_package(&v2, &app_data.0, &builtin_models).unwrap();
        assert_eq!(updated.id, "com.example.momo");
        assert_eq!(updated.version, "1.1.0");
        assert_eq!(updated.description.as_deref(), Some("updated"));
        assert_eq!(
            list_models(&app_data.0, &builtin_models).unwrap(),
            vec![updated.clone()]
        );

        remove_model(&updated.id, &app_data.0).unwrap();
        assert!(
            list_models(&app_data.0, &builtin_models)
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn deduplicates_exact_content_and_rejects_version_conflicts_and_downgrades() {
        let app_data = TestDirectory::new("conflict-app-data");
        let builtin_models = app_data.0.join("builtins");
        let (_first_dir, first_package) = create_package("1.0.0", "first");
        let first = import_package(&first_package, &app_data.0, &builtin_models).unwrap();
        assert_eq!(
            import_package(&first_package, &app_data.0, &builtin_models).unwrap(),
            first
        );

        let (_conflict_dir, conflict) = create_package("1.0.0", "different");
        assert!(
            import_package(&conflict, &app_data.0, &builtin_models)
                .unwrap_err()
                .to_string()
                .contains("conflicts")
        );

        let (_newer_dir, newer) = create_package("2.0.0", "newer");
        import_package(&newer, &app_data.0, &builtin_models).unwrap();
        assert!(
            import_package(&first_package, &app_data.0, &builtin_models)
                .unwrap_err()
                .to_string()
                .contains("downgrade")
        );
    }

    #[test]
    fn discovers_builtins_through_the_same_manifest_and_blocks_id_collisions() {
        let app_data = TestDirectory::new("builtin-app-data");
        let builtin_models = TestDirectory::new("builtin-models");
        let builtin_root = builtin_models.0.join("com.example.momo");
        fs::create_dir(&builtin_root).unwrap();
        write_package_source(&builtin_root, "1.0.0", "built in");

        let models = list_models(&app_data.0, &builtin_models.0).unwrap();
        assert_eq!(models.len(), 1);
        assert!(models[0].is_builtin);
        assert_eq!(
            models[0].resource_path,
            builtin_root.join("resources").to_string_lossy()
        );

        let (_package_dir, package) = create_package("1.1.0", "external collision");
        assert!(
            import_package(&package, &app_data.0, &builtin_models.0)
                .unwrap_err()
                .to_string()
                .contains("built-in pet")
        );
    }

    #[test]
    fn removal_cannot_escape_the_repository() {
        let app_data = TestDirectory::new("remove-app-data");
        let error = remove_model("../outside", &app_data.0).unwrap_err();
        assert!(error.to_string().contains("invalid installed pet ID"));
    }
}
