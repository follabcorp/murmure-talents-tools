//! The disk, descriptor-relative (`openat` with `O_NOFOLLOW`): what a contribution links never gets read.

use std::path::{Path, PathBuf};

#[cfg(not(unix))]
use std::collections::BTreeMap;
#[cfg(not(unix))]
use crate::application::{CatalogDisk, CatalogSnapshot};
#[cfg(not(unix))]
use crate::domain::{CatalogIndexArtifact, CatalogSource, SourceSnapshot};
use crate::domain::CatalogError;

pub struct FsCatalogDisk;

impl FsCatalogDisk {
    /// The root with its links resolved, as every later check compares real paths.
    pub fn resolve(path: &Path) -> PathBuf {
        std::fs::canonicalize(path).unwrap_or_else(|_| path.to_owned())
    }
}

fn invalid(message: &str) -> CatalogError {
    CatalogError::new(message)
}

#[cfg(unix)]
mod unix {
    use std::collections::BTreeMap;
    use std::ffi::OsStr;
    use std::io::{Read, Write};
    use std::os::fd::OwnedFd;
    use std::path::{Component, Path, PathBuf};

    use murmure_talents_kit::{TalentIndex, TalentIndexRevocation, TalentRevocationReason};
    use rustix::fs::{fstat, statat, mkdirat, openat, AtFlags, Dir, FileType, Mode, OFlags, CWD};
    use serde_json::Value;
    use strict_json::parse_strict_json;

    use super::{invalid, FsCatalogDisk};
    use crate::application::{CatalogDisk, CatalogSnapshot};
    use crate::domain::{CatalogError, CatalogIndexArtifact, CatalogSource, CatalogFile, FileKind, SigningKeyFacts, SigningKeyRule, SourceSnapshot};

    const FILES_MAX: usize = 32;
    const ENTRIES_MAX: usize = 258;
    const SEGMENTS_MAX: usize = 8;
    const BYTES_MAX: u64 = 256 * 1024;
    const MANIFEST_MAX: u64 = 16_384;
    /// A conversion source is a whole clone: its bounds are global and generous, the per-skill bounds are a rule.
    const SOURCE_ENTRIES_MAX: usize = 20_000;
    const SOURCE_DEPTH_MAX: usize = 16;
    const SOURCE_BYTES_MAX: u64 = 64 * 1024 * 1024;
    /// The only hidden folders read: bundle manifests, needed to sort the components.
    const PLUGIN_FOLDERS: [&str; 3] = [".claude-plugin", ".codex-plugin", ".cursor-plugin"];

    /// Mode bits and link count, whose integer types differ between macOS and Linux.
    #[allow(clippy::useless_conversion)]
    fn mode_and_links(attributes: &rustix::fs::Stat) -> (u32, u64) {
        (u32::from(attributes.st_mode), u64::from(attributes.st_nlink))
    }

    fn directory_flags() -> OFlags {
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC
    }

    fn open_child(parent: &OwnedFd, name: &OsStr) -> Result<OwnedFd, CatalogError> {
        openat(parent, name, directory_flags(), Mode::empty()).map_err(|_| invalid("Dossier inaccessible ou lié."))
    }

    /// Opens an absolute, already canonical path one component at a time without following any link.
    fn open_directory(path: &Path) -> Result<OwnedFd, CatalogError> {
        let mut current = openat(CWD, "/", directory_flags(), Mode::empty()).map_err(|_| invalid("Ouverture du dossier impossible."))?;
        for component in path.components() {
            match component {
                Component::Normal(name) => current = open_child(&current, name)?,
                Component::RootDir => {}
                _ => return Err(invalid("Ouverture du dossier impossible.")),
            }
        }
        Ok(current)
    }

    fn names(directory: &OwnedFd, limit: usize) -> Result<Vec<String>, CatalogError> {
        let reading = || invalid("Lecture du dossier impossible.");
        let mut names = Vec::new();
        for entry in Dir::read_from(directory).map_err(|_| reading())? {
            let entry = entry.map_err(|_| invalid("Lecture du dossier interrompue."))?;
            let name = entry.file_name().to_str().map_err(|_| invalid("Nom de fichier non UTF-8."))?;
            if name == "." || name == ".." {
                continue;
            }
            names.push(name.to_owned());
            if names.len() > limit {
                return Err(invalid("Dossier trop volumineux."));
            }
        }
        names.sort();
        Ok(names)
    }

