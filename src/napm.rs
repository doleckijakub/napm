use alpm::{Alpm, Package, SigLevel, TransFlag, Usage};
use anyhow::{Result, anyhow};
use libarchive2::ReadArchive;
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

pub struct Pkg {
    pub name: String,
    pub version: String,
    pub db_name: String,
    pub desc: String,
}

impl From<&Package> for Pkg {
    fn from(package: &Package) -> Self {
        Self {
            name: package.name().to_string(),
            version: package.version().to_string(),
            db_name: package
                .db()
                .map(|db| db.name())
                .unwrap_or("local")
                .to_string(),
            desc: package.desc().unwrap_or("").to_string(),
        }
    }
}

pub struct Napm {
    handle: Option<Alpm>,
}

impl Napm {
    pub fn new() -> Result<Self> {
        let root = "./test-system";
        let dbpath = format!("{root}/var/lib/pacman"); // TODO: maybe change "pacman" to "napm"

        let mut handle = Alpm::new(root, &dbpath) //
            .map_err(|e| anyhow!("failed to initialize alpm: {e}"))?;

        handle.add_cachedir(format!("{root}/var/cache/pacman/pkg").as_str())?;

        // TODO: get from config
        let dbs = [
            (
                &[
                    "https://artix.sakamoto.pl/$repo/os/$arch",
                    "https://mirrors.dotsrc.org/artix-linux/repos/$repo/os/$arch",
                ][..],
                &["system", "world", "galaxy"][..],
            ),
            (
                &[
                    "https://arch.sakamoto.pl/$repo/os/$arch",
                    "https://mirror.pkgbuild.com/$repo/os/$arch",
                ][..],
                &["core", "extra", "multilib"][..],
            ),
        ];

        for (url_fmts, names) in &dbs {
            for &name in names.iter() {
                let db = handle.register_syncdb_mut(
                    name,
                    SigLevel::USE_DEFAULT | SigLevel::DATABASE_OPTIONAL,
                )?;

                for url_fmt in *url_fmts {
                    let url = url_fmt.replace("$repo", name).replace("$arch", "x86_64");
                    db.add_server(url)?;
                }

                db.set_usage(Usage::all())?;
            }
        }

        Ok(Self {
            handle: Some(handle),
        })
    }

    fn h(&self) -> &Alpm {
        self.handle.as_ref().unwrap()
    }

    fn h_mut(&mut self) -> &mut Alpm {
        self.handle.as_mut().unwrap()
    }

    fn cache_dir(&self) -> PathBuf {
        Path::new(self.h().root()).join("var/cache/pacman/files")
    }

    pub fn sync(&mut self, force: bool) -> Result<bool> {
        let handle = self.h_mut();

        handle
            .syncdbs_mut()
            .update(force)
            .map_err(|e| anyhow!("sync failed: {e}"))
    }

    pub fn install(&mut self, pkg_names: &[&str]) -> Result<()> {
        let handle = self.h_mut();

        handle
            .trans_init(TransFlag::NONE)
            .map_err(|e| anyhow!("failed to initialize transaction: {e}"))?;

        for pkg_name in pkg_names {
            let pkg = handle
                .syncdbs()
                .iter()
                .find_map(|db| db.pkg(*pkg_name).ok())
                .ok_or_else(|| anyhow!("package '{pkg_name}' not found"))?;

            handle
                .trans_add_pkg(pkg)
                .map_err(|e| anyhow!("failed to add package to transaction: {e}"))?;
        }

        handle
            .trans_prepare()
            .map_err(|e| anyhow!("failed to prepare transaction: {e}"))?;

        handle
            .trans_commit()
            .map_err(|e| anyhow!("failed to commit transaction: {e}"))?;

        Ok(())
    }

