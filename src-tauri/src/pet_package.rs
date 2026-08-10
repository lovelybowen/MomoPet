use image::{GenericImageView, ImageFormat, ImageReader, Limits, RgbaImage, imageops};
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
pub const MAX_SPRITE_SHEETS: usize = 32;
pub const MAX_SPRITE_CLIPS: usize = 128;
pub const MAX_CLIP_FRAMES: usize = 512;

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
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub actions: Vec<PetAction>,
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
    Sprite2d,
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
    Animation {
        id: String,
        name: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        description: Option<String>,
        clip: String,
        mode: PetActionMode,
    },
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum PetActionMode {
    Once,
    Toggle,
}

impl PetAction {
    pub fn id(&self) -> &str {
        match self {
            Self::Animation { id, .. } => id,
        }
    }

    pub fn name(&self) -> &str {
        match self {
            Self::Animation { name, .. } => name,
        }
    }

    fn description(&self) -> Option<&str> {
        match self {
            Self::Animation { description, .. } => description.as_deref(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SpriteConfig {
    pub frame_size: SpriteSize,
    #[serde(default = "default_sprite_anchor")]
    pub anchor: SpriteAnchor,
    pub sheets: BTreeMap<String, String>,
    pub clips: BTreeMap<String, SpriteClip>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub interactions: BTreeMap<String, String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SpriteSize {
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SpriteAnchor {
    pub x: f32,
    pub y: f32,
}

fn default_sprite_anchor() -> SpriteAnchor {
    SpriteAnchor { x: 0.5, y: 1.0 }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SpriteClip {
    pub sheet: String,
    pub frames: Vec<u32>,
    pub fps: u32,
    pub r#loop: bool,
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
    validate_sprite(root, &manifest, &file_names)?;
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

#[derive(Clone, Debug, PartialEq)]
pub struct PackedSpriteSheet {
    pub frame_width: u32,
    pub frame_height: u32,
    pub frame_count: usize,
    pub columns: u32,
    pub rows: u32,
}

pub fn pack_sprite_sheet(
    source: &Path,
    output: &Path,
    columns: u32,
) -> Result<PackedSpriteSheet, PackageError> {
    if columns == 0 || columns > 64 {
        return Err(PackageError::new(
            "sprite sheet columns must be between 1 and 64",
        ));
    }
    if !output
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("png"))
    {
        return Err(PackageError::new("sprite sheet output must be a PNG file"));
    }
    if output.exists() {
        return Err(PackageError::new(format!(
            "sprite sheet output already exists: {}",
            output.display()
        )));
    }

    let source = source.canonicalize().map_err(|error| {
        PackageError::new(format!(
            "cannot read sprite frame directory '{}': {error}",
            source.display()
        ))
    })?;
    if !fs::metadata(&source)?.is_dir() {
        return Err(PackageError::new("sprite frame source must be a directory"));
    }

    let output_parent = output
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(output_parent)?;
    let output_parent = output_parent.canonicalize()?;
    if output_parent.starts_with(&source) {
        return Err(PackageError::new(
            "sprite sheet output must be outside the frame directory",
        ));
    }

    let mut entries = fs::read_dir(&source)?.collect::<Result<Vec<_>, _>>()?;
    entries.sort_by_key(|entry| entry.file_name());
    let mut frames = Vec::new();
    for entry in entries {
        let metadata = fs::symlink_metadata(entry.path())?;
        if metadata.file_type().is_symlink() {
            return Err(PackageError::new("symbolic links are not allowed"));
        }
        if !metadata.is_file()
            || !entry
                .path()
                .extension()
                .and_then(|extension| extension.to_str())
                .is_some_and(|extension| extension.eq_ignore_ascii_case("png"))
        {
            continue;
        }
        let image = ImageReader::open(entry.path())?
            .with_guessed_format()?
            .decode()?;
        if !image.color().has_alpha() {
            return Err(PackageError::new(format!(
                "sprite frame must have an alpha channel: {}",
                entry.path().display()
            )));
        }
        frames.push((entry.path(), image.to_rgba8()));
    }
    if frames.is_empty() {
        return Err(PackageError::new(
            "sprite frame directory does not contain any PNG files",
        ));
    }
    if frames.len() > MAX_CLIP_FRAMES {
        return Err(PackageError::new(format!(
            "sprite frame directory cannot contain more than {MAX_CLIP_FRAMES} PNG files"
        )));
    }

    let frame_width = frames[0].1.width();
    let frame_height = frames[0].1.height();
    for (path, frame) in &frames {
        if frame.dimensions() != (frame_width, frame_height) {
            return Err(PackageError::new(format!(
                "sprite frames must use identical dimensions: {}",
                path.display()
            )));
        }
    }

    let frame_count = frames.len();
    let columns = columns.min(frame_count as u32);
    let rows = (frame_count as u32).div_ceil(columns);
    let width = frame_width
        .checked_mul(columns)
        .ok_or_else(|| PackageError::new("sprite sheet width overflow"))?;
    let height = frame_height
        .checked_mul(rows)
        .ok_or_else(|| PackageError::new("sprite sheet height overflow"))?;
    if width > MAX_IMAGE_DIMENSION
        || height > MAX_IMAGE_DIMENSION
        || u64::from(width) * u64::from(height) > MAX_IMAGE_PIXELS
    {
        return Err(PackageError::new(
            "packed sprite sheet exceeds the image budget",
        ));
    }

    let mut sheet = RgbaImage::new(width, height);
    for (index, (_, frame)) in frames.iter().enumerate() {
        let x = (index as u32 % columns) * frame_width;
        let y = (index as u32 / columns) * frame_height;
        imageops::overlay(&mut sheet, frame, i64::from(x), i64::from(y));
    }
    sheet.save_with_format(output, ImageFormat::Png)?;

    Ok(PackedSpriteSheet {
        frame_width,
        frame_height,
        frame_count,
        columns,
        rows,
    })
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
            "unsupported Sprite2D runtime profile version: {}",
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
    if !entry.ends_with(".sprite.json") {
        return Err(PackageError::new(
            "runtime.entry must reference a .sprite.json file",
        ));
    }

    let sprite_entries = files
        .iter()
        .filter(|path| path.ends_with(".sprite.json"))
        .count();
    if sprite_entries != 1 {
        return Err(PackageError::new(format!(
            "V1 packages must contain exactly one .sprite.json file, found {sprite_entries}"
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

    if manifest.actions.len() > 128 {
        return Err(PackageError::new(
            "actions cannot contain more than 128 entries",
        ));
    }
    let mut action_ids = HashSet::new();
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

        let PetAction::Animation { clip, .. } = action;
        validate_action_id(clip).map_err(|_| {
            PackageError::new(format!("action '{}' has an invalid clip ID", action.id()))
        })?;
    }

    for key in manifest.extensions.keys() {
        validate_package_id(key).map_err(|_| {
            PackageError::new(format!(
                "extension keys must use reverse-domain namespaces: {key}"
            ))
        })?;
    }

    Ok(())
}

fn validate_sprite(
    root: &Path,
    manifest: &PetManifest,
    files: &BTreeSet<String>,
) -> Result<(), PackageError> {
    let entry_relative = validate_portable_path(&manifest.runtime.entry)?;
    let entry_path = root.join(entry_relative);
    let config: SpriteConfig = serde_json::from_slice(&fs::read(&entry_path)?)?;

    let frame_width = config.frame_size.width;
    let frame_height = config.frame_size.height;
    if frame_width == 0
        || frame_height == 0
        || frame_width > MAX_IMAGE_DIMENSION
        || frame_height > MAX_IMAGE_DIMENSION
        || u64::from(frame_width) * u64::from(frame_height) > MAX_IMAGE_PIXELS
    {
        return Err(PackageError::new(
            "sprite frameSize exceeds the image budget",
        ));
    }
    if !config.anchor.x.is_finite()
        || !config.anchor.y.is_finite()
        || !(0.0..=1.0).contains(&config.anchor.x)
        || !(0.0..=1.0).contains(&config.anchor.y)
    {
        return Err(PackageError::new(
            "sprite anchor coordinates must be finite values between 0 and 1",
        ));
    }
    if config.sheets.is_empty() || config.sheets.len() > MAX_SPRITE_SHEETS {
        return Err(PackageError::new(format!(
            "sprite sheets must contain between 1 and {MAX_SPRITE_SHEETS} entries"
        )));
    }
    if config.clips.is_empty() || config.clips.len() > MAX_SPRITE_CLIPS {
        return Err(PackageError::new(format!(
            "sprite clips must contain between 1 and {MAX_SPRITE_CLIPS} entries"
        )));
    }

    let entry_parent = Path::new(&manifest.runtime.entry)
        .parent()
        .unwrap_or_else(|| Path::new(""));
    let mut sheet_images = BTreeMap::<String, RgbaImage>::new();
    let mut sheet_frame_counts = BTreeMap::<String, u32>::new();

    for (sheet_id, file) in &config.sheets {
        validate_action_id(sheet_id)
            .map_err(|_| PackageError::new(format!("invalid sprite sheet ID: {sheet_id}")))?;
        if !file.to_ascii_lowercase().ends_with(".png") {
            return Err(PackageError::new(format!(
                "sprite sheet '{sheet_id}' must reference a PNG file"
            )));
        }

        let relative = validate_portable_path(file)?;
        let resolved = path_to_portable_string(&entry_parent.join(relative))?;
        if !files.contains(&resolved) {
            return Err(PackageError::new(format!(
                "referenced sprite sheet is missing: {file}"
            )));
        }

        let reader = ImageReader::open(root.join(&resolved))?.with_guessed_format()?;
        if reader.format() != Some(ImageFormat::Png) {
            return Err(PackageError::new(format!(
                "sprite sheet is not a PNG file: {file}"
            )));
        }
        let image = reader.decode()?;
        if !image.color().has_alpha() {
            return Err(PackageError::new(format!(
                "sprite sheet must have an alpha channel: {file}"
            )));
        }
        let (width, height) = image.dimensions();
        if width % frame_width != 0 || height % frame_height != 0 {
            return Err(PackageError::new(format!(
                "sprite sheet dimensions must be divisible by frameSize: {file} ({width}x{height})"
            )));
        }
        let frame_count = (width / frame_width)
            .checked_mul(height / frame_height)
            .ok_or_else(|| PackageError::new("sprite sheet frame count overflow"))?;
        if frame_count == 0 {
            return Err(PackageError::new(format!(
                "sprite sheet does not contain any frames: {file}"
            )));
        }
        sheet_frame_counts.insert(sheet_id.clone(), frame_count);
        sheet_images.insert(sheet_id.clone(), image.to_rgba8());
    }

    let Some(idle) = config.clips.get("idle") else {
        return Err(PackageError::new("sprite clips must define 'idle'"));
    };
    if !idle.r#loop {
        return Err(PackageError::new("the 'idle' sprite clip must loop"));
    }

    let mut referenced_frames = BTreeMap::<String, BTreeSet<u32>>::new();
    for (clip_id, clip) in &config.clips {
        validate_action_id(clip_id)
            .map_err(|_| PackageError::new(format!("invalid sprite clip ID: {clip_id}")))?;
        let Some(frame_count) = sheet_frame_counts.get(&clip.sheet) else {
            return Err(PackageError::new(format!(
                "sprite clip '{clip_id}' references missing sheet '{}'",
                clip.sheet
            )));
        };
        if clip.frames.is_empty() || clip.frames.len() > MAX_CLIP_FRAMES {
            return Err(PackageError::new(format!(
                "sprite clip '{clip_id}' must contain between 1 and {MAX_CLIP_FRAMES} frames"
            )));
        }
        if !(1..=60).contains(&clip.fps) {
            return Err(PackageError::new(format!(
                "sprite clip '{clip_id}' fps must be between 1 and 60"
            )));
        }
        for frame in &clip.frames {
            if frame >= frame_count {
                return Err(PackageError::new(format!(
                    "sprite clip '{clip_id}' references out-of-range frame {frame}"
                )));
            }
            referenced_frames
                .entry(clip.sheet.clone())
                .or_default()
                .insert(*frame);
        }
    }

    let action_ids = manifest
        .actions
        .iter()
        .map(PetAction::id)
        .collect::<HashSet<_>>();
    for action in &manifest.actions {
        let PetAction::Animation { clip, mode, .. } = action;
        let Some(target) = config.clips.get(clip) else {
            return Err(PackageError::new(format!(
                "action '{}' references missing sprite clip '{clip}'",
                action.id()
            )));
        };
        if matches!(mode, PetActionMode::Toggle) && !target.r#loop {
            return Err(PackageError::new(format!(
                "toggle action '{}' must reference a looping clip",
                action.id()
            )));
        }
    }

    for (event, action_id) in &config.interactions {
        if event != "tap" {
            return Err(PackageError::new(format!(
                "unsupported sprite interaction event: {event}"
            )));
        }
        if !action_ids.contains(action_id.as_str()) {
            return Err(PackageError::new(format!(
                "sprite interaction '{event}' references missing action '{action_id}'"
            )));
        }
    }

    for (sheet_id, frames) in referenced_frames {
        let image = &sheet_images[&sheet_id];
        let columns = image.width() / frame_width;
        for frame in frames {
            let x = (frame % columns) * frame_width;
            let y = (frame / columns) * frame_height;
            let corners = [
                image.get_pixel(x, y),
                image.get_pixel(x + frame_width - 1, y),
                image.get_pixel(x, y + frame_height - 1),
                image.get_pixel(x + frame_width - 1, y + frame_height - 1),
            ];
            if corners.iter().any(|pixel| pixel[3] != 0) {
                return Err(PackageError::new(format!(
                    "sprite sheet '{sheet_id}' frame {frame} must have transparent corners"
                )));
            }
            let frame_image = imageops::crop_imm(image, x, y, frame_width, frame_height).to_image();
            if !frame_image.pixels().any(|pixel| pixel[3] != 0) {
                return Err(PackageError::new(format!(
                    "sprite sheet '{sheet_id}' frame {frame} is empty"
                )));
            }
        }
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
    use image::{Rgba, RgbaImage};
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

    fn write_sprite_png(path: &Path, frames: u32) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        let mut image = RgbaImage::new(4 * frames, 4);
        for frame in 0..frames {
            image.put_pixel(frame * 4 + 1, 1, Rgba([255, 128, 32, 255]));
            image.put_pixel(frame * 4 + 2, 2, Rgba([255, 128, 32, 255]));
        }
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
            extensions: BTreeMap::new(),
        }
    }

    fn write_valid_source(root: &Path, version: &str) {
        fs::create_dir_all(root.join("model/sprites")).unwrap();
        fs::write(root.join("LICENSE.txt"), "Test redistribution license").unwrap();
        write_sprite_png(&root.join("model/sprites/pet.png"), 4);
        write_cover(&root.join("resources/cover.png"));
        fs::write(
            root.join("model/pet.sprite.json"),
            br#"{
                "frameSize": {"width": 4, "height": 4},
                "anchor": {"x": 0.5, "y": 1.0},
                "sheets": {"pet": "sprites/pet.png"},
                "clips": {
                    "idle": {"sheet": "pet", "frames": [0, 1], "fps": 8, "loop": true},
                    "happy": {"sheet": "pet", "frames": [2, 3], "fps": 10, "loop": false}
                },
                "interactions": {"tap": "happy"}
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
        assert_eq!(validated.manifest.actions.len(), 1);
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
        let mut config: Value = serde_json::from_slice(
            &fs::read(missing_idle.0.join("model/pet.sprite.json")).unwrap(),
        )
        .unwrap();
        config["clips"].as_object_mut().unwrap().remove("idle");
        fs::write(
            missing_idle.0.join("model/pet.sprite.json"),
            serde_json::to_vec_pretty(&config).unwrap(),
        )
        .unwrap();
        assert!(
            validate_directory(&missing_idle.0)
                .unwrap_err()
                .to_string()
                .contains("define 'idle'")
        );
    }

    #[test]
    fn rejects_actions_that_do_not_reference_a_clip() {
        let source = TestDirectory::new("missing-action");
        write_valid_source(&source.0, "1.0.0");
        let mut manifest = valid_manifest("1.0.0");
        manifest.actions.push(PetAction::Animation {
            id: "wave".to_owned(),
            name: "Wave".to_owned(),
            description: None,
            clip: "wave".to_owned(),
            mode: PetActionMode::Once,
        });
        fs::write(
            source.0.join(MANIFEST_FILE),
            serde_json::to_vec_pretty(&manifest).unwrap(),
        )
        .unwrap();

        let error = validate_directory(&source.0).unwrap_err();
        assert!(error.to_string().contains("missing sprite clip 'wave'"));
    }

    #[test]
    fn rejects_non_looping_idle_and_out_of_range_frames() {
        let invalid_idle = TestDirectory::new("invalid-idle");
        write_valid_source(&invalid_idle.0, "1.0.0");
        let mut config: Value = serde_json::from_slice(
            &fs::read(invalid_idle.0.join("model/pet.sprite.json")).unwrap(),
        )
        .unwrap();
        config["clips"]["idle"]["loop"] = Value::Bool(false);
        fs::write(
            invalid_idle.0.join("model/pet.sprite.json"),
            serde_json::to_vec_pretty(&config).unwrap(),
        )
        .unwrap();
        assert!(
            validate_directory(&invalid_idle.0)
                .unwrap_err()
                .to_string()
                .contains("must loop")
        );

        let invalid_frame = TestDirectory::new("invalid-frame");
        write_valid_source(&invalid_frame.0, "1.0.0");
        let mut config: Value = serde_json::from_slice(
            &fs::read(invalid_frame.0.join("model/pet.sprite.json")).unwrap(),
        )
        .unwrap();
        config["clips"]["happy"]["frames"] = serde_json::json!([99]);
        fs::write(
            invalid_frame.0.join("model/pet.sprite.json"),
            serde_json::to_vec_pretty(&config).unwrap(),
        )
        .unwrap();
        assert!(
            validate_directory(&invalid_frame.0)
                .unwrap_err()
                .to_string()
                .contains("out-of-range frame")
        );
    }

    #[test]
    fn rejects_invalid_toggle_and_interaction_targets() {
        let invalid_toggle = TestDirectory::new("invalid-toggle");
        write_valid_source(&invalid_toggle.0, "1.0.0");
        let mut manifest = valid_manifest("1.0.0");
        let PetAction::Animation { mode, .. } = &mut manifest.actions[0];
        *mode = PetActionMode::Toggle;
        fs::write(
            invalid_toggle.0.join(MANIFEST_FILE),
            serde_json::to_vec_pretty(&manifest).unwrap(),
        )
        .unwrap();
        assert!(
            validate_directory(&invalid_toggle.0)
                .unwrap_err()
                .to_string()
                .contains("must reference a looping clip")
        );

        let invalid_interaction = TestDirectory::new("invalid-interaction");
        write_valid_source(&invalid_interaction.0, "1.0.0");
        let mut config: Value = serde_json::from_slice(
            &fs::read(invalid_interaction.0.join("model/pet.sprite.json")).unwrap(),
        )
        .unwrap();
        config["interactions"]["tap"] = Value::String("missing".to_owned());
        fs::write(
            invalid_interaction.0.join("model/pet.sprite.json"),
            serde_json::to_vec_pretty(&config).unwrap(),
        )
        .unwrap();
        assert!(
            validate_directory(&invalid_interaction.0)
                .unwrap_err()
                .to_string()
                .contains("missing action 'missing'")
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
        assert_eq!(
            schema["$defs"]["runtime"]["properties"]["type"]["const"],
            "sprite2d"
        );

        let sprite_schema: Value =
            serde_json::from_str(include_str!("../../schemas/momopet-sprite-v1.schema.json"))
                .unwrap();
        assert_eq!(sprite_schema["additionalProperties"], false);
        assert_eq!(sprite_schema["properties"]["clips"]["required"][0], "idle");
    }

    #[test]
    fn checked_in_example_matches_the_runtime_manifest_contract() {
        let manifest: PetManifest = serde_json::from_str(include_str!(
            "../../examples/pet-package/manifest.example.json"
        ))
        .unwrap();

        assert_eq!(manifest.protocol_version, PROTOCOL_VERSION);
        assert_eq!(manifest.runtime.profile_version, RUNTIME_PROFILE_VERSION);
        assert_eq!(manifest.actions[0].id(), "happy");
        assert_eq!(manifest.runtime.runtime_type, PetRuntimeType::Sprite2d);

        let config: SpriteConfig = serde_json::from_str(include_str!(
            "../../examples/pet-package/model/pet.sprite.example.json"
        ))
        .unwrap();
        assert!(config.clips["idle"].r#loop);
        assert_eq!(config.frame_size.width, 512);
    }

    #[test]
    fn checked_in_momo_uses_the_public_package_validator() {
        let root =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/models/com.4096bytes.momopet.momo");
        let validated = validate_directory(&root).unwrap();

        assert_eq!(validated.manifest.id, "com.4096bytes.momopet.momo");
        assert_eq!(
            validated.manifest.runtime.runtime_type,
            PetRuntimeType::Sprite2d
        );
        assert_eq!(validated.manifest.actions.len(), 3);

        let config: SpriteConfig =
            serde_json::from_slice(&fs::read(root.join("model/pet.sprite.json")).unwrap()).unwrap();
        let expected = [
            ("idle", 60, 12),
            ("happy", 18, 18),
            ("curious", 18, 18),
            ("sleep", 8, 6),
        ];

        for (clip_id, unique_frames, fps) in expected {
            let clip = &config.clips[clip_id];
            assert_eq!(
                clip.frames.iter().copied().collect::<HashSet<_>>().len(),
                unique_frames
            );
            assert_eq!(clip.fps, fps);
        }
    }

    #[test]
    fn packs_numbered_frames_into_a_uniform_sprite_sheet() {
        let frames = TestDirectory::new("sprite-frames");
        let output = TestDirectory::new("sprite-output");
        write_sprite_png(&frames.0.join("001.png"), 1);
        write_sprite_png(&frames.0.join("002.png"), 1);
        write_sprite_png(&frames.0.join("003.png"), 1);

        let result = pack_sprite_sheet(&frames.0, &output.0.join("sheet.png"), 2).unwrap();

        assert_eq!(result.frame_count, 3);
        assert_eq!((result.columns, result.rows), (2, 2));
        assert_eq!(
            image::open(output.0.join("sheet.png"))
                .unwrap()
                .dimensions(),
            (8, 8)
        );
        assert!(pack_sprite_sheet(&frames.0, &output.0.join("sheet.png"), 2).is_err());
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