    /// A regular file with one link, not executable, at most `maximum` bytes; a private key must also be `0600`.
    fn read(directory: &OwnedFd, name: &OsStr, maximum: u64, private_key: bool) -> Result<(Vec<u8>, u16), CatalogError> {
        let refused = || invalid("Fichier spécial, exécutable, lié, trop grand ou permissions invalides.");
        let descriptor = openat(directory, name, OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC, Mode::empty())
            .map_err(|_| invalid("Fichier inaccessible ou lié."))?;
        let attributes = fstat(&descriptor).map_err(|_| refused())?;
        let (mode, links) = mode_and_links(&attributes);
        let allowed = FileType::from_raw_mode(attributes.st_mode) == FileType::RegularFile
            && links == 1
            && mode & 0o111 == 0
            && u64::try_from(attributes.st_size).is_ok_and(|size| size <= maximum)
            && (!private_key || mode & 0o7777 == 0o600);
        if !allowed {
            return Err(refused());
        }
        let mut data = Vec::new();
        std::fs::File::from(descriptor).take(maximum + 1).read_to_end(&mut data).map_err(|_| invalid("Lecture impossible."))?;
        if data.len() as u64 > maximum {
            return Err(invalid("Fichier trop grand."));
        }
        Ok((data, (mode & 0o7777) as u16))
    }

    fn write(data: &[u8], path: &str, directory: &OwnedFd) -> Result<(), CatalogError> {
        let parts: Vec<&str> = path.split('/').collect();
        if parts.iter().any(|part| part.is_empty() || *part == "." || *part == "..") {
            return Err(invalid("Chemin de sortie invalide."));
        }
        let mut parent = directory.try_clone().map_err(|_| invalid("Ouverture de sortie impossible."))?;
        for part in &parts[..parts.len() - 1] {
            if let Err(error) = mkdirat(&parent, *part, Mode::from_bits_truncate(0o755)) {
                if error != rustix::io::Errno::EXIST {
                    return Err(invalid("Création du dossier impossible."));
                }
            }
            parent = open_child(&parent, OsStr::new(part))?;
        }
        let file = openat(
            &parent,
            parts[parts.len() - 1],
            OFlags::WRONLY | OFlags::CREATE | OFlags::EXCL | OFlags::NOFOLLOW | OFlags::CLOEXEC,
            Mode::from_bits_truncate(0o644),
        )
        .map_err(|_| invalid("Fichier de sortie existant ou inaccessible."))?;
        std::fs::File::from(file).write_all(data).map_err(|_| invalid("Écriture impossible."))
    }

    fn file_name(path: &Path) -> Result<&OsStr, CatalogError> {
        path.file_name().ok_or_else(|| invalid("Chemin invalide."))
    }

    fn parent_directory(path: &Path) -> Result<(OwnedFd, PathBuf), CatalogError> {
        let parent = match path.parent() {
            Some(parent) if !parent.as_os_str().is_empty() => parent,
            _ => Path::new("."),
        };
        let resolved = std::fs::canonicalize(parent).map_err(|_| invalid("Ouverture du dossier impossible."))?;
        Ok((open_directory(&resolved)?, resolved))
    }

    /// Creates `output` as a new folder (its parent exists), distinct from `source` when given, fills it with
    /// `fill` and removes it again when anything fails: no half-written output is left behind.
    fn write_new_folder(
        output: &Path,
        source: Option<&Path>,
        fill: impl FnOnce(&OwnedFd) -> Result<(), CatalogError>,
    ) -> Result<(), CatalogError> {
        let name = file_name(output)?;
        let (parent, resolved_parent) = parent_directory(output)?;
        let canonical = resolved_parent.join(name);
        if source.is_some_and(|source| canonical.starts_with(source) || source.starts_with(&canonical)) {
            return Err(invalid("La sortie doit être distincte de la source."));
        }
        mkdirat(&parent, name, Mode::from_bits_truncate(0o755))
            .map_err(|_| invalid("La sortie doit être un dossier neuf sous un parent existant."))?;
        let result = open_child(&parent, name).and_then(|destination| fill(&destination));
        if result.is_err() {
            let _ = std::fs::remove_dir_all(&canonical);
        }
        result
    }

    struct Collection {
        files: Vec<CatalogFile>,
        manifest: Option<Vec<u8>>,
        entries: usize,
        bytes: u64,
    }

