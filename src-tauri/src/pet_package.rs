use image::{GenericImageView, ImageFormat, ImageReader, Limits};
use semver::Version;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet, HashSet},
    fs,
    io::{self, BufReader, Read, Write},
    path::{Component, Path, PathBuf},
};
use zip::{CompressionMethod, ZipArchive, ZipWriter, write::SimpleFileOptions};

pub const PROTOCOL_VERSION: u32 = 1;
pub const RUNTIME_PROFILE_VERSION: u32 = 1;
pub const MANIFEST_FILE: &str = "manifest.json";
pub const INTERNAL_METADATA_FILE: &str = ".momopet-model.json";
pub const MAX_ARCHIVE_SIZE: u64 = 256 * 1024 * 1024;
pub const MAX_FILE_COUNT: usize = 1024;
pub const MAX_UNCOMPRESSED_SIZE: u64 = 512 * 1024 * 1024;
pub const MAX_FILE_SIZE: u64 = 128 * 1024 * 1024;
pub const MAX_MANIFEST_SIZE: u64 = 256 * 1024;
pub const MAX_PATH_BYTES: usize = 240;
pub const MAX_IMAGE_DIMENSION: u32 = 8192;
pub const MAX_IMAGE_PIXELS: u64 = 33_554_432;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PetManifest {
    pub protocol_version: u32,
    pub id: String,
    pub version: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub authors: Vec<PetAuthor>,
    pub license: PetLicense,
    pub runtime: PetRuntime,
    pub presentation: PetPresentation,
    pub actions: Vec<PetAction>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input: Option<PetInput>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub extensions: BTreeMap<String, Value>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PetAuthor {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PetLicense {
    pub name: String,
    pub file: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PetRuntime {
    #[serde(rename = "type")]
    pub runtime_type: PetRuntimeType,
    pub profile_version: u32,
    pub entry: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum PetRuntimeType {
    Live2dCubism,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PetPresentation {
    pub cover: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background: Option<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "lowercase", deny_unknown_fields)]
pub enum PetAction {
    Motion {
        id: String,
        name: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        description: Option<String>,
        #[serde(rename = "motionGroup")]
        motion_group: String,
        #[serde(rename = "motionIndex")]
        motion_index: u32,
    },
    Expression {
        id: String,
        name: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        description: Option<String>,
        expression: String,
    },
}

impl PetAction {
    pub fn id(&self) -> &str {
        match self {
            Self::Motion { id, .. } | Self::Expression { id, .. } => id,
        }
    }

    pub fn name(&self) -> &str {
        match self {
            Self::Motion { name, .. } | Self::Expression { name, .. } => name,
        }
    }

    fn description(&self) -> Option<&str> {
        match self {
            Self::Motion { description, .. } | Self::Expression { description, .. } => {
                description.as_deref()
            }
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PetInput {
    pub mode: ModelMode,
    pub parameters: InputParameters,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ModelMode {
    #[default]
    Standard,
    Keyboard,
    Gamepad,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InputParameters {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hands: Option<HandParameters>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub mouse_buttons: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pointer: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gamepad: Option<GamepadParameters>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HandParameters {
    pub left: String,
    pub right: String,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GamepadParameters {
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub axes: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub thumb_buttons: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stick_hands: Option<HandParameters>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ValidatedPackage {
    pub manifest: PetManifest,
    pub content_digest: String,
}

pub fn validate_archive(source: &Path) -> Result<ValidatedPackage, PackageError> {
    let temporary = tempfile::tempdir()?;
    extract_and_validate(source, temporary.path())
}

pub fn extract_and_validate(
    source: &Path,
    destination: &Path,
) -> Result<ValidatedPackage, PackageError> {
    validate_archive_source(source)?;

    if destination.exists() {
        let mut entries = fs::read_dir(destination)?;
        if entries.next().transpose()?.is_some() {
            return Err(PackageError::new(
                "package extraction destination must be empty",
            ));
        }
    } else {
        fs::create_dir_all(destination)?;
    }

    let file = fs::File::open(source)?;
    let mut archive = ZipArchive::new(BufReader::new(file))?;
    let archive_files = inspect_archive(&mut archive)?;
    let manifest = read_archive_manifest(&mut archive)?;

    validate_manifest(&manifest, &archive_files)?;
    extract_archive(&mut archive, destination)?;

    let validated = validate_directory(destination)?;
    if validated.manifest != manifest {
        return Err(PackageError::new(
            "manifest changed while the package was being extracted",
        ));
    }

    Ok(validated)
}

pub fn validate_directory(root: &Path) -> Result<ValidatedPackage, PackageError> {
    validate_directory_with_options(root, false)
}

pub fn validate_installed_directory(root: &Path) -> Result<ValidatedPackage, PackageError> {
    validate_directory_with_options(root, true)
}

fn validate_directory_with_options(
    root: &Path,
    allow_internal_metadata: bool,
) -> Result<ValidatedPackage, PackageError> {
    let files = collect_directory_files(root, allow_internal_metadata)?;
    let manifest_path = root.join(MANIFEST_FILE);
    let manifest_metadata = fs::metadata(&manifest_path)
        .map_err(|_| PackageError::new("package is missing root manifest.json"))?;

    if manifest_metadata.len() > MAX_MANIFEST_SIZE {
        return Err(PackageError::new("manifest.json exceeds 256 KiB"));
    }

    let manifest: PetManifest = serde_json::from_slice(&fs::read(&manifest_path)?)?;
    let file_names = files
        .iter()
        .map(|path| relative_portable_path(root, path))
        .collect::<Result<BTreeSet<_>, _>>()?;

    validate_manifest(&manifest, &file_names)?;
    validate_model(root, &manifest, &file_names)?;
    validate_png_files(root, &files)?;

    Ok(ValidatedPackage {
        manifest,
        content_digest: hash_files(root, &files)?,
    })
}

pub fn pack_directory(source: &Path, output: &Path) -> Result<ValidatedPackage, PackageError> {
    if !has_momopet_extension(output) {
        return Err(PackageError::new(
            "package output must use the .momopet extension",
        ));
    }
    if output.exists() {
        return Err(PackageError::new(format!(
            "package output already exists: {}",
            output.display()
        )));
    }

    let source = source.canonicalize().map_err(|error| {
        PackageError::new(format!(
            "cannot read package source '{}': {error}",
            source.display()
        ))
    })?;
    let output_parent = output
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(output_parent)?;
    let output_parent = output_parent.canonicalize()?;

    if output_parent.starts_with(&source) {
        return Err(PackageError::new(
            "package output must be outside the source directory",
        ));
    }

    let source_validation = validate_directory(&source)?;
    let files = collect_directory_files(&source, false)?;
    let temporary_output = tempfile::Builder::new()
        .prefix(&format!(".momopet-pack-{}-", std::process::id()))
        .suffix(".momopet")
        .tempfile_in(output_parent)?;
    let temporary_path = temporary_output.path().to_path_buf();

    (|| {
        let output_file = temporary_output.reopen()?;
        let mut writer = ZipWriter::new(output_file);
        let options = SimpleFileOptions::default()
            .compression_method(CompressionMethod::Deflated)
            .unix_permissions(0o644);

        for path in &files {
            let relative = relative_portable_path(&source, path)?;
            writer.start_file(relative, options)?;
            io::copy(&mut BufReader::new(fs::File::open(path)?), &mut writer)?;
        }

        let output_file = writer.finish()?;
        output_file.sync_all()?;

        let temporary_extract = tempfile::tempdir()?;
        let archive_validation = extract_and_validate(&temporary_path, temporary_extract.path())?;

        if archive_validation != source_validation {
            return Err(PackageError::new(
                "packed archive does not match its source directory",
            ));
        }

        temporary_output
            .persist_noclobber(output)
            .map_err(|error| {
                PackageError::new(format!(
                    "cannot create package output '{}': {}",
                    output.display(),
                    error.error
                ))
            })?;
        Ok::<ValidatedPackage, PackageError>(archive_validation)
    })()
}

fn validate_archive_source(source: &Path) -> Result<(), PackageError> {
    if !has_momopet_extension(source) {
        return Err(PackageError::new(
            "pet packages must use the .momopet extension",
        ));
    }

    let metadata = fs::symlink_metadata(source).map_err(|error| {
        PackageError::new(format!(
            "cannot read pet package '{}': {error}",
            source.display()
        ))
    })?;

    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(PackageError::new(
            "pet package source must be a regular file",
        ));
    }
    if metadata.len() > MAX_ARCHIVE_SIZE {
        return Err(PackageError::new("pet package exceeds 256 MiB"));
    }

    Ok(())
}

fn has_momopet_extension(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("momopet"))
}

fn inspect_archive<R: Read + io::Seek>(
    archive: &mut ZipArchive<R>,
) -> Result<BTreeSet<String>, PackageError> {
    if archive.len() > MAX_FILE_COUNT {
        return Err(PackageError::new("pet package contains too many entries"));
    }

    let mut files = BTreeSet::new();
    let mut case_insensitive_paths = HashSet::new();
    let mut total_size = 0_u64;

    for index in 0..archive.len() {
        let file = archive.by_index(index)?;
        if file.encrypted() {
            return Err(PackageError::new("encrypted ZIP entries are not allowed"));
        }
        if file.is_symlink() {
            return Err(PackageError::new("symbolic links are not allowed"));
        }
        if let Some(mode) = file.unix_mode() {
            let file_type = mode & 0o170000;
            if file_type != 0 && file_type != 0o040000 && file_type != 0o100000 {
                return Err(PackageError::new(
                    "only regular files and directories are allowed",
                ));
            }
        }
        if !matches!(
            file.compression(),
            CompressionMethod::Stored | CompressionMethod::Deflated
        ) {
            return Err(PackageError::new(
                "only Stored and Deflate ZIP compression are allowed",
            ));
        }

        let raw_name = std::str::from_utf8(file.name_raw())
            .map_err(|_| PackageError::new("ZIP entry names must be UTF-8"))?;
        let entry_name = raw_name.strip_suffix('/').unwrap_or(raw_name);
        validate_portable_path(entry_name)?;

        let case_folded = entry_name.to_ascii_lowercase();
        if !case_insensitive_paths.insert(case_folded) {
            return Err(PackageError::new(format!(
                "duplicate or case-colliding ZIP path: {entry_name}"
            )));
        }

        if file.is_dir() {
            continue;
        }
        if !file.is_file() {
            return Err(PackageError::new(
                "only regular files and directories are allowed",
            ));
        }
        if file.size() > MAX_FILE_SIZE {
            return Err(PackageError::new(format!(
                "package file exceeds 128 MiB: {entry_name}"
            )));
        }

        total_size = total_size
            .checked_add(file.size())
            .ok_or_else(|| PackageError::new("package size overflow"))?;
        if total_size > MAX_UNCOMPRESSED_SIZE {
            return Err(PackageError::new(
                "pet package exceeds the 512 MiB uncompressed budget",
            ));
        }

        validate_package_file_name(entry_name)?;
        files.insert(entry_name.to_owned());
    }

    if !files.contains(MANIFEST_FILE) {
        return Err(PackageError::new(
            "pet package is missing root manifest.json",
        ));
    }

    for file in &files {
        let mut parent = Path::new(file).parent();
        while let Some(path) = parent.filter(|path| !path.as_os_str().is_empty()) {
            let parent_name = path_to_portable_string(path)?;
            if files.contains(&parent_name) {
                return Err(PackageError::new(format!(
                    "package path is both a file and a directory: {parent_name}"
                )));
            }
            parent = path.parent();
        }
    }

    Ok(files)
}

fn read_archive_manifest<R: Read + io::Seek>(
    archive: &mut ZipArchive<R>,
) -> Result<PetManifest, PackageError> {
    let file = archive.by_name(MANIFEST_FILE)?;
    let expected_size = file.size();
    if expected_size > MAX_MANIFEST_SIZE {
        return Err(PackageError::new("manifest.json exceeds 256 KiB"));
    }

    let mut bytes = Vec::with_capacity(expected_size as usize);
    file.take(MAX_MANIFEST_SIZE + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 != expected_size || bytes.len() as u64 > MAX_MANIFEST_SIZE {
        return Err(PackageError::new(
            "manifest.json size did not match its ZIP declaration",
        ));
    }
    Ok(serde_json::from_slice(&bytes)?)
}

fn extract_archive<R: Read + io::Seek>(
    archive: &mut ZipArchive<R>,
    destination: &Path,
) -> Result<(), PackageError> {
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index)?;
        let raw_name = std::str::from_utf8(entry.name_raw())
            .map_err(|_| PackageError::new("ZIP entry names must be UTF-8"))?;
        let entry_name = raw_name.strip_suffix('/').unwrap_or(raw_name).to_owned();
        let relative = validate_portable_path(&entry_name)?;
        let output = destination.join(relative);

        if entry.is_dir() {
            fs::create_dir_all(output)?;
            continue;
        }

        if let Some(parent) = output.parent() {
            fs::create_dir_all(parent)?;
        }

        let mut output_file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(output)?;
        let expected = entry.size();
        let copied = io::copy(
            &mut entry.by_ref().take(MAX_FILE_SIZE + 1),
            &mut output_file,
        )?;

        if copied != expected || copied > MAX_FILE_SIZE {
            return Err(PackageError::new(format!(
                "ZIP entry size did not match its declaration: {entry_name}"
            )));
        }
        output_file.flush()?;
    }

    Ok(())
}

fn collect_directory_files(
    root: &Path,
    allow_internal_metadata: bool,
) -> Result<Vec<PathBuf>, PackageError> {
    let metadata = fs::symlink_metadata(root)?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(PackageError::new("package source must be a real directory"));
    }

    let mut files = Vec::new();
    collect_directory_files_from(root, root, allow_internal_metadata, &mut files)?;
    files.sort();

    if files.len() > MAX_FILE_COUNT {
        return Err(PackageError::new("pet package contains too many files"));
    }

    let mut total_size = 0_u64;
    let mut case_insensitive_paths = HashSet::new();
    for path in &files {
        let relative = relative_portable_path(root, path)?;
        if !case_insensitive_paths.insert(relative.to_ascii_lowercase()) {
            return Err(PackageError::new(format!(
                "duplicate or case-colliding package path: {relative}"
            )));
        }

        let size = fs::metadata(path)?.len();
        if size > MAX_FILE_SIZE {
            return Err(PackageError::new(format!(
                "package file exceeds 128 MiB: {relative}"
            )));
        }
        total_size = total_size
            .checked_add(size)
            .ok_or_else(|| PackageError::new("package size overflow"))?;
    }

    if total_size > MAX_UNCOMPRESSED_SIZE {
        return Err(PackageError::new(
            "pet package exceeds the 512 MiB uncompressed budget",
        ));
    }

    Ok(files)
}

fn collect_directory_files_from(
    root: &Path,
    directory: &Path,
    allow_internal_metadata: bool,
    files: &mut Vec<PathBuf>,
) -> Result<(), PackageError> {
    let mut entries = fs::read_dir(directory)?.collect::<Result<Vec<_>, _>>()?;
    entries.sort_by_key(|entry| entry.file_name());

    for entry in entries {
        let path = entry.path();
        let metadata = fs::symlink_metadata(&path)?;

        if metadata.file_type().is_symlink() {
            return Err(PackageError::new(format!(
                "symbolic links are not allowed: {}",
                path.display()
            )));
        }
        if metadata.is_dir() {
            collect_directory_files_from(root, &path, allow_internal_metadata, files)?;
            continue;
        }
        if !metadata.is_file() {
            return Err(PackageError::new(format!(
                "unsupported filesystem entry: {}",
                path.display()
            )));
        }

        if allow_internal_metadata && path == root.join(INTERNAL_METADATA_FILE) {
            continue;
        }
        let relative = relative_portable_path(root, &path)?;
        validate_package_file_name(&relative)?;
        files.push(path);
    }

    Ok(())
}

fn validate_manifest(manifest: &PetManifest, files: &BTreeSet<String>) -> Result<(), PackageError> {
    if manifest.protocol_version != PROTOCOL_VERSION {
        return Err(PackageError::new(format!(
            "unsupported pet package protocol version: {}",
            manifest.protocol_version
        )));
    }
    if manifest.runtime.profile_version != RUNTIME_PROFILE_VERSION {
        return Err(PackageError::new(format!(
            "unsupported Live2D runtime profile version: {}",
            manifest.runtime.profile_version
        )));
    }

    validate_package_id(&manifest.id)?;
    if manifest.version.len() > 64 {
        return Err(PackageError::new("package version exceeds 64 bytes"));
    }
    Version::parse(&manifest.version)
        .map_err(|error| PackageError::new(format!("invalid package version: {error}")))?;
    validate_text("name", &manifest.name, 100)?;
    if let Some(description) = &manifest.description {
        validate_optional_text("description", description, 2048)?;
    }

    if manifest.authors.is_empty() || manifest.authors.len() > 8 {
        return Err(PackageError::new(
            "authors must contain between 1 and 8 entries",
        ));
    }
    for author in &manifest.authors {
        validate_text("author name", &author.name, 100)?;
        if let Some(url) = &author.url {
            validate_url("author URL", url)?;
        }
    }

    validate_text("license name", &manifest.license.name, 100)?;
    if let Some(url) = &manifest.license.url {
        validate_url("license URL", url)?;
    }

    let license_file = validate_manifest_file("license.file", &manifest.license.file, files)?;
    if license_file == MANIFEST_FILE {
        return Err(PackageError::new(
            "license.file must reference a dedicated license file",
        ));
    }

    let entry = validate_manifest_file("runtime.entry", &manifest.runtime.entry, files)?;
    if !entry.ends_with(".model3.json") {
        return Err(PackageError::new(
            "runtime.entry must reference a .model3.json file",
        ));
    }

    let model_entries = files
        .iter()
        .filter(|path| path.ends_with(".model3.json"))
        .count();
    if model_entries != 1 {
        return Err(PackageError::new(format!(
            "V1 packages must contain exactly one .model3.json file, found {model_entries}"
        )));
    }

    let cover = validate_manifest_file("presentation.cover", &manifest.presentation.cover, files)?;
    if !cover.to_ascii_lowercase().ends_with(".png") {
        return Err(PackageError::new("presentation.cover must be a PNG file"));
    }
    if let Some(background) = &manifest.presentation.background {
        let background = validate_manifest_file("presentation.background", background, files)?;
        if !background.to_ascii_lowercase().ends_with(".png") {
            return Err(PackageError::new(
                "presentation.background must be a PNG file",
            ));
        }
    }

    if manifest.actions.is_empty() || manifest.actions.len() > 128 {
        return Err(PackageError::new(
            "actions must contain between 1 and 128 entries",
        ));
    }
    let mut action_ids = HashSet::new();
    let mut has_idle_motion = false;
    for action in &manifest.actions {
        validate_action_id(action.id())?;
        validate_text("action name", action.name(), 100)?;
        if let Some(description) = action.description() {
            validate_optional_text("action description", description, 512)?;
        }
        if !action_ids.insert(action.id()) {
            return Err(PackageError::new(format!(
                "duplicate action ID: {}",
                action.id()
            )));
        }

        match action {
            PetAction::Motion {
                id,
                motion_group,
                motion_index,
                ..
            } => {
                validate_text("motion group", motion_group, 128)?;
                if *motion_index > 65_535 {
                    return Err(PackageError::new("motionIndex exceeds 65535"));
                }
                if id == "idle" {
                    has_idle_motion = true;
                }
            }
            PetAction::Expression { expression, .. } => {
                validate_text("expression", expression, 128)?;
            }
        }
    }
    if !has_idle_motion {
        return Err(PackageError::new(
            "actions must contain a motion action with ID 'idle'",
        ));
    }

    if let Some(input) = &manifest.input {
        validate_input(input)?;
    }
    validate_input_overlay_files(files)?;
    for key in manifest.extensions.keys() {
        validate_package_id(key).map_err(|_| {
            PackageError::new(format!(
                "extension keys must use reverse-domain namespaces: {key}"
            ))
        })?;
    }

    Ok(())
}

fn validate_input_overlay_files(files: &BTreeSet<String>) -> Result<(), PackageError> {
    for file in files {
        for directory in ["resources/left-keys/", "resources/right-keys/"] {
            let Some(relative) = file.strip_prefix(directory) else {
                continue;
            };
            if relative.contains('/') || !relative.to_ascii_lowercase().ends_with(".png") {
                return Err(PackageError::new(format!(
                    "input overlay files must be direct PNG children of {directory}: {file}"
                )));
            }
        }
    }

    Ok(())
}

fn validate_input(input: &PetInput) -> Result<(), PackageError> {
    let parameters = &input.parameters;
    let has_gamepad_mapping = parameters.gamepad.as_ref().is_some_and(|gamepad| {
        !gamepad.axes.is_empty()
            || !gamepad.thumb_buttons.is_empty()
            || gamepad.stick_hands.is_some()
    });
    if parameters.hands.is_none()
        && parameters.mouse_buttons.is_empty()
        && parameters.pointer.is_empty()
        && !has_gamepad_mapping
    {
        return Err(PackageError::new(
            "input.parameters must declare at least one mapping",
        ));
    }
    if input.mode == ModelMode::Gamepad && !has_gamepad_mapping {
        return Err(PackageError::new(
            "gamepad input mode requires gamepad parameter mappings",
        ));
    }

    if let Some(hands) = &parameters.hands {
        validate_parameter_id("left hand parameter", &hands.left)?;
        validate_parameter_id("right hand parameter", &hands.right)?;
    }
    validate_parameter_map("mouse button", &parameters.mouse_buttons)?;
    if parameters.pointer.len() > 32 {
        return Err(PackageError::new(
            "input pointer mappings cannot exceed 32 entries",
        ));
    }
    let mut pointer_parameters = HashSet::new();
    for parameter in &parameters.pointer {
        validate_parameter_id("pointer parameter", parameter)?;
        if !pointer_parameters.insert(parameter) {
            return Err(PackageError::new(format!(
                "duplicate pointer parameter mapping: {parameter}"
            )));
        }
    }
    if let Some(gamepad) = &parameters.gamepad {
        validate_parameter_map("gamepad axis", &gamepad.axes)?;
        validate_parameter_map("gamepad thumb button", &gamepad.thumb_buttons)?;
        if let Some(stick_hands) = &gamepad.stick_hands {
            validate_parameter_id("left stick hand parameter", &stick_hands.left)?;
            validate_parameter_id("right stick hand parameter", &stick_hands.right)?;
        }
    }

    Ok(())
}

fn validate_parameter_map(
    label: &str,
    parameters: &BTreeMap<String, String>,
) -> Result<(), PackageError> {
    if parameters.len() > 32 {
        return Err(PackageError::new(format!(
            "{label} mappings cannot exceed 32 entries"
        )));
    }
    for (input, parameter) in parameters {
        validate_text(&format!("{label} input"), input, 128)?;
        validate_parameter_id(&format!("{label} parameter"), parameter)?;
    }
    Ok(())
}

fn validate_parameter_id(label: &str, value: &str) -> Result<(), PackageError> {
    validate_text(label, value, 128)
}

fn validate_model(
    root: &Path,
    manifest: &PetManifest,
    files: &BTreeSet<String>,
) -> Result<(), PackageError> {
    let entry_path = root.join(validate_portable_path(&manifest.runtime.entry)?);
    let model_json: Value = serde_json::from_slice(&fs::read(&entry_path)?)?;
    let file_references_value = model_json
        .get("FileReferences")
        .ok_or_else(|| PackageError::new("model is missing FileReferences"))?;
    let file_references = file_references_value
        .as_object()
        .ok_or_else(|| PackageError::new("model FileReferences must be an object"))?;
    if file_references.get("Moc").and_then(Value::as_str).is_none() {
        return Err(PackageError::new(
            "model FileReferences.Moc must be a file path",
        ));
    }
    let textures = file_references
        .get("Textures")
        .and_then(Value::as_array)
        .ok_or_else(|| PackageError::new("model Textures must be an array of PNG paths"))?;
    if textures.is_empty() {
        return Err(PackageError::new(
            "model Textures must contain at least one PNG path",
        ));
    }
    for texture in textures {
        let texture = texture
            .as_str()
            .ok_or_else(|| PackageError::new("Textures must contain only file paths"))?;
        if !texture.to_ascii_lowercase().ends_with(".png") {
            return Err(PackageError::new(format!(
                "Live2D texture paths must use PNG files: {texture}"
            )));
        }
    }

    let mut references = Vec::new();
    collect_reference_paths(file_references_value, None, &mut references)?;
    if references.is_empty() {
        return Err(PackageError::new(
            "model does not reference any runtime files",
        ));
    }

    let entry_parent = Path::new(&manifest.runtime.entry)
        .parent()
        .unwrap_or_else(|| Path::new(""));
    for reference in references {
        let relative = validate_portable_path(&reference)?;
        let resolved = path_to_portable_string(&entry_parent.join(relative))?;
        if !files.contains(&resolved) {
            return Err(PackageError::new(format!(
                "referenced model file is missing: {reference}"
            )));
        }
    }

    let motions = file_references.get("Motions").and_then(Value::as_object);
    let expressions = file_references.get("Expressions").and_then(Value::as_array);

    for action in &manifest.actions {
        match action {
            PetAction::Motion {
                id,
                motion_group,
                motion_index,
                ..
            } => {
                let motion = motions
                    .and_then(|groups| groups.get(motion_group))
                    .and_then(Value::as_array)
                    .and_then(|motions| motions.get(*motion_index as usize))
                    .and_then(Value::as_object);
                let Some(motion) = motion else {
                    return Err(PackageError::new(format!(
                        "action '{id}' references missing motion {motion_group}[{motion_index}]"
                    )));
                };
                if motion.get("File").and_then(Value::as_str).is_none() {
                    return Err(PackageError::new(format!(
                        "action '{id}' motion target is missing a File path"
                    )));
                }
            }
            PetAction::Expression { id, expression, .. } => {
                let expression_target = expressions.and_then(|items| {
                    items.iter().find(|item| {
                        item.get("Name").and_then(Value::as_str) == Some(expression.as_str())
                    })
                });
                let Some(expression_target) = expression_target else {
                    return Err(PackageError::new(format!(
                        "action '{id}' references missing expression '{expression}'"
                    )));
                };
                if expression_target
                    .get("File")
                    .and_then(Value::as_str)
                    .is_none()
                {
                    return Err(PackageError::new(format!(
                        "action '{id}' expression target is missing a File path"
                    )));
                }
            }
        }
    }

    Ok(())
}

fn collect_reference_paths(
    value: &Value,
    key: Option<&str>,
    references: &mut Vec<String>,
) -> Result<(), PackageError> {
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
        _ if key.is_some_and(|key| SINGLE_PATH_KEYS.contains(&key)) => {
            let key = key.unwrap_or("unknown");
            return Err(PackageError::new(format!(
                "model {key} reference must be a file path"
            )));
        }
        Value::Array(items) if key == Some("Textures") => {
            for item in items {
                let path = item
                    .as_str()
                    .ok_or_else(|| PackageError::new("Textures must contain only file paths"))?;
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

fn validate_png_files(root: &Path, files: &[PathBuf]) -> Result<(), PackageError> {
    for path in files {
        if !path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| extension.eq_ignore_ascii_case("png"))
        {
            continue;
        }

        let relative = relative_portable_path(root, path)?;
        let reader = ImageReader::open(path)?.with_guessed_format()?;
        if reader.format() != Some(ImageFormat::Png) {
            return Err(PackageError::new(format!(
                "file uses a .png extension but is not PNG: {relative}"
            )));
        }
        let (width, height) = reader.into_dimensions()?;
        if width == 0
            || height == 0
            || width > MAX_IMAGE_DIMENSION
            || height > MAX_IMAGE_DIMENSION
            || u64::from(width) * u64::from(height) > MAX_IMAGE_PIXELS
        {
            return Err(PackageError::new(format!(
                "PNG dimensions exceed the V1 budget: {relative} ({width}x{height})"
            )));
        }

        let mut reader = ImageReader::open(path)?.with_guessed_format()?;
        let mut limits = Limits::default();
        limits.max_image_width = Some(MAX_IMAGE_DIMENSION);
        limits.max_image_height = Some(MAX_IMAGE_DIMENSION);
        limits.max_alloc = Some(MAX_IMAGE_PIXELS * 4);
        reader.limits(limits);
        let image = reader.decode()?;
        if image.dimensions() != (width, height) {
            return Err(PackageError::new(format!(
                "PNG dimensions changed while decoding: {relative}"
            )));
        }
    }

    Ok(())
}

fn validate_manifest_file(
    field: &str,
    value: &str,
    files: &BTreeSet<String>,
) -> Result<String, PackageError> {
    let path = validate_portable_path(value)?;
    let path = path_to_portable_string(&path)?;
    if !files.contains(&path) {
        return Err(PackageError::new(format!(
            "{field} references missing file: {value}"
        )));
    }
    Ok(path)
}

pub fn validate_package_id(value: &str) -> Result<(), PackageError> {
    if value.len() > 128 {
        return Err(PackageError::new("package ID exceeds 128 bytes"));
    }
    let segments = value.split('.').collect::<Vec<_>>();
    if segments.len() < 2
        || segments.iter().any(|segment| {
            segment.is_empty()
                || segment.len() > 63
                || !segment
                    .bytes()
                    .next()
                    .is_some_and(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
                || !segment
                    .bytes()
                    .last()
                    .is_some_and(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
                || !segment
                    .bytes()
                    .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        })
    {
        return Err(PackageError::new(format!(
            "invalid reverse-domain package ID: {value}"
        )));
    }
    Ok(())
}

fn validate_action_id(value: &str) -> Result<(), PackageError> {
    let valid = !value.is_empty()
        && value.len() <= 64
        && value
            .bytes()
            .next()
            .is_some_and(|byte| byte.is_ascii_lowercase())
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-');
    if !valid {
        return Err(PackageError::new(format!("invalid action ID: {value}")));
    }
    Ok(())
}

fn validate_text(label: &str, value: &str, maximum: usize) -> Result<(), PackageError> {
    if value.trim().is_empty() || value.chars().count() > maximum {
        return Err(PackageError::new(format!(
            "{label} must contain between 1 and {maximum} characters"
        )));
    }
    Ok(())
}

fn validate_optional_text(label: &str, value: &str, maximum: usize) -> Result<(), PackageError> {
    if value.chars().count() > maximum {
        return Err(PackageError::new(format!(
            "{label} cannot exceed {maximum} characters"
        )));
    }
    Ok(())
}

fn validate_url(label: &str, value: &str) -> Result<(), PackageError> {
    let has_supported_scheme = ["https://", "http://"].iter().any(|prefix| {
        value
            .strip_prefix(prefix)
            .is_some_and(|rest| !rest.is_empty())
    });
    if value.len() > 2048 || value.chars().any(char::is_whitespace) || !has_supported_scheme {
        return Err(PackageError::new(format!("{label} must be an HTTP(S) URL")));
    }
    Ok(())
}

fn validate_portable_path(value: &str) -> Result<PathBuf, PackageError> {
    if value.is_empty()
        || value.len() > MAX_PATH_BYTES
        || value.contains('\\')
        || value.contains('\0')
        || !value.is_ascii()
    {
        return Err(PackageError::new(format!(
            "invalid portable package path: {value}"
        )));
    }

    let path = Path::new(value);
    let mut has_component = false;
    for component in path.components() {
        let Component::Normal(segment) = component else {
            return Err(PackageError::new(format!(
                "absolute paths and traversal are not allowed: {value}"
            )));
        };
        let segment = segment
            .to_str()
            .ok_or_else(|| PackageError::new("package paths must be UTF-8"))?;
        let valid = !segment.is_empty()
            && !segment.starts_with('.')
            && segment
                .bytes()
                .next()
                .is_some_and(|byte| byte.is_ascii_alphanumeric())
            && segment
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'));
        if !valid {
            return Err(PackageError::new(format!(
                "invalid portable package path: {value}"
            )));
        }
        has_component = true;
    }
    if !has_component || path_to_portable_string(path)? != value {
        return Err(PackageError::new(format!(
            "invalid portable package path: {value}"
        )));
    }

    Ok(path.to_path_buf())
}

fn validate_package_file_name(value: &str) -> Result<(), PackageError> {
    validate_portable_path(value)?;
    let file_name = Path::new(value)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default();
    if file_name == INTERNAL_METADATA_FILE {
        return Err(PackageError::new(format!(
            "{INTERNAL_METADATA_FILE} is reserved for MomoPet"
        )));
    }

    const FORBIDDEN_EXTENSIONS: &[&str] = &[
        "bat", "cjs", "cmd", "com", "dll", "dylib", "exe", "jar", "js", "lua", "mjs", "msi", "ps1",
        "py", "sh", "so", "vbs", "wasm",
    ];
    if Path::new(value)
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            FORBIDDEN_EXTENSIONS.contains(&extension.to_ascii_lowercase().as_str())
        })
    {
        return Err(PackageError::new(format!(
            "executable or script files are not allowed: {value}"
        )));
    }

    Ok(())
}

fn relative_portable_path(root: &Path, path: &Path) -> Result<String, PackageError> {
    let relative = path
        .strip_prefix(root)
        .map_err(|_| PackageError::new("package file escaped its source directory"))?;
    let portable = path_to_portable_string(relative)?;
    validate_portable_path(&portable)?;
    Ok(portable)
}

fn path_to_portable_string(path: &Path) -> Result<String, PackageError> {
    let mut parts = Vec::new();
    for component in path.components() {
        match component {
            Component::Normal(value) => parts.push(
                value
                    .to_str()
                    .ok_or_else(|| PackageError::new("package paths must be UTF-8"))?,
            ),
            Component::CurDir => {}
            _ => {
                return Err(PackageError::new(format!(
                    "absolute paths and traversal are not allowed: {}",
                    path.display()
                )));
            }
        }
    }
    Ok(parts.join("/"))
}

fn hash_files(root: &Path, files: &[PathBuf]) -> Result<String, PackageError> {
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 16 * 1024];

    hasher.update(b"MomoPet package content digest v1\0");
    hasher.update((files.len() as u64).to_le_bytes());
    for path in files {
        let relative = relative_portable_path(root, path)?;
        let file_size = fs::metadata(path)?.len();
        hasher.update((relative.len() as u64).to_le_bytes());
        hasher.update(relative.as_bytes());
        hasher.update(file_size.to_le_bytes());

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

#[derive(Debug)]
pub struct PackageError(String);

impl PackageError {
    pub fn new(message: impl Into<String>) -> Self {
        Self(message.into())
    }
}

impl std::fmt::Display for PackageError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for PackageError {}

impl From<io::Error> for PackageError {
    fn from(error: io::Error) -> Self {
        Self::new(error.to_string())
    }
}

impl From<serde_json::Error> for PackageError {
    fn from(error: serde_json::Error) -> Self {
        Self::new(error.to_string())
    }
}

impl From<zip::result::ZipError> for PackageError {
    fn from(error: zip::result::ZipError) -> Self {
        Self::new(error.to_string())
    }
}

impl From<image::ImageError> for PackageError {
    fn from(error: image::ImageError) -> Self {
        Self::new(error.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ColorType, ImageEncoder, codecs::png::PngEncoder};
    use std::sync::atomic::{AtomicUsize, Ordering};

    static TEMP_COUNTER: AtomicUsize = AtomicUsize::new(0);

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new(name: &str) -> Self {
            let counter = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "momopet-package-test-{}-{counter}-{name}",
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
        let encoder = PngEncoder::new(fs::File::create(path).unwrap());
        encoder
            .write_image(&[255, 255, 255, 255], 1, 1, ColorType::Rgba8.into())
            .unwrap();
    }

    fn valid_manifest(version: &str) -> PetManifest {
        PetManifest {
            protocol_version: PROTOCOL_VERSION,
            id: "com.example.momo".to_owned(),
            version: version.to_owned(),
            name: "Momo".to_owned(),
            description: Some("Test pet".to_owned()),
            authors: vec![PetAuthor {
                name: "Example Author".to_owned(),
                url: Some("https://example.com".to_owned()),
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
            actions: vec![
                PetAction::Motion {
                    id: "idle".to_owned(),
                    name: "Idle".to_owned(),
                    description: None,
                    motion_group: "Idle".to_owned(),
                    motion_index: 0,
                },
                PetAction::Expression {
                    id: "smile".to_owned(),
                    name: "Smile".to_owned(),
                    description: None,
                    expression: "smile".to_owned(),
                },
            ],
            input: None,
            extensions: BTreeMap::new(),
        }
    }

    fn write_valid_source(root: &Path, version: &str) {
        fs::create_dir_all(root.join("model/motions")).unwrap();
        fs::create_dir_all(root.join("model/expressions")).unwrap();
        fs::create_dir_all(root.join("model/textures")).unwrap();
        fs::write(root.join("LICENSE.txt"), "Test redistribution license").unwrap();
        fs::write(root.join("model/pet.moc3"), b"moc").unwrap();
        fs::write(root.join("model/motions/idle.motion3.json"), b"{}").unwrap();
        fs::write(root.join("model/expressions/smile.exp3.json"), b"{}").unwrap();
        write_png(&root.join("model/textures/texture.png"));
        write_png(&root.join("resources/cover.png"));
        fs::write(
            root.join("model/pet.model3.json"),
            br#"{
                "Version": 3,
                "FileReferences": {
                    "Moc": "pet.moc3",
                    "Textures": ["textures/texture.png"],
                    "Motions": {
                        "Idle": [{"File": "motions/idle.motion3.json"}]
                    },
                    "Expressions": [
                        {"Name": "smile", "File": "expressions/smile.exp3.json"}
                    ]
                }
            }"#,
        )
        .unwrap();
        fs::write(
            root.join(MANIFEST_FILE),
            serde_json::to_vec_pretty(&valid_manifest(version)).unwrap(),
        )
        .unwrap();
    }

    #[test]
    fn packs_and_validates_a_v1_package() {
        let source = TestDirectory::new("valid-source");
        let output = TestDirectory::new("valid-output");
        write_valid_source(&source.0, "1.0.0");
        let package_path = output.0.join("momo.momopet");

        let packed = pack_directory(&source.0, &package_path).unwrap();
        let validated = validate_archive(&package_path).unwrap();

        assert_eq!(packed, validated);
        assert_eq!(validated.manifest.id, "com.example.momo");
        assert_eq!(validated.manifest.actions.len(), 2);
    }

    #[test]
    fn rejects_unknown_skin_fields_and_missing_idle() {
        let source = TestDirectory::new("invalid-manifest");
        write_valid_source(&source.0, "1.0.0");
        let mut manifest: Value =
            serde_json::from_slice(&fs::read(source.0.join(MANIFEST_FILE)).unwrap()).unwrap();
        manifest["skins"] = serde_json::json!([]);
        fs::write(
            source.0.join(MANIFEST_FILE),
            serde_json::to_vec_pretty(&manifest).unwrap(),
        )
        .unwrap();
        assert!(
            validate_directory(&source.0)
                .unwrap_err()
                .to_string()
                .contains("unknown field")
        );

        let missing_idle = TestDirectory::new("missing-idle");
        write_valid_source(&missing_idle.0, "1.0.0");
        let mut manifest = valid_manifest("1.0.0");
        manifest.actions.remove(0);
        fs::write(
            missing_idle.0.join(MANIFEST_FILE),
            serde_json::to_vec_pretty(&manifest).unwrap(),
        )
        .unwrap();
        assert!(
            validate_directory(&missing_idle.0)
                .unwrap_err()
                .to_string()
                .contains("ID 'idle'")
        );
    }

    #[test]
    fn rejects_actions_that_do_not_exist_in_the_model() {
        let source = TestDirectory::new("missing-action");
        write_valid_source(&source.0, "1.0.0");
        let mut manifest = valid_manifest("1.0.0");
        manifest.actions.push(PetAction::Motion {
            id: "wave".to_owned(),
            name: "Wave".to_owned(),
            description: None,
            motion_group: "Wave".to_owned(),
            motion_index: 0,
        });
        fs::write(
            source.0.join(MANIFEST_FILE),
            serde_json::to_vec_pretty(&manifest).unwrap(),
        )
        .unwrap();

        let error = validate_directory(&source.0).unwrap_err();
        assert!(error.to_string().contains("missing motion Wave[0]"));
    }

    #[test]
    fn rejects_empty_and_duplicate_input_mappings() {
        let empty_input = TestDirectory::new("empty-input");
        write_valid_source(&empty_input.0, "1.0.0");
        let mut manifest = valid_manifest("1.0.0");
        manifest.input = Some(PetInput {
            mode: ModelMode::Gamepad,
            parameters: InputParameters::default(),
        });
        fs::write(
            empty_input.0.join(MANIFEST_FILE),
            serde_json::to_vec_pretty(&manifest).unwrap(),
        )
        .unwrap();
        assert!(
            validate_directory(&empty_input.0)
                .unwrap_err()
                .to_string()
                .contains("at least one mapping")
        );

        let duplicate_pointer = TestDirectory::new("duplicate-pointer");
        write_valid_source(&duplicate_pointer.0, "1.0.0");
        let mut manifest = valid_manifest("1.0.0");
        manifest.input = Some(PetInput {
            mode: ModelMode::Standard,
            parameters: InputParameters {
                pointer: vec!["ParamAngleX".to_owned(), "ParamAngleX".to_owned()],
                ..Default::default()
            },
        });
        fs::write(
            duplicate_pointer.0.join(MANIFEST_FILE),
            serde_json::to_vec_pretty(&manifest).unwrap(),
        )
        .unwrap();
        assert!(
            validate_directory(&duplicate_pointer.0)
                .unwrap_err()
                .to_string()
                .contains("duplicate pointer")
        );
    }

    #[test]
    fn rejects_malformed_action_targets_and_non_png_overlays() {
        let malformed_motion = TestDirectory::new("malformed-motion");
        write_valid_source(&malformed_motion.0, "1.0.0");
        fs::write(
            malformed_motion.0.join("model/pet.model3.json"),
            br#"{
                "Version": 3,
                "FileReferences": {
                    "Moc": "pet.moc3",
                    "Textures": ["textures/texture.png"],
                    "Motions": {"Idle": [{}]},
                    "Expressions": [
                        {"Name": "smile", "File": "expressions/smile.exp3.json"}
                    ]
                }
            }"#,
        )
        .unwrap();
        assert!(
            validate_directory(&malformed_motion.0)
                .unwrap_err()
                .to_string()
                .contains("missing a File path")
        );

        let invalid_overlay = TestDirectory::new("invalid-overlay");
        write_valid_source(&invalid_overlay.0, "1.0.0");
        fs::create_dir_all(invalid_overlay.0.join("resources/left-keys")).unwrap();
        fs::write(
            invalid_overlay.0.join("resources/left-keys/A.svg"),
            "<svg xmlns=\"http://www.w3.org/2000/svg\"/>",
        )
        .unwrap();
        assert!(
            validate_directory(&invalid_overlay.0)
                .unwrap_err()
                .to_string()
                .contains("direct PNG children")
        );
    }

    #[test]
    fn rejects_archive_path_traversal() {
        let output = TestDirectory::new("traversal");
        let package_path = output.0.join("traversal.momopet");
        let file = fs::File::create(&package_path).unwrap();
        let mut writer = ZipWriter::new(file);
        writer
            .start_file("../manifest.json", SimpleFileOptions::default())
            .unwrap();
        writer.write_all(b"{}").unwrap();
        writer.finish().unwrap();

        let error = validate_archive(&package_path).unwrap_err();
        assert!(error.to_string().contains("traversal"));
    }

    #[test]
    fn checked_in_schema_declares_the_v1_boundaries() {
        let schema: Value =
            serde_json::from_str(include_str!("../../schemas/momopet-package-v1.schema.json"))
                .unwrap();

        assert_eq!(schema["properties"]["protocolVersion"]["const"], 1);
        assert_eq!(schema["additionalProperties"], false);
        assert!(schema["properties"].get("actions").is_some());
        assert!(schema["properties"].get("skins").is_none());
        assert!(schema["properties"].get("variants").is_none());
    }

    #[test]
    fn checked_in_example_matches_the_runtime_manifest_contract() {
        let manifest: PetManifest = serde_json::from_str(include_str!(
            "../../examples/pet-package/manifest.example.json"
        ))
        .unwrap();

        assert_eq!(manifest.protocol_version, PROTOCOL_VERSION);
        assert_eq!(manifest.runtime.profile_version, RUNTIME_PROFILE_VERSION);
        assert_eq!(manifest.actions[1].id(), "wave");
        assert!(manifest.input.is_some());
    }

    #[test]
    fn content_digest_preserves_file_boundaries() {
        let single_file = TestDirectory::new("single-file-digest");
        let split_files = TestDirectory::new("split-file-digest");
        let mut combined = b"left".to_vec();
        combined.extend_from_slice(&("b.bin".len() as u64).to_le_bytes());
        combined.extend_from_slice(b"b.bin");
        combined.extend_from_slice(b"right");

        fs::write(single_file.0.join("a.bin"), combined).unwrap();
        fs::write(split_files.0.join("a.bin"), b"left").unwrap();
        fs::write(split_files.0.join("b.bin"), b"right").unwrap();

        let single_paths = vec![single_file.0.join("a.bin")];
        let split_paths = vec![split_files.0.join("a.bin"), split_files.0.join("b.bin")];

        assert_ne!(
            hash_files(&single_file.0, &single_paths).unwrap(),
            hash_files(&split_files.0, &split_paths).unwrap()
        );
    }
}
