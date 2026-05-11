use anyhow::{Context, Result, bail};
use std::{
    fs::{File, create_dir_all, read_to_string}, io, path::{Path, PathBuf}
};
use zip::ZipArchive;

use tempfile::TempDir;

use crate::package;

pub struct MarmotPackage {
    temp_dir: TempDir,
    root: PathBuf,
}

impl MarmotPackage {
    pub fn open(path: &Path) -> Result<Self> {
        Self::ensure_marmot_archive_path(path)?;
        let temp_dir =
            tempfile::tempdir().context("failed to create temporary package directory")?;
        Self::unpack_archive(path, temp_dir.path())?;

        let root = temp_dir.path().to_path_buf();

        let package = Self { temp_dir, root };
        package.ensure_template_exists()?;

        Ok(package)
    }

    pub fn template_path(&self) -> PathBuf {
        self.root.join("template.psl")
    }

    pub fn read_template_source(&self) -> Result<String> {
        read_to_string(self.template_path())
            .with_context(|| format!("failed to read {}", self.template_path().display()))
    }

    fn ensure_template_exists(&self) -> Result<()> {
        let template_path = self.template_path();

        if !template_path.exists() {
            bail!("package is missing template.psl");
        }
        if !template_path.is_file() {
            bail!("package entry template.psl is not a file");
        }

        Ok(())
    }

    fn ensure_marmot_archive_path(path: &Path) -> Result<()> {
        if !path.exists() {
            bail!("package does not exist: {}", path.display());
        }

        if !path.is_file() {
            bail!("package path is not a file: {}", path.display());
        }

        if path.extension().and_then(|ext| ext.to_str()) != Some("marmot") {
            bail!("package must end with .marmot: {}", path.display());
        }

        Ok(())
    }

    fn unpack_archive(package_path: &Path, dest: &Path) -> Result<()> {
        let file = File::open(package_path)
            .with_context(|| format!("failed to open package: {}", package_path.display()))?;

        let mut archive = ZipArchive::new(file).with_context(
            || format!("failed to read package archive: {}", package_path.display()),
        )?;

        for index in 0..archive.len() {
            let mut entry = archive
                .by_index(index)
                .with_context(|| format!("failed to ready archive entry {index}"))?;

            let Some(enclosed_name) = entry.enclosed_name() else {
                bail!("archive contains unsafe path: {}", entry.name());
            };

            let output_path = dest.join(enclosed_name);

            if entry.is_dir() {
                create_dir_all(&output_path)
                    .with_context(|| format!("failed to create directory: {}", output_path.display()))?;
                continue;
            }

            if let Some(parent) = output_path.parent() {
                create_dir_all(parent)
                    .with_context(|| format!("failed to create directory: {}", output_path.display()))?;
            }

            let mut output_file = File::create(&output_path)
                .with_context(|| format!("failed to create file: {}", output_path.display()))?;

            io::copy(&mut entry, &mut output_file)
                .with_context(|| format!("failed to extract file: {}", output_path.display()))?;
        }

        Ok(())
    }
}

