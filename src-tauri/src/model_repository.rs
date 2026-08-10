use crate::pet_package::{
    INTERNAL_METADATA_FILE, PetAction, PetAuthor, PetLicense, PetManifest, PetRuntimeType,
    extract_and_validate, validate_installed_directory, validate_package_id,
};
use semver::Version;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    fs,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};
use tauri::{AppHandle, Manager, Runtime, command};

const INSTALL_DIRECTORY: &str = "pets";
const LEGACY_INSTALL_DIRECTORY: &str = "custom-models";
const APP_IDENTIFIER: &str = "com.bytes4096.momopet";
const LEGACY_APP_IDENTIFIER: &str = "com.4096bytes.momopet.live2d";
const BUILTIN_DIRECTORY: &str = "assets/models";
const METADATA_FORMAT_VERSION: u32 = 2;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstalledModel {
    pub id: String,
    pub version: String,
    pub name: String,
    pub description: Option<String>,
    pub authors: Vec<PetAuthor>,
    pub license: Option<PetLicense>,
    pub runtime_type: PetRuntimeType,
    pub path: String,
    pub entry_path: String,
    pub cover_path: Option<String>,
    pub background_path: Option<String>,
    pub actions: Vec<PetAction>,
    pub is_builtin: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct InstalledPackageMetadata {
    format_version: u32,
    content_digest: String,
    manifest: PetManifest,
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
    migrate_legacy_repository(app_data_dir)?;
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
            let existing_metadata = read_package_metadata(&destination)?;

            let existing_version = Version::parse(&existing_metadata.manifest.version)?;
            let incoming_version = Version::parse(&metadata.manifest.version)?;

            if incoming_version == existing_version {
                if metadata.content_digest == existing_metadata.content_digest {
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
    migrate_legacy_repository(app_data_dir)?;
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
    migrate_legacy_repository(app_data_dir)?;
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

fn migrate_legacy_repository(app_data_dir: &Path) -> Result<(), ModelError> {
    remove_legacy_directory(&app_data_dir.join(LEGACY_INSTALL_DIRECTORY))?;

    if app_data_dir.file_name().and_then(|value| value.to_str()) == Some(APP_IDENTIFIER)
        && let Some(app_data_parent) = app_data_dir.parent()
    {
        let previous_app_data = app_data_parent.join(LEGACY_APP_IDENTIFIER);
        if previous_app_data != app_data_dir {
            remove_previous_legacy_repository(&previous_app_data)?;
        }
    }

    Ok(())
}

fn remove_previous_legacy_repository(previous_app_data: &Path) -> Result<(), ModelError> {
    let metadata = match fs::symlink_metadata(previous_app_data) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error.into()),
    };
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(ModelError::new(
            "legacy app data path is not a real directory",
        ));
    }

    remove_legacy_directory(&previous_app_data.join(LEGACY_INSTALL_DIRECTORY))
}

fn remove_legacy_directory(legacy: &Path) -> Result<(), ModelError> {
    let metadata = match fs::symlink_metadata(legacy) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error.into()),
    };
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(ModelError::new(
            "legacy custom-models path is not a real directory",
        ));
    }

    fs::remove_dir_all(legacy)?;
    Ok(())
}

