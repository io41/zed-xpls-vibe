use std::{
    fs::{self, File},
    io::{self, Read},
    path::{Path, PathBuf},
};

use flate2::read::GzDecoder;
use sha2::{Digest, Sha256};
use tar::EntryType;

use crate::resolver::ArchiveKind;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChecksumMismatch {
    pub expected: String,
    pub actual: String,
}

pub fn sha256_file(path: &str) -> Result<String, String> {
    let mut file = File::open(path).map_err(|error| format!("failed to open `{path}`: {error}"))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 16 * 1024];

    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|error| format!("failed to read `{path}`: {error}"))?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }

    Ok(format!("{:x}", hasher.finalize()))
}

pub fn verify_sha256_file(path: &str, expected: &str) -> Result<(), ChecksumMismatch> {
    let actual = sha256_file(path).map_err(|error| ChecksumMismatch {
        expected: expected.to_string(),
        actual: error,
    })?;

    if actual.eq_ignore_ascii_case(expected) {
        Ok(())
    } else {
        Err(ChecksumMismatch {
            expected: expected.to_ascii_lowercase(),
            actual,
        })
    }
}

pub fn extract_archive(
    kind: ArchiveKind,
    archive_path: &str,
    destination: &str,
) -> Result<(), String> {
    match kind {
        ArchiveKind::GzipTar => extract_tar_gz(archive_path, destination),
        ArchiveKind::Zip => extract_zip(archive_path, destination),
    }
}

fn extract_tar_gz(archive_path: &str, destination: &str) -> Result<(), String> {
    let file = File::open(archive_path)
        .map_err(|error| format!("failed to open `{archive_path}`: {error}"))?;
    let decoder = GzDecoder::new(file);
    let mut archive = tar::Archive::new(decoder);

    for entry in archive
        .entries()
        .map_err(|error| format!("failed to read `{archive_path}`: {error}"))?
    {
        let mut entry = entry
            .map_err(|error| format!("failed to read tar entry from `{archive_path}`: {error}"))?;
        let entry_type = entry.header().entry_type();
        if entry_type.is_symlink() || entry_type.is_hard_link() {
            return Err("refusing to extract archive link entry".to_string());
        }
        if !(entry_type == EntryType::Regular || entry_type == EntryType::Directory) {
            return Err(format!(
                "refusing to extract unsupported tar entry type `{entry_type:?}`"
            ));
        }

        let path = entry
            .path()
            .map_err(|error| format!("failed to read tar entry path: {error}"))?;
        let relative = safe_archive_path(&path_to_string(&path)?)?;
        let destination_root = destination_root(destination)?;

        if entry_type == EntryType::Directory {
            prepare_destination_directory(&destination_root, &relative)?;
        } else {
            let mode = entry
                .header()
                .mode()
                .map_err(|error| format!("failed to read tar entry mode: {error}"))?;
            let target = prepare_destination_file(&destination_root, &relative)?;
            let mut output = File::create(&target)
                .map_err(|error| format!("failed to create `{}`: {error}", target.display()))?;
            io::copy(&mut entry, &mut output)
                .map_err(|error| format!("failed to extract `{}`: {error}", target.display()))?;
            apply_unix_permissions(&target, Some(mode))?;
        }
    }

    Ok(())
}

