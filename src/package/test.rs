use super::*;
use std::fs;
use tempfile::tempdir;
use zip::{CompressionMethod, ZipWriter, write::SimpleFileOptions};

fn write_zip(path: &Path, entries: &[(&str, &[u8])]) {
    let file = fs::File::create(path).unwrap();
    let mut zip = ZipWriter::new(file);
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);

    for (name, bytes) in entries {
        zip.start_file(name, options).unwrap();
        use std::io::Write;
        zip.write_all(bytes).unwrap();
    }

    zip.finish().unwrap();
}

#[test]
fn open_errors_when_archive_contains_unsafe_path() {
    let dir = tempdir().unwrap();
    let package_path = dir.path().join("unsafe.marmot");

    write_zip(
        &package_path,
        &[
            ("../escape.txt", b"oops"),
            ("template.psl", b"%!PSL 0.1\npage 10 10\ndraw begin\nend\n"),
        ],
    );

    let err = match MarmotPackage::open(&package_path) {
        Ok(_) => panic!("expected open to fail for unsafe archive path"),
        Err(err) => err.to_string(),
    };
    assert!(err.contains("archive contains unsafe path"));
}

#[test]
fn open_errors_when_template_is_missing() {
    let dir = tempdir().unwrap();
    let package_path = dir.path().join("missing-template.marmot");

    write_zip(&package_path, &[("assets/logo.png", b"png")]);

    let err = match MarmotPackage::open(&package_path) {
        Ok(_) => panic!("expected open to fail for missing template"),
        Err(err) => err.to_string(),
    };
    assert!(err.contains("package is missing template.psl"));
}

#[test]
fn create_package_errors_when_asset_filenames_collide() {
    let dir = tempdir().unwrap();
    let a_dir = dir.path().join("a");
    let b_dir = dir.path().join("b");
    fs::create_dir_all(&a_dir).unwrap();
    fs::create_dir_all(&b_dir).unwrap();

    let template = dir.path().join("template.psl");
    fs::write(&template, "%!PSL 0.1\npage 10 10\ndraw begin\nend\n").unwrap();

    let a_logo = a_dir.join("logo.png");
    let b_logo = b_dir.join("logo.png");
    fs::write(&a_logo, b"a").unwrap();
    fs::write(&b_logo, b"b").unwrap();

    let output = dir.path().join("out.marmot");
    let options = PackageBuilderOptions {
        template_file: template,
        output_file: output,
        assets: vec![a_logo, b_logo],
        fonts: vec![],
        scripts: vec![],
        remap_file: None,
    };

    let err = create_package(options).unwrap_err().to_string();
    assert!(err.contains("duplicate package entry: assets/logo.png"));
}

#[test]
fn creates_and_opens_package_with_resolvable_entries() {
    let dir = tempdir().unwrap();
    let template = dir.path().join("template.psl");
    let font = dir.path().join("font.ttf");
    let asset = dir.path().join("logo.png");
    let output = dir.path().join("bundle.marmot");

    fs::write(&template, "%!PSL 0.1\npage 10 10\ndraw begin\nend\n").unwrap();
    fs::write(&font, b"fake-font").unwrap();
    fs::write(&asset, b"fake-image").unwrap();

    create_package(PackageBuilderOptions {
        template_file: template,
        output_file: output.clone(),
        assets: vec![asset],
        fonts: vec![font],
        scripts: vec![],
        remap_file: None,
    })
    .unwrap();

    let package = MarmotPackage::open(&output).unwrap();

    assert!(package.resolve_path("template.psl").unwrap().is_file());
    assert!(package.resolve_path("fonts/font.ttf").unwrap().is_file());
    assert!(package.resolve_path("assets/logo.png").unwrap().is_file());

    let err = package.resolve_path("../escape").unwrap_err().to_string();
    assert!(err.contains("invalid package path"));
}

#[test]
fn create_package_includes_remap_file_when_provided() {
    let dir = tempdir().unwrap();
    let template = dir.path().join("template.psl");
    let remap = dir.path().join("remap.plt");
    let output = dir.path().join("bundle.marmot");

    fs::write(&template, "%!PSL 0.1\npage 10 10\ndraw begin\nend\n").unwrap();
    fs::write(&remap, "FFFFFF\n000000\nFF0000\n").unwrap();

    create_package(PackageBuilderOptions {
        template_file: template,
        output_file: output.clone(),
        assets: vec![],
        fonts: vec![],
        scripts: vec![],
        remap_file: Some(remap),
    })
    .unwrap();

    let package = MarmotPackage::open(&output).unwrap();
    let remap_path = package.resolve_path("remap.plt").unwrap();
    assert!(remap_path.is_file());
    let remap_contents = fs::read_to_string(remap_path).unwrap();
    assert!(remap_contents.contains("FFFFFF"));
    assert!(remap_contents.contains("000000"));
}