fn read_installed_model(path: &Path, expected_id: &str) -> Result<InstalledModel, ModelError> {
    let directory_metadata = fs::symlink_metadata(path)?;
    if directory_metadata.file_type().is_symlink() || !directory_metadata.is_dir() {
        return Err(ModelError::new(
            "installed pet path is not a real directory",
        ));
    }

    let metadata = read_package_metadata(path)?;
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

fn read_package_metadata(path: &Path) -> Result<InstalledPackageMetadata, ModelError> {
    let bytes = fs::read(path.join(INTERNAL_METADATA_FILE))?;
    Ok(serde_json::from_slice(&bytes)?)
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

    Ok(InstalledModel {
        id: manifest.id,
        version: manifest.version,
        name: manifest.name,
        description: manifest.description,
        authors: manifest.authors,
        license: Some(manifest.license),
        runtime_type: manifest.runtime.runtime_type,
        path: path_for_frontend(model_directory),
        entry_path: path_for_frontend(&entry_path),
        cover_path: Some(path_for_frontend(&cover_path)),
        background_path: background_path.map(|path| path_for_frontend(&path)),
        actions: manifest.actions,
        is_builtin,
    })
}

fn path_for_frontend(path: &Path) -> String {
    normalize_windows_verbatim_path(&path.to_string_lossy())
}

fn normalize_windows_verbatim_path(value: &str) -> String {
    if let Some(path) = value.strip_prefix(r"\\?\UNC\") {
        return format!(r"\\{path}");
    }
    if let Some(path) = value.strip_prefix(r"\\?\") {
        return path.to_owned();
    }
    value.to_owned()
}

fn is_repository_id(value: &str) -> bool {
    validate_package_id(value).is_ok()
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
        PROTOCOL_VERSION, PetActionMode, PetPresentation, PetRuntime, PetRuntimeType,
        RUNTIME_PROFILE_VERSION, pack_directory,
    };
    use image::{Rgba, RgbaImage};
    use std::{
        path::PathBuf,
        sync::atomic::{AtomicUsize, Ordering},
    };

    static TEMP_COUNTER: AtomicUsize = AtomicUsize::new(0);

    #[test]
    fn frontend_path_removes_windows_verbatim_disk_prefix() {
        assert_eq!(
            normalize_windows_verbatim_path(r"\\?\C:\Program Files\MomoPet\model"),
            r"C:\Program Files\MomoPet\model"
        );
    }

    #[test]
    fn frontend_path_converts_windows_verbatim_unc_prefix() {
        assert_eq!(
            normalize_windows_verbatim_path(r"\\?\UNC\server\share\model"),
            r"\\server\share\model"
        );
    }

    #[test]
    fn frontend_path_preserves_regular_paths() {
        assert_eq!(
            normalize_windows_verbatim_path("/opt/momopet/model"),
            "/opt/momopet/model"
        );
    }

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

    fn write_sprite_png(path: &Path) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        let mut image = RgbaImage::new(8, 4);
        image.put_pixel(1, 1, Rgba([255, 128, 32, 255]));
        image.put_pixel(5, 1, Rgba([255, 128, 32, 255]));
        image.save(path).unwrap();
    }

    fn write_cover(path: &Path) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        RgbaImage::from_pixel(4, 3, Rgba([255, 128, 32, 255]))
            .save(path)
            .unwrap();
    }

    fn write_package_source(root: &Path, version: &str, description: &str) {
        fs::create_dir_all(root.join("model/sprites")).unwrap();
        fs::write(root.join("LICENSE.txt"), "Test license").unwrap();
        write_sprite_png(&root.join("model/sprites/pet.png"));
        write_cover(&root.join("resources/cover.png"));
        fs::write(
            root.join("model/pet.sprite.json"),
            br#"{
                "frameSize": {"width": 4, "height": 4},
                "sheets": {"pet": "sprites/pet.png"},
                "clips": {
                    "idle": {"sheet": "pet", "frames": [0], "fps": 8, "loop": true},
                    "happy": {"sheet": "pet", "frames": [1], "fps": 8, "loop": false}
                },
                "interactions": {"tap": "happy"}
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
                runtime_type: PetRuntimeType::Sprite2d,
                profile_version: RUNTIME_PROFILE_VERSION,
                entry: "model/pet.sprite.json".to_owned(),
            },
            presentation: PetPresentation {
                cover: "resources/cover.png".to_owned(),
                background: None,
            },
            actions: vec![PetAction::Animation {
                id: "happy".to_owned(),
                name: "Happy".to_owned(),
                description: None,
                clip: "happy".to_owned(),
                mode: PetActionMode::Once,
            }],
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
        assert_eq!(first.actions[0].id(), "happy");
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
        assert_eq!(models[0].runtime_type, PetRuntimeType::Sprite2d);

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

    #[test]
    fn removes_the_legacy_repository_without_touching_siblings() {
        let app_data = TestDirectory::new("legacy-migration");
        fs::create_dir(app_data.0.join(LEGACY_INSTALL_DIRECTORY)).unwrap();
        fs::write(
            app_data.0.join(LEGACY_INSTALL_DIRECTORY).join("old-model"),
            "legacy",
        )
        .unwrap();
        fs::create_dir(app_data.0.join("keep-me")).unwrap();

        migrate_legacy_repository(&app_data.0).unwrap();

        assert!(!app_data.0.join(LEGACY_INSTALL_DIRECTORY).exists());
        assert!(app_data.0.join("keep-me").exists());
    }

    #[test]
    fn removes_only_custom_models_from_the_previous_app_identifier() {
        let data_root = TestDirectory::new("previous-app-migration");
        let app_data = data_root.0.join("com.bytes4096.momopet");
        let previous_app_data = data_root.0.join(LEGACY_APP_IDENTIFIER);
        fs::create_dir_all(previous_app_data.join(LEGACY_INSTALL_DIRECTORY)).unwrap();
        fs::write(
            previous_app_data
                .join(LEGACY_INSTALL_DIRECTORY)
                .join("old-model"),
            "legacy",
        )
        .unwrap();
        fs::write(previous_app_data.join("keep.txt"), "keep").unwrap();

        migrate_legacy_repository(&app_data).unwrap();

        assert!(!previous_app_data.join(LEGACY_INSTALL_DIRECTORY).exists());
        assert_eq!(
            fs::read_to_string(previous_app_data.join("keep.txt")).unwrap(),
            "keep"
        );
    }

    #[cfg(unix)]
    #[test]
    fn refuses_a_legacy_repository_symlink_without_touching_its_target() {
        use std::os::unix::fs::symlink;

        let app_data = TestDirectory::new("legacy-symlink");
        let target = app_data.0.join("keep-target");
        fs::create_dir(&target).unwrap();
        fs::write(target.join("keep.txt"), "keep").unwrap();
        symlink(&target, app_data.0.join(LEGACY_INSTALL_DIRECTORY)).unwrap();

        let error = migrate_legacy_repository(&app_data.0).unwrap_err();

        assert!(error.to_string().contains("not a real directory"));
        assert_eq!(fs::read_to_string(target.join("keep.txt")).unwrap(), "keep");
    }

    #[cfg(unix)]
    #[test]
    fn refuses_a_previous_app_data_symlink_without_touching_its_target() {
        use std::os::unix::fs::symlink;

        let data_root = TestDirectory::new("previous-app-symlink");
        let app_data = data_root.0.join(APP_IDENTIFIER);
        let target = data_root.0.join("keep-target");
        fs::create_dir_all(target.join(LEGACY_INSTALL_DIRECTORY)).unwrap();
        fs::write(
            target.join(LEGACY_INSTALL_DIRECTORY).join("keep.txt"),
            "keep",
        )
        .unwrap();
        symlink(&target, data_root.0.join(LEGACY_APP_IDENTIFIER)).unwrap();

        let error = migrate_legacy_repository(&app_data).unwrap_err();

        assert!(error.to_string().contains("not a real directory"));
        assert_eq!(
            fs::read_to_string(target.join(LEGACY_INSTALL_DIRECTORY).join("keep.txt")).unwrap(),
            "keep"
        );
    }
}