    pub fn update(&mut self) -> Option<Result<()>> {
        let h = self.h_mut();

        if let Err(e) = h.syncdbs_mut().update(false) {
            return Some(Err(anyhow!("failed to refresh dbs: {e}")));
        }

        if let Err(e) = h.trans_init(TransFlag::NONE) {
            return Some(Err(anyhow!("failed to initialize transaction: {e}")));
        }

        if let Err(e) = h.sync_sysupgrade(false) {
            return Some(Err(anyhow!("failed to upgrade: {e}")));
        }

        if let Err(e) = h.trans_prepare() {
            return Some(Err(anyhow!("failed to prepare transaction: {e}")));
        }

        match h.trans_commit() {
            Err(e) if e.to_string().contains("not prepared") => None,
            Err(e) => Some(Err(anyhow!("failed to commit transaction: {e}"))),
            _ => Some(Ok(())),
        }
    }

    pub fn remove(&mut self, names: &[&str], deep: bool) -> Result<()> {
        let h = self.h_mut();

        h.trans_init(if deep {
            TransFlag::RECURSE | TransFlag::CASCADE | TransFlag::NO_SAVE
        } else {
            TransFlag::NONE
        })?;

        for n in names {
            let pkg = h.localdb().pkg(*n)?;
            h.trans_remove_pkg(pkg)?;
        }

        h.trans_prepare() //
            .map_err(|e| anyhow!("failed to prepare transaction: {e}"))?;

        h.trans_commit() //
            .map_err(|e| anyhow!("failed to commit transaction: {e}"))?;

        Ok(())
    }

    pub fn search(&self, needles: &[&str]) -> Result<Vec<Pkg>> {
        let mut out = Vec::new();

        for db in self.h().syncdbs() {
            out.extend(db.search(needles.iter())?);
        }

        Ok(out.into_iter().map(Pkg::from).collect())
    }

    fn unarchive_files_db(
        archive_path: &std::path::Path,
        extract_to: &std::path::Path,
    ) -> anyhow::Result<()> {
        let mut archive = ReadArchive::open(archive_path)
            .map_err(|e| anyhow::anyhow!("failed to open archive: {}", e))?;

        if extract_to.exists() {
            std::fs::remove_dir_all(extract_to)?;
        }
        std::fs::create_dir_all(extract_to)?;

        while let Some(entry) = archive.next_entry()? {
            let entry_path = entry.pathname().unwrap_or_default().to_string();
            if entry_path.is_empty() || entry_path == "." {
                continue;
            }

            let mode = entry.mode();
            let ftype = entry.file_type();

            let full_path = extract_to.join(&entry_path);

            match ftype {
                libarchive2::FileType::Directory => {
                    std::fs::create_dir_all(&full_path)?;
                }
                libarchive2::FileType::RegularFile => {
                    if let Some(parent) = full_path.parent() {
                        std::fs::create_dir_all(parent)?;
                    }

                    let entry_data = archive
                        .read_data_to_vec()
                        .map_err(|e| anyhow::anyhow!("failed to read data: {}", e))?;

                    let mut writer = std::fs::File::create(&full_path)?;
                    writer.write_all(&entry_data)?;

                    #[cfg(unix)]
                    {
                        use std::os::unix::fs::PermissionsExt;
                        std::fs::set_permissions(
                            &full_path,
                            std::fs::Permissions::from_mode(mode),
                        )?;
                    }
                }
                _ => continue,
            }
        }

        Ok(())
    }