    fn collect(directory: &OwnedFd, prefix: &str, depth: usize, found: &mut Collection) -> Result<(), CatalogError> {
        if depth > SEGMENTS_MAX {
            return Err(invalid("Plus de huit segments de chemin."));
        }
        for name in names(directory, ENTRIES_MAX)? {
            found.entries += 1;
            if found.entries > ENTRIES_MAX {
                return Err(invalid("Arborescence trop volumineuse."));
            }
            let path = format!("{prefix}{name}");
            let attributes = statat(directory, name.as_str(), AtFlags::SYMLINK_NOFOLLOW)
                .map_err(|_| invalid("Lecture des attributs impossible."))?;
            if FileType::from_raw_mode(attributes.st_mode) == FileType::Directory {
                let child = open_child(directory, OsStr::new(&name))?;
                collect(&child, &format!("{path}/"), depth + 1, found)?;
                continue;
            }
            if path == "talent.json" {
                found.manifest = Some(read(directory, OsStr::new(&name), MANIFEST_MAX, false)?.0);
                continue;
            }
            let (data, permissions) = read(directory, OsStr::new(&name), BYTES_MAX.saturating_sub(found.bytes), false)?;
            found.bytes += data.len() as u64;
            found.files.push(CatalogFile { path, data, kind: FileKind::Regular, permissions });
            if found.files.len() > FILES_MAX {
                return Err(invalid("Plus de 32 fichiers."));
            }
        }
        Ok(())
    }

    struct SourceWalk {
        snapshot: SourceSnapshot,
        entries: usize,
        bytes: u64,
    }

    /// Walks a conversion source without following any link; hidden folders (`.git`, `.github`…) are not entered.
    /// A file over `BYTES_MAX` is noted, not read; any other unreadable entry fails the whole walk.
    fn walk_source(directory: &OwnedFd, prefix: &str, depth: usize, walk: &mut SourceWalk) -> Result<(), CatalogError> {
        if depth > SOURCE_DEPTH_MAX {
            return Err(invalid("Plus de seize niveaux de dossiers."));
        }
        for name in names(directory, SOURCE_ENTRIES_MAX)? {
            walk.entries += 1;
            if walk.entries > SOURCE_ENTRIES_MAX {
                return Err(invalid("Arborescence trop volumineuse."));
            }
            let path = format!("{prefix}{name}");
            let attributes = statat(directory, name.as_str(), AtFlags::SYMLINK_NOFOLLOW)
                .map_err(|_| invalid("Lecture des attributs impossible."))?;
            if FileType::from_raw_mode(attributes.st_mode) == FileType::Directory {
                if name.starts_with('.') && !PLUGIN_FOLDERS.contains(&name.as_str()) {
                    continue;
                }
                let child = open_child(directory, OsStr::new(&name))?;
                walk_source(&child, &format!("{path}/"), depth + 1, walk)?;
                continue;
            }
            if FileType::from_raw_mode(attributes.st_mode) == FileType::RegularFile
                && u64::try_from(attributes.st_size).is_ok_and(|size| size > BYTES_MAX)
            {
                walk.snapshot.oversized.push(path);
                continue;
            }
            let (data, permissions) = read(directory, OsStr::new(&name), BYTES_MAX, false)?;
            walk.bytes += data.len() as u64;
            if walk.bytes > SOURCE_BYTES_MAX {
                return Err(invalid("Source trop volumineuse."));
            }
            walk.snapshot.files.push(CatalogFile { path, data, kind: FileKind::Regular, permissions });
        }
        Ok(())
    }

    fn strings(data: &[u8]) -> Result<Vec<String>, CatalogError> {
        let malformed = || invalid("featured.json ou revoked.json invalide.");
        parse_strict_json(data).map_err(|_| malformed())?.as_array().ok_or_else(malformed)?.iter()
            .map(|item| item.as_str().map(str::to_owned).ok_or_else(malformed)).collect()
    }

    fn revocations(data: &[u8]) -> Result<Vec<TalentIndexRevocation>, CatalogError> {
        let malformed = || invalid("featured.json ou revoked.json invalide.");
        let value = parse_strict_json(data).map_err(|_| malformed())?;
        value.as_array().ok_or_else(malformed)?.iter().map(|item| {
            let id = item.get("id").and_then(Value::as_str).ok_or_else(malformed)?;
            let reason = item.get("reason").and_then(Value::as_str).and_then(TalentRevocationReason::from_wire).ok_or_else(malformed)?;
            Ok(TalentIndexRevocation { id: id.to_owned(), reason })
        }).collect()
    }