fn extract_zip(archive_path: &str, destination: &str) -> Result<(), String> {
    let file = File::open(archive_path)
        .map_err(|error| format!("failed to open `{archive_path}`: {error}"))?;
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|error| format!("failed to read `{archive_path}`: {error}"))?;

    for index in 0..archive.len() {
        let mut entry = archive
            .by_index(index)
            .map_err(|error| format!("failed to read zip entry {index}: {error}"))?;
        if zip_entry_is_symlink(&entry) {
            return Err("refusing to extract archive link entry".to_string());
        }
        if zip_entry_is_unsupported(&entry) {
            return Err("refusing to extract unsupported zip entry type".to_string());
        }

        let relative = safe_archive_path(entry.name())?;
        let destination_root = destination_root(destination)?;

        if entry.is_dir() {
            prepare_destination_directory(&destination_root, &relative)?;
        } else {
            let mode = entry.unix_mode();
            let target = prepare_destination_file(&destination_root, &relative)?;
            let mut output = File::create(&target)
                .map_err(|error| format!("failed to create `{}`: {error}", target.display()))?;
            io::copy(&mut entry, &mut output)
                .map_err(|error| format!("failed to extract `{}`: {error}", target.display()))?;
            apply_unix_permissions(&target, mode)?;
        }
    }

    Ok(())
}

fn safe_archive_path(raw: &str) -> Result<PathBuf, String> {
    let normalized = raw.replace('\\', "/");
    if normalized.starts_with('/') || normalized.starts_with("//") {
        return Err(format!("refusing to extract absolute archive path `{raw}`"));
    }
    if normalized.len() >= 2
        && normalized.as_bytes()[0].is_ascii_alphabetic()
        && normalized.as_bytes()[1] == b':'
    {
        return Err(format!("refusing to extract Windows drive path `{raw}`"));
    }

    let mut relative = PathBuf::new();
    for component in Path::new(&normalized).components() {
        match component {
            std::path::Component::Normal(part) => relative.push(part),
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                return Err(format!("refusing to extract parent traversal path `{raw}`"));
            }
            std::path::Component::RootDir | std::path::Component::Prefix(_) => {
                return Err(format!("refusing to extract unsafe archive path `{raw}`"));
            }
        }
    }

    if relative.as_os_str().is_empty() {
        return Err("refusing to extract empty archive path".to_string());
    }

    Ok(relative)
}

fn destination_root(destination: &str) -> Result<PathBuf, String> {
    let destination = PathBuf::from(destination);
    let root = if destination.is_absolute() {
        destination
    } else {
        let current_dir = std::env::current_dir()
            .map_err(|error| format!("failed to read current dir: {error}"))?;
        current_dir.join(destination)
    };

    let mut current = PathBuf::new();
    for component in root.components() {
        match component {
            std::path::Component::Prefix(prefix) => current.push(prefix.as_os_str()),
            std::path::Component::RootDir => current.push(component.as_os_str()),
            std::path::Component::CurDir => {}
            std::path::Component::Normal(part) => {
                current.push(part);
                ensure_directory_component(&current)?;
            }
            std::path::Component::ParentDir => {
                return Err(format!(
                    "refusing to extract into parent traversal destination `{}`",
                    root.display()
                ));
            }
        }
    }

    if current.as_os_str().is_empty() {
        return Err("refusing to extract into empty destination".to_string());
    }

    let metadata = fs::symlink_metadata(&current)
        .map_err(|error| format!("failed to inspect `{}`: {error}", current.display()))?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(format!(
            "refusing to extract into unsafe destination `{}`",
            current.display()
        ));
    }

    Ok(current)
}

fn prepare_destination_directory(root: &Path, relative: &Path) -> Result<PathBuf, String> {
    let mut current = root.to_path_buf();
    for component in relative.components() {
        let std::path::Component::Normal(part) = component else {
            return Err(format!(
                "refusing to extract unsafe archive path `{}`",
                relative.display()
            ));
        };

        current.push(part);
        ensure_directory_component(&current)?;
    }

    Ok(current)
}

fn prepare_destination_file(root: &Path, relative: &Path) -> Result<PathBuf, String> {
    let parent = relative
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty());
    if let Some(parent) = parent {
        prepare_destination_directory(root, parent)?;
    }

    let target = root.join(relative);
    if let Ok(metadata) = fs::symlink_metadata(&target) {
        if metadata.file_type().is_symlink() {
            return Err(format!(
                "refusing to extract through destination symlink `{}`",
                target.display()
            ));
        }
    }

    Ok(target)
}

