use anyhow::{Context, Result, bail};
use std::{
    collections::HashSet,
    fs::{File, create_dir_all, read_to_string},
    io,
    path::{Component, Path, PathBuf},
};
use zip::{CompressionMethod, ZipArchive, ZipWriter, write::SimpleFileOptions};

use crate::ensure_file_exists;

pub struct MarmotPackage {
    root: PathBuf,
}

impl MarmotPackage {
    pub fn open(path: &Path) -> Result<Self> {
        Self::ensure_marmot_archive_path(path)?;
        let temp_dir =
            tempfile::tempdir().context("failed to create temporary package directory")?;
        Self::unpack_archive(path, temp_dir.path())?;

        let root = temp_dir.path().to_path_buf();

        let package = Self { root };
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

    pub fn resolve_path(&self, package_relative_path: &str) -> Result<PathBuf> {
        let relative = Path::new(package_relative_path);

        for component in relative.components() {
            match component {
                Component::Normal(_) => {}
                _ => bail!("invalid package path: {package_relative_path}"),
            }
        }

        let resolved = self.root.join(relative);

        if !resolved.exists() {
            bail!("package file does not exist: {package_relative_path}");
        }

        if !resolved.is_file() {
            bail!("package path is not a file: {package_relative_path}");
        }

        Ok(resolved)
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

        let mut archive = ZipArchive::new(file).with_context(|| {
            format!("failed to read package archive: {}", package_path.display())
        })?;

        for index in 0..archive.len() {
            let mut entry = archive
                .by_index(index)
                .with_context(|| format!("failed to ready archive entry {index}"))?;

            let Some(enclosed_name) = entry.enclosed_name() else {
                bail!("archive contains unsafe path: {}", entry.name());
            };

            let output_path = dest.join(enclosed_name);

            if entry.is_dir() {
                create_dir_all(&output_path).with_context(|| {
                    format!("failed to create directory: {}", output_path.display())
                })?;
                continue;
            }

            if let Some(parent) = output_path.parent() {
                create_dir_all(parent).with_context(|| {
                    format!("failed to create directory: {}", output_path.display())
                })?;
            }

            let mut output_file = File::create(&output_path)
                .with_context(|| format!("failed to create file: {}", output_path.display()))?;

            io::copy(&mut entry, &mut output_file)
                .with_context(|| format!("failed to extract file: {}", output_path.display()))?;
        }

        Ok(())
    }
}

pub struct PackageBuilderOptions {
    pub template_file: PathBuf,
    pub output_file: PathBuf,
    pub assets: Vec<PathBuf>,
    pub fonts: Vec<PathBuf>,
}

pub fn create_package(options: PackageBuilderOptions) -> Result<()> {
    let mut archive_paths = HashSet::new();

    validate_package_build_options(&options)?;

    let output = File::create(&options.output_file).with_context(|| {
        format!(
            "failed to create package: {}",
            options.output_file.display()
        )
    })?;

    let mut zip = ZipWriter::new(output);

    add_unique_file(
        &mut zip,
        &mut archive_paths,
        &options.template_file,
        "template.psl",
    )?;

    for font in &options.fonts {
        let filename = filename_string(font)?;
        let archive_path = format!("fonts/{filename}");
        add_unique_file(&mut zip, &mut archive_paths, font, &archive_path)?;
    }

    for asset in &options.assets {
        let filename = filename_string(asset)?;
        let archive_path = format!("assets/{filename}");
        add_unique_file(&mut zip, &mut archive_paths, asset, &archive_path)?;
    }

    zip.finish().with_context(|| {
        format!(
            "failed to finish package: {}",
            options.output_file.display()
        )
    })?;

    Ok(())
}

fn filename_string(path: &Path) -> Result<String> {
    let filename = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| anyhow::anyhow!("invalid filename: {}", path.display()))?;
    Ok(filename.to_string())
}

fn validate_package_build_options(options: &PackageBuilderOptions) -> Result<()> {
    ensure_file_exists(&options.template_file)?;

    if options.output_file.extension().and_then(|ext| ext.to_str()) != Some("marmot") {
        bail!(
            "package output must end with .marmot: {}",
            options.output_file.display()
        );
    }

    // TODO: figure out if we should allow overwriting packages
    if options.output_file.exists() {
        bail!("package already exists: {}", options.output_file.display());
    }

    if let Some(parent) = options.output_file.parent() {
        if !parent.as_os_str().is_empty() && !parent.is_dir() {
            bail!("output directory does not exist: {}", parent.display());
        }
    }

    for font in &options.fonts {
        ensure_file_exists(font)?;
    }

    for asset in &options.assets {
        ensure_file_exists(asset)?;
    }

    Ok(())
}

fn add_unique_file(
    zip: &mut ZipWriter<File>,
    archive_paths: &mut HashSet<String>,
    source_path: &Path,
    archive_path: &str,
) -> Result<()> {
    if !archive_paths.insert(archive_path.to_string()) {
        bail!("duplicate package entry: {archive_path}");
    }

    add_file_to_zip(zip, source_path, archive_path)
}

fn add_file_to_zip(
    zip: &mut ZipWriter<File>,
    source_path: &Path,
    archive_path: &str,
) -> Result<()> {
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);

    zip.start_file(archive_path, options)
        .with_context(|| format!("failed to start archive entry: {archive_path}"))?;

    let mut input = File::open(source_path)
        .with_context(|| format!("failed to open file: {}", source_path.display()))?;

    io::copy(&mut input, zip)
        .with_context(|| format!("failed to write archive entry: {archive_path}"))?;

    Ok(())
}