    /// The nearest folder holding `.git` at or above `root`; the root itself when there is none.
    fn repository_root(root: &Path) -> PathBuf {
        root.ancestors().find(|candidate| candidate.join(".git").exists()).unwrap_or(root).to_owned()
    }

    impl CatalogDisk for FsCatalogDisk {
        fn talents(&self, root: &Path) -> Result<Vec<CatalogSource>, CatalogError> {
            let directory = open_directory(root)?;
            let talents_directory = open_child(&directory, OsStr::new("talents"))?;
            let mut talents = Vec::new();
            for id in names(&talents_directory, 2000)? {
                if !murmure_talents_kit::is_valid_identifier(&id) {
                    return Err(invalid("Identifiant de dossier invalide."));
                }
                let talent = open_child(&talents_directory, OsStr::new(&id))?;
                let mut found = Collection { files: Vec::new(), manifest: None, entries: 0, bytes: 0 };
                collect(&talent, "", 1, &mut found)?;
                let manifest_json = found.manifest.ok_or_else(|| invalid("Manifeste manquant."))?;
                talents.push(CatalogSource { directory_id: id, manifest_json, files: found.files });
            }
            Ok(talents)
        }

        fn snapshot(&self, root: &Path) -> Result<CatalogSnapshot, CatalogError> {
            let directory = open_directory(root)?;
            let talents = self.talents(root)?;
            let featured = strings(&read(&directory, OsStr::new("featured.json"), MANIFEST_MAX, false)?.0)?;
            let revoked = revocations(&read(&directory, OsStr::new("revoked.json"), BYTES_MAX, false)?.0)?;
            let codeowners = match open_child(&directory, OsStr::new(".github")) {
                Ok(github) => match read(&github, OsStr::new("CODEOWNERS"), MANIFEST_MAX, false) {
                    Ok((data, _)) => Some(String::from_utf8(data).map_err(|_| invalid("CODEOWNERS non UTF-8."))?),
                    Err(_) => None,
                },
                Err(_) => None,
            };
            Ok(CatalogSnapshot { talents, featured, revoked, codeowners })
        }

        fn source_files(&self, source: &Path) -> Result<SourceSnapshot, CatalogError> {
            let directory = open_directory(source)?;
            let mut walk = SourceWalk { snapshot: SourceSnapshot::default(), entries: 0, bytes: 0 };
            walk_source(&directory, "", 1, &mut walk)?;
            Ok(walk.snapshot)
        }

        fn write_proposals(&self, talents: &[CatalogSource], output: &Path, source: &Path) -> Result<(), CatalogError> {
            write_new_folder(output, Some(source), |destination| {
                for talent in talents {
                    let folder = format!("talents/{}", talent.directory_id);
                    write(&talent.manifest_json, &format!("{folder}/talent.json"), destination)?;
                    for file in &talent.files {
                        write(&file.data, &format!("{folder}/{}", file.path), destination)?;
                    }
                }
                write(b"[]\n", "featured.json", destination)?;
                write(b"[]\n", "revoked.json", destination)
            })
        }

        fn publish(
            &self,
            artifact: &CatalogIndexArtifact,
            extra: &BTreeMap<String, Vec<u8>>,
            output: &Path,
            source: &Path,
        ) -> Result<(), CatalogError> {
            write_new_folder(output, Some(source), |destination| {
                // The index goes last: a partial copy is never a complete publication.
                let files: BTreeMap<&String, &Vec<u8>> = artifact.files.iter().chain(extra).collect();
                for (path, data) in files {
                    write(data, path, destination)?;
                }
                write(&artifact.index_data, "index.json", destination)
            })
        }

        fn write_delegation(&self, document: &[u8], signature: &[u8], output: &Path) -> Result<(), CatalogError> {
            write_new_folder(output, None, |destination| {
                write(document, "delegation.json", destination)?;
                write(signature, "delegation.json.sig", destination)
            })
        }

        fn read_delegation(&self, folder: &Path) -> Result<(Vec<u8>, Vec<u8>), CatalogError> {
            let resolved = std::fs::canonicalize(folder).map_err(|_| invalid("Dossier de délégation inaccessible."))?;
            let directory = open_directory(&resolved)?;
            let document = read(&directory, OsStr::new("delegation.json"), 4096, false)?.0;
            let signature = read(&directory, OsStr::new("delegation.json.sig"), 4096, false)?.0;
            Ok((document, signature))
        }