fn ensure_directory_component(path: &Path) -> Result<(), String> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => {
            if metadata.file_type().is_symlink() {
                return Err(format!(
                    "refusing to extract through destination symlink `{}`",
                    path.display()
                ));
            }
            if !metadata.is_dir() {
                return Err(format!(
                    "refusing to extract through non-directory `{}`",
                    path.display()
                ));
            }
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            fs::create_dir(path)
                .map_err(|error| format!("failed to create `{}`: {error}", path.display()))?;
            let metadata = fs::symlink_metadata(path)
                .map_err(|error| format!("failed to inspect `{}`: {error}", path.display()))?;
            if metadata.file_type().is_symlink() || !metadata.is_dir() {
                return Err(format!(
                    "refusing to extract through unsafe destination `{}`",
                    path.display()
                ));
            }
        }
        Err(error) => {
            return Err(format!("failed to inspect `{}`: {error}", path.display()));
        }
    }

    Ok(())
}

#[cfg(unix)]
fn apply_unix_permissions(path: &Path, mode: Option<u32>) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;

    if let Some(mode) = mode {
        fs::set_permissions(path, fs::Permissions::from_mode(mode & 0o777)).map_err(|error| {
            format!("failed to set permissions on `{}`: {error}", path.display())
        })?;
    }

    Ok(())
}

#[cfg(not(unix))]
fn apply_unix_permissions(_path: &Path, _mode: Option<u32>) -> Result<(), String> {
    Ok(())
}

fn path_to_string(path: &Path) -> Result<String, String> {
    path.to_str()
        .map(str::to_string)
        .ok_or_else(|| format!("archive path is not valid UTF-8: `{}`", path.display()))
}

fn zip_entry_is_symlink(entry: &zip::read::ZipFile<'_, File>) -> bool {
    entry
        .unix_mode()
        .is_some_and(|mode| mode & 0o170000 == 0o120000)
}

