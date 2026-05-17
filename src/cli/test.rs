use crate::{ops::common::load_remap_palette_if_needed, package::{MarmotPackage, PackageBuilderOptions, create_package}, util::format_output_name};

use super::*;
use serde_json::json;
use std::fs;
use tempfile::tempdir;
#[test]
fn output_name_supports_single_top_level_field() {
    let record = json!({ "sku": "49000000001" });
    let out = format_output_name("{sku}.pdf", &record, 1).unwrap();
    assert_eq!(out, "49000000001.pdf");
}
#[test]
fn output_name_supports_many_fields() {
    let record = json!({
        "sku": "49000000001",
        "buy_qty": 1,
        "get_qty": 2
    });
    let out = format_output_name("{index}-{sku}-{buy_qty}-{get_qty}.pdf", &record, 7).unwrap();
    assert_eq!(out, "7-49000000001-1-2.pdf");
}
#[test]
fn output_name_supports_repeated_fields() {
    let record = json!({ "sku": "ABC123" });
    let out = format_output_name("{sku}-{sku}.pdf", &record, 1).unwrap();
    assert_eq!(out, "ABC123-ABC123.pdf");
}
#[test]
fn output_name_errors_when_field_missing() {
    let record = json!({ "id": "x" });
    let err = format_output_name("{sku}.pdf", &record, 1)
        .unwrap_err()
        .to_string();
    assert!(err.contains("record missing field 'sku'"));
}
#[test]
fn output_name_errors_when_field_not_scalar() {
    let record = json!({ "sku": { "nested": "x" } });
    let err = format_output_name("{sku}.pdf", &record, 1)
        .unwrap_err()
        .to_string();
    assert!(err.contains("record field 'sku' must be string/number/bool"));
}
#[test]
fn output_name_keeps_malformed_open_brace_literal() {
    let record = json!({ "sku": "49000000001" });
    let out = format_output_name("prefix-{sku.pdf", &record, 1).unwrap();
    assert_eq!(out, "prefix-{sku.pdf");
}
#[test]
fn output_name_sanitizes_invalid_filename_chars() {
    let record = json!({ "sku": "49/000:000?01" });
    let out = format_output_name("{sku}.pdf", &record, 1).unwrap();
    assert_eq!(out, "49_000_000_01.pdf");
}
#[test]
fn output_name_rejects_dotdot_segments() {
    let record = json!({ "sku": ".." });
    let err = format_output_name("{sku}.pdf", &record, 1)
        .unwrap_err()
        .to_string();
    assert!(err.contains("unsafe '..'"));
}

#[test]
fn dither_type_parsing_accepts_supported_values() {
    assert!(DitherType::try_from_word("floyd").is_ok());
    assert!(DitherType::try_from_word("atkinson").is_ok());
    assert!(DitherType::try_from_word("stucki").is_ok());
    assert!(DitherType::try_from_word("burkes").is_ok());
    assert!(DitherType::try_from_word("jarvis").is_ok());
    assert!(DitherType::try_from_word("sierra3").is_ok());
}

#[test]
fn dither_type_parsing_rejects_unknown_value() {
    let err = match DitherType::try_from_word("ordered") {
        Ok(_) => panic!("expected invalid dither type error"),
        Err(err) => err.to_string(),
    };
    assert!(err.contains("invalid dither type"));
}

#[test]
fn load_remap_returns_none_when_dither_is_not_requested() {
    let dir = tempdir().unwrap();
    let template = dir.path().join("template.psl");
    let package_file = dir.path().join("no-dither.marmot");
    fs::write(&template, "%!PSL 0.1\npage 10 10\ndraw begin\nend\n").unwrap();

    create_package(PackageBuilderOptions {
        template_file: template,
        output_file: package_file.clone(),
        assets: vec![],
        fonts: vec![],
        scripts: vec![],
        remap_file: None,
    })
    .unwrap();

    let pkg = MarmotPackage::open(&package_file).unwrap();
    let remap = load_remap_palette_if_needed(&pkg, None).unwrap();
    assert!(remap.is_none());
}

#[test]
fn load_remap_errors_when_dither_is_requested_but_package_has_no_remap() {
    let dir = tempdir().unwrap();
    let template = dir.path().join("template.psl");
    let package_file = dir.path().join("missing-remap.marmot");
    fs::write(&template, "%!PSL 0.1\npage 10 10\ndraw begin\nend\n").unwrap();

    create_package(PackageBuilderOptions {
        template_file: template,
        output_file: package_file.clone(),
        assets: vec![],
        fonts: vec![],
        scripts: vec![],
        remap_file: None,
    })
    .unwrap();

    let pkg = MarmotPackage::open(&package_file).unwrap();
    let err = load_remap_palette_if_needed(&pkg, Some(DitherType::Floyd))
        .unwrap_err()
        .to_string();
    assert!(err.contains("--dither requires remap.plt in package"));
}

#[test]
fn load_remap_reads_palette_when_dither_is_requested() {
    let dir = tempdir().unwrap();
    let template = dir.path().join("template.psl");
    let remap = dir.path().join("remap.plt");
    let package_file = dir.path().join("has-remap.marmot");
    fs::write(&template, "%!PSL 0.1\npage 10 10\ndraw begin\nend\n").unwrap();
    fs::write(&remap, "FFFFFF\n000000\nFF0000\n").unwrap();

    create_package(PackageBuilderOptions {
        template_file: template,
        output_file: package_file.clone(),
        assets: vec![],
        fonts: vec![],
        scripts: vec![],
        remap_file: Some(remap),
    })
    .unwrap();

    let pkg = MarmotPackage::open(&package_file).unwrap();
    let remap = load_remap_palette_if_needed(&pkg, Some(DitherType::Floyd)).unwrap();
    let remap = remap.expect("expected remap source when dither is enabled");
    assert!(remap.contains("FFFFFF"));
    assert!(remap.contains("000000"));
}