        fn read_private_key(&self, key: &Path, repository: &Path) -> Result<[u8; 32], CatalogError> {
            let name = file_name(key)?;
            let (key_parent, _) = parent_directory(key)?;
            // The facts are observed before any private byte is read.
            let attributes = statat(&key_parent, name, AtFlags::SYMLINK_NOFOLLOW).map_err(|_| invalid("Clé privée non régulière ou liée."))?;
            let kind = match FileType::from_raw_mode(attributes.st_mode) {
                FileType::RegularFile => FileKind::Regular,
                FileType::Symlink => FileKind::SymbolicLink,
                _ => FileKind::Other,
            };
            let (mode, links) = mode_and_links(&attributes);
            let resolved_key = std::fs::canonicalize(key).map_err(|_| invalid("Clé privée non régulière ou liée."))?;
            let key_repository = resolved_key.ancestors().skip(1).find(|candidate| candidate.join(".git").exists()).map(Path::to_owned);
            SigningKeyRule::validate(&SigningKeyFacts {
                resolved_key,
                repository: repository_root(repository),
                key_repository,
                kind,
                link_count: u32::try_from(links).unwrap_or(u32::MAX),
                permissions: (mode & 0o7777) as u16,
                length: u64::try_from(attributes.st_size).unwrap_or(u64::MAX),
            })?;
            read(&key_parent, name, 32, true)?.0.try_into().map_err(|_| invalid("Clé Ed25519 brute de 32 octets attendue."))
        }

        fn read_signing_input(&self, index: &Path, key: &Path, root: &Path) -> Result<(Vec<u8>, [u8; 32]), CatalogError> {
            let secret = self.read_private_key(key, root)?;
            let (index_directory, _) = parent_directory(index)?;
            let data = read(&index_directory, file_name(index)?, TalentIndex::MAXIMUM_BYTES as u64, false)?.0;
            TalentIndex::parse(&data).map_err(|_| invalid("Index invalide."))?;
            Ok((data, secret))
        }

        fn write_signature(&self, signature: &[u8], index: &Path) -> Result<(), CatalogError> {
            let (parent, _) = parent_directory(index)?;
            let name = format!("{}.sig", file_name(index)?.to_str().ok_or_else(|| invalid("Chemin invalide."))?);
            write(signature, &name, &parent)
        }
    }
}

#[cfg(not(unix))]
impl CatalogDisk for FsCatalogDisk {
    fn talents(&self, _root: &Path) -> Result<Vec<CatalogSource>, CatalogError> {
        Err(invalid("Plateforme non prise en charge : l’outil du Catalogue tourne sur macOS ou Linux."))
    }

    fn snapshot(&self, _root: &Path) -> Result<CatalogSnapshot, CatalogError> {
        Err(invalid("Plateforme non prise en charge : l’outil du Catalogue tourne sur macOS ou Linux."))
    }

    fn publish(
        &self,
        _artifact: &CatalogIndexArtifact,
        _extra: &BTreeMap<String, Vec<u8>>,
        _output: &Path,
        _source: &Path,
    ) -> Result<(), CatalogError> {
        self.snapshot(Path::new("")).map(|_| ())
    }

    fn write_delegation(&self, _document: &[u8], _signature: &[u8], _output: &Path) -> Result<(), CatalogError> {
        self.snapshot(Path::new("")).map(|_| ())
    }

    fn read_delegation(&self, _folder: &Path) -> Result<(Vec<u8>, Vec<u8>), CatalogError> {
        self.snapshot(Path::new("")).map(|_| (Vec::new(), Vec::new()))
    }

    fn read_private_key(&self, _key: &Path, _repository: &Path) -> Result<[u8; 32], CatalogError> {
        self.snapshot(Path::new("")).map(|_| [0; 32])
    }

    fn read_signing_input(&self, _index: &Path, _key: &Path, _root: &Path) -> Result<(Vec<u8>, [u8; 32]), CatalogError> {
        self.snapshot(Path::new("")).map(|_| (Vec::new(), [0; 32]))
    }

    fn write_signature(&self, _signature: &[u8], _index: &Path) -> Result<(), CatalogError> {
        self.snapshot(Path::new("")).map(|_| ())
    }

    fn source_files(&self, _source: &Path) -> Result<SourceSnapshot, CatalogError> {
        self.snapshot(Path::new("")).map(|_| SourceSnapshot::default())
    }

    fn write_proposals(&self, _talents: &[CatalogSource], _output: &Path, _source: &Path) -> Result<(), CatalogError> {
        self.snapshot(Path::new("")).map(|_| ())
    }
}