    pub fn query(&mut self, file: &str, fetch: bool) -> Result<Vec<(Pkg, String)>> {
        let cache_dir = self.cache_dir();

        if fetch {
            let h = self.h_mut();

            let db_path = Path::new(h.dbpath());
            let sync_dir = db_path.join("sync");

            if sync_dir.exists() {
                for entry in fs::read_dir(&sync_dir)? {
                    let entry = entry?;
                    let path = entry.path();

                    if let Some(filename) = path.file_name().and_then(|n| n.to_str())
                        && filename.ends_with(".files")
                    {
                        let db_name = filename.trim_end_matches(".files");
                        let db_cache_dir = cache_dir.join(db_name);

                        let should_update = if db_cache_dir.exists() {
                            let sync_mtime = std::fs::metadata(&path)?.modified()?;
                            let cache_mtime = std::fs::metadata(&db_cache_dir)?.modified()?;
                            sync_mtime > cache_mtime
                        } else {
                            true
                        };

                        if should_update {
                            std::fs::create_dir_all(&db_cache_dir)?;

                            Self::unarchive_files_db(&path, &db_cache_dir)
                                .map_err(|e| anyhow!("failed to unarchive {}: {}", filename, e))?;
                        }
                    }
                }
            }

            h.set_dbext(".files");
            h.syncdbs_mut()
                .update(false)
                .map_err(|e| anyhow!("failed to refresh dbs: {e}"))?;
        }

        let mut out = Vec::new();

        for db_entry in std::fs::read_dir(&cache_dir)? {
            let db_entry = db_entry?;
            let db_cache_dir = db_entry.path();

            if !db_cache_dir.is_dir() {
                continue;
            }

            let db_name = db_entry.file_name().to_string_lossy().to_string();

            for pkg_entry in std::fs::read_dir(&db_cache_dir)? {
                let pkg_entry = pkg_entry?;
                let pkg_path = pkg_entry.path();

                if !pkg_path.is_dir() {
                    continue;
                }

                let desc_path = pkg_path.join("desc");

                let mut pkg_name = String::new();
                let mut pkg_version = String::new();
                let mut pkg_desc = String::new();

                if desc_path.exists() {
                    let content = std::fs::read_to_string(&desc_path)?;
                    let mut current_key: Option<&str> = None;

                    for line in content.lines() {
                        let line = line.trim();
                        if line.is_empty() {
                            continue;
                        }

                        if line.starts_with('%') && line.ends_with('%') {
                            current_key = Some(line.trim_matches('%'));
                            continue;
                        }

                        match current_key {
                            Some("NAME") => pkg_name = line.to_string(),
                            Some("VERSION") => pkg_version = line.to_string(),
                            Some("DESC") => {
                                if pkg_desc.is_empty() {
                                    pkg_desc = line.to_string();
                                } else {
                                    pkg_desc.push(' ');
                                    pkg_desc.push_str(line);
                                }
                            }
                            _ => {}
                        }
                    }
                } else {
                    let dir_name = pkg_entry.file_name().to_string_lossy().to_string();
                    let mut parts = dir_name.rsplitn(2, '-');
                    pkg_version = parts.next().unwrap_or("").to_string();
                    pkg_name = parts.next().unwrap_or(&dir_name).to_string();
                }

                let files_path = pkg_path.join("files");
                if !files_path.exists() {
                    continue;
                }

                let files_content = std::fs::read_to_string(&files_path)?;
                for line in files_content.lines() {
                    if line.starts_with('%') || line.trim().is_empty() {
                        continue;
                    }

                    if line.ends_with(&format!("/{file}")) {
                        out.push((
                            Pkg {
                                name: pkg_name.clone(),
                                version: pkg_version.clone(),
                                db_name: db_name.clone(),
                                desc: pkg_desc.clone(),
                            },
                            line.to_string(),
                        ));
                    }
                }
            }
        }

        Ok(out)
    }

    pub fn info(&self, name: &str) -> Result<Pkg> {
        let local_pkg = self.h().localdb().pkg(name);

        if let Ok(pkg) = local_pkg {
            return Ok(Pkg::from(pkg));
        }

        unimplemented!("non-local info");
    }

    pub fn list(&self) -> Vec<Pkg> {
        self.h()
            .localdb()
            .pkgs()
            .into_iter()
            .map(Pkg::from)
            .collect()
    }

    pub fn files(&self, name: &str) -> Result<Vec<String>> {
        let local_pkg = self.h().localdb().pkg(name);

        if let Ok(pkg) = local_pkg {
            return Ok(pkg
                .files()
                .files()
                .iter()
                .map(|f| f.name().to_owned())
                .collect());
        }

        unimplemented!("non-local files");
    }
}

impl Drop for Napm {
    fn drop(&mut self) {
        if let Some(h) = self.handle.take() {
            let _ = h.unlock();
            let _ = h.release();
        }
    }
}