fn zip_entry_is_unsupported(entry: &zip::read::ZipFile<'_, File>) -> bool {
    entry.unix_mode().is_some_and(|mode| {
        let file_type = mode & 0o170000;
        file_type != 0 && file_type != 0o100000 && file_type != 0o040000
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::sync::{Mutex, OnceLock};
    use tempfile::tempdir;

    #[test]
    fn sha256_file_returns_lowercase_hex_digest() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("sample.txt");
        fs::write(&path, b"crossplane-yaml\n").unwrap();

        assert_eq!(
            sha256_file(path.to_str().unwrap()).unwrap(),
            "a4239be924cdd597eb7b59664a686b9a9a7ea038b59edbca74608a870047b6a5"
        );
    }

    #[test]
    fn verify_sha256_file_rejects_mismatch() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("sample.txt");
        fs::write(&path, b"crossplane-yaml\n").unwrap();

        let mismatch = verify_sha256_file(path.to_str().unwrap(), &"0".repeat(64)).unwrap_err();

        assert_eq!(mismatch.expected, "0".repeat(64));
        assert_eq!(
            mismatch.actual,
            "a4239be924cdd597eb7b59664a686b9a9a7ea038b59edbca74608a870047b6a5"
        );
    }

    #[test]
    fn tar_gz_extraction_rejects_parent_traversal() {
        let dir = tempdir().unwrap();
        let archive_path = dir.path().join("bad.tar.gz");
        write_tar_gz(&archive_path, "../vibe-xpls", b"bad", TarEntryKind::File);

        let error = extract_archive(
            ArchiveKind::GzipTar,
            archive_path.to_str().unwrap(),
            canonical_temp_path(&dir).join("out").to_str().unwrap(),
        )
        .unwrap_err();

        assert!(error.contains("parent traversal"));
        assert!(!dir.path().join("vibe-xpls").exists());
    }

    #[test]
    fn tar_gz_extraction_rejects_symlink_entries() {
        let dir = tempdir().unwrap();
        let archive_path = dir.path().join("bad-link.tar.gz");
        write_tar_gz(
            &archive_path,
            "vibe-xpls",
            b"/tmp/target",
            TarEntryKind::Symlink,
        );

        let error = extract_archive(
            ArchiveKind::GzipTar,
            archive_path.to_str().unwrap(),
            canonical_temp_path(&dir).join("out").to_str().unwrap(),
        )
        .unwrap_err();

        assert!(error.contains("link entry"));
    }

    #[test]
    fn tar_gz_extraction_rejects_hardlink_entries() {
        let dir = tempdir().unwrap();
        let archive_path = dir.path().join("bad-hardlink.tar.gz");
        write_tar_gz(
            &archive_path,
            "vibe-xpls",
            b"target",
            TarEntryKind::Hardlink,
        );

        let error = extract_archive(
            ArchiveKind::GzipTar,
            archive_path.to_str().unwrap(),
            canonical_temp_path(&dir).join("out").to_str().unwrap(),
        )
        .unwrap_err();

        assert!(error.contains("link entry"));
    }

    #[test]
    fn tar_gz_extraction_rejects_unsupported_entry_types() {
        let dir = tempdir().unwrap();
        let archive_path = dir.path().join("bad-fifo.tar.gz");
        write_tar_gz(&archive_path, "vibe-xpls", b"", TarEntryKind::Fifo);

        let error = extract_archive(
            ArchiveKind::GzipTar,
            archive_path.to_str().unwrap(),
            canonical_temp_path(&dir).join("out").to_str().unwrap(),
        )
        .unwrap_err();

        assert!(error.contains("unsupported tar entry type"));
    }

    #[cfg(unix)]
    #[test]
    fn tar_gz_extraction_rejects_preexisting_symlinked_parent() {
        let dir = tempdir().unwrap();
        let archive_path = dir.path().join("symlink-parent.tar.gz");
        let root = canonical_temp_path(&dir);
        let out = root.join("out");
        let outside = root.join("outside");
        fs::create_dir_all(&outside).unwrap();
        fs::create_dir_all(&out).unwrap();
        std::os::unix::fs::symlink(&outside, out.join("bin")).unwrap();
        write_tar_gz(
            &archive_path,
            "bin/vibe-xpls",
            b"server",
            TarEntryKind::File,
        );

        let error = extract_archive(
            ArchiveKind::GzipTar,
            archive_path.to_str().unwrap(),
            out.to_str().unwrap(),
        )
        .unwrap_err();

        assert!(error.contains("destination symlink"));
        assert!(!outside.join("vibe-xpls").exists());
    }

    #[cfg(unix)]
    #[test]
    fn tar_gz_extraction_rejects_symlinked_destination_root_ancestor() {
        let dir = tempdir().unwrap();
        let archive_path = dir.path().join("symlink-root-ancestor.tar.gz");
        let root = canonical_temp_path(&dir);
        let out = root.join("out");
        let destination = out.join("cache");
        let outside = root.join("outside");
        fs::create_dir_all(&outside).unwrap();
        std::os::unix::fs::symlink(&outside, &out).unwrap();
        write_tar_gz(&archive_path, "vibe-xpls", b"server", TarEntryKind::File);

        let error = extract_archive(
            ArchiveKind::GzipTar,
            archive_path.to_str().unwrap(),
            destination.to_str().unwrap(),
        )
        .unwrap_err();

        assert!(error.contains("destination symlink"));
        assert!(!outside.join("cache").join("vibe-xpls").exists());
    }

    #[cfg(unix)]
    #[test]
    fn relative_destination_is_anchored_to_current_dir_before_extraction() {
        let _guard = cwd_lock().lock().unwrap();
        let original_cwd = std::env::current_dir().unwrap();
        let dir = tempdir().unwrap();
        let archive_path = dir.path().join("relative-cwd.tar.gz");
        let real_cwd = canonical_temp_path(&dir).join("real-cwd");
        let symlink_cwd = canonical_temp_path(&dir).join("symlink-cwd");
        fs::create_dir_all(&real_cwd).unwrap();
        std::os::unix::fs::symlink(&real_cwd, &symlink_cwd).unwrap();
        write_tar_gz(&archive_path, "vibe-xpls", b"server", TarEntryKind::File);

        std::env::set_current_dir(&symlink_cwd).unwrap();
        let anchored_cwd = std::env::current_dir().unwrap();
        let result = extract_archive(
            ArchiveKind::GzipTar,
            archive_path.to_str().unwrap(),
            "relative-out",
        );
        std::env::set_current_dir(original_cwd).unwrap();

        result.unwrap();
        assert_eq!(
            fs::read(anchored_cwd.join("relative-out").join("vibe-xpls")).unwrap(),
            b"server"
        );
    }

    #[test]
    fn tar_gz_extraction_writes_regular_file() {
        let dir = tempdir().unwrap();
        let archive_path = dir.path().join("good.tar.gz");
        let out = canonical_temp_path(&dir).join("out");
        write_tar_gz(&archive_path, "vibe-xpls", b"server", TarEntryKind::File);

        extract_archive(
            ArchiveKind::GzipTar,
            archive_path.to_str().unwrap(),
            out.to_str().unwrap(),
        )
        .unwrap();

        assert_eq!(fs::read(out.join("vibe-xpls")).unwrap(), b"server");
        assert_eq!(file_mode(&out.join("vibe-xpls")) & 0o777, 0o755);
    }

    #[test]
    fn zip_extraction_rejects_windows_drive_paths() {
        let dir = tempdir().unwrap();
        let archive_path = dir.path().join("bad.zip");
        write_zip(
            &archive_path,
            "C:\\tmp\\vibe-xpls.exe",
            b"bad",
            Some(0o100755),
        );

        let error = extract_archive(
            ArchiveKind::Zip,
            archive_path.to_str().unwrap(),
            canonical_temp_path(&dir).join("out").to_str().unwrap(),
        )
        .unwrap_err();

        assert!(error.contains("Windows drive path"));
    }

    #[test]
    fn zip_extraction_rejects_symlink_entries() {
        let dir = tempdir().unwrap();
        let archive_path = dir.path().join("bad-link.zip");
        write_zip(&archive_path, "vibe-xpls.exe", b"target", Some(0o120777));

        let error = extract_archive(
            ArchiveKind::Zip,
            archive_path.to_str().unwrap(),
            canonical_temp_path(&dir).join("out").to_str().unwrap(),
        )
        .unwrap_err();

        assert!(error.contains("link entry"));
    }

    #[test]
    fn zip_extraction_rejects_unsupported_file_types() {
        let dir = tempdir().unwrap();
        let archive_path = dir.path().join("bad-fifo.zip");
        write_zip(&archive_path, "vibe-xpls.exe", b"target", Some(0o010777));

        let error = extract_archive(
            ArchiveKind::Zip,
            archive_path.to_str().unwrap(),
            canonical_temp_path(&dir).join("out").to_str().unwrap(),
        )
        .unwrap_err();

        assert!(error.contains("unsupported zip entry type"));
    }

    #[test]
    fn zip_extraction_writes_regular_file() {
        let dir = tempdir().unwrap();
        let archive_path = dir.path().join("good.zip");
        let out = canonical_temp_path(&dir).join("out");
        write_zip(&archive_path, "vibe-xpls.exe", b"server", Some(0o100755));

        extract_archive(
            ArchiveKind::Zip,
            archive_path.to_str().unwrap(),
            out.to_str().unwrap(),
        )
        .unwrap();

        assert_eq!(fs::read(out.join("vibe-xpls.exe")).unwrap(), b"server");
        assert_eq!(file_mode(&out.join("vibe-xpls.exe")) & 0o777, 0o755);
    }

    enum TarEntryKind {
        File,
        Symlink,
        Hardlink,
        Fifo,
    }

    fn write_tar_gz(path: &Path, entry_path: &str, bytes: &[u8], kind: TarEntryKind) {
        let file = File::create(path).unwrap();
        let encoder = flate2::write::GzEncoder::new(file, flate2::Compression::default());
        let mut builder = tar::Builder::new(encoder);
        let mut header = tar::Header::new_gnu();

        match kind {
            TarEntryKind::File => {
                header.set_entry_type(EntryType::Regular);
                set_raw_tar_path(&mut header, entry_path);
                header.set_size(bytes.len() as u64);
                header.set_mode(0o755);
                header.set_cksum();
                builder.append(&header, bytes).unwrap();
            }
            TarEntryKind::Symlink => {
                header.set_entry_type(EntryType::Symlink);
                set_raw_tar_path(&mut header, entry_path);
                header.set_size(0);
                header.set_mode(0o777);
                header
                    .set_link_name(std::str::from_utf8(bytes).unwrap())
                    .unwrap();
                header.set_cksum();
                builder.append(&header, io::empty()).unwrap();
            }
            TarEntryKind::Hardlink => {
                header.set_entry_type(EntryType::Link);
                set_raw_tar_path(&mut header, entry_path);
                header.set_size(0);
                header.set_mode(0o777);
                header
                    .set_link_name(std::str::from_utf8(bytes).unwrap())
                    .unwrap();
                header.set_cksum();
                builder.append(&header, io::empty()).unwrap();
            }
            TarEntryKind::Fifo => {
                header.set_entry_type(EntryType::Fifo);
                set_raw_tar_path(&mut header, entry_path);
                header.set_size(0);
                header.set_mode(0o777);
                header.set_cksum();
                builder.append(&header, io::empty()).unwrap();
            }
        }

        builder.finish().unwrap();
    }

    fn canonical_temp_path(dir: &tempfile::TempDir) -> PathBuf {
        fs::canonicalize(dir.path()).unwrap()
    }

    fn cwd_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    #[cfg(unix)]
    fn file_mode(path: &Path) -> u32 {
        use std::os::unix::fs::PermissionsExt;

        fs::metadata(path).unwrap().permissions().mode()
    }

    #[cfg(not(unix))]
    fn file_mode(_path: &Path) -> u32 {
        0o755
    }

    fn set_raw_tar_path(header: &mut tar::Header, entry_path: &str) {
        let name = entry_path.as_bytes();
        assert!(name.len() <= 100);
        header.as_gnu_mut().unwrap().name[..name.len()].copy_from_slice(name);
    }

    fn write_zip(path: &Path, entry_path: &str, bytes: &[u8], unix_mode: Option<u32>) {
        let file = File::create(path).unwrap();
        let mut writer = zip::ZipWriter::new(file);
        let mut options = zip::write::SimpleFileOptions::default();
        if let Some(unix_mode) = unix_mode {
            options = options.unix_permissions(unix_mode);
        }

        writer.start_file(entry_path, options).unwrap();
        writer.write_all(bytes).unwrap();
        writer.finish().unwrap();

        if let Some(unix_mode) = unix_mode {
            if unix_mode & 0o170000 != 0o100000 {
                mark_zip_entry_as_unix_mode(path, unix_mode);
            }
        }
    }

    fn mark_zip_entry_as_unix_mode(path: &Path, unix_mode: u32) {
        let mut bytes = fs::read(path).unwrap();
        let central_header = bytes
            .windows(4)
            .position(|window| window == b"PK\x01\x02")
            .unwrap();
        bytes[central_header + 5] = 3;
        bytes[central_header + 38..central_header + 42]
            .copy_from_slice(&(unix_mode << 16).to_le_bytes());
        fs::write(path, bytes).unwrap();
    }
}
