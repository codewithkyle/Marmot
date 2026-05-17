use anyhow::{Context, Result, anyhow, bail};
use serde_json::Value;
use std::{
    fs::{create_dir_all},
    path::{Path},
    time::{Duration},
};

use crate::RenderStats;

pub fn format_duration(d: Duration) -> String {
    let seconds = d.as_secs_f64();

    if seconds >= 60.0 {
        format!("{:.3} min", seconds / 60.0)
    } else if seconds >= 1.0 {
        format!("{:.3} s", seconds)
    } else {
        format!("{:.3} ms", seconds * 1000.0)
    }
}

pub fn summarize_render_times(render_times: &[Duration]) -> Option<RenderStats> {
    if render_times.is_empty() {
        return None;
    }

    let mut sorted = render_times.to_vec();
    sorted.sort_unstable();

    let min = sorted[0];
    let max = sorted[sorted.len() - 1];

    let total_nanos: u128 = sorted.iter().map(Duration::as_nanos).sum();
    let avg_nanos = total_nanos / sorted.len() as u128;
    let avg = Duration::from_nanos(avg_nanos.min(u64::MAX as u128) as u64);

    Some(RenderStats {
        avg,
        min,
        max,
        p90: percentile_duration(&sorted, 0.90),
        p95: percentile_duration(&sorted, 0.95),
        p99: percentile_duration(&sorted, 0.99),
        p999: percentile_duration(&sorted, 0.999),
    })
}

pub fn percentile_duration(sorted: &[Duration], percentile: f64) -> Duration {
    let n = sorted.len();
    if n == 0 {
        return Duration::from_nanos(0);
    }

    let rank = (percentile.clamp(0.0, 1.0) * n as f64).ceil() as usize;
    let index = rank.saturating_sub(1).min(n - 1);
    sorted[index]
}

pub fn ensure_file_exists(path: &Path) -> Result<()> {
    if !path.exists() {
        bail!("file does not exist: {}", path.display());
    }
    if !path.is_file() {
        bail!("path is not a file: {}", path.display());
    }
    Ok(())
}

pub fn ensure_dir_exists(path: &Path) -> Result<()> {
    if !path.exists() {
        create_dir_all(path)
            .with_context(|| format!("failed to create directory: {}", path.display()))?;
        return Ok(());
    }

    if !path.is_dir() {
        bail!("path is not a directory: {}", path.display());
    }

    Ok(())
}

pub fn ensure_parent_exists(path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() && !parent.exists() {
            create_dir_all(parent)
                .with_context(|| format!("failed to create directory: {}", parent.display()))?;
            return Ok(());
        }
        if !parent.as_os_str().is_empty() && !parent.is_dir() {
            bail!("output parent is not a directory: {}", parent.display());
        }
    }
    Ok(())
}

pub fn value_to_filename_fragment(value: &Value) -> Option<String> {
    match value {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        Value::Bool(b) => Some(b.to_string()),
        _ => None,
    }
}

pub fn sanitize_filename(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    for ch in name.chars() {
        let bad = matches!(
            ch,
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' | '\0'
        );
        if bad || ch.is_control() {
            out.push('_');
        } else {
            out.push(ch);
        }
    }

    let out = out.trim();
    if out.is_empty() {
        "_".to_string()
    } else {
        out.to_string()
    }
}

pub fn format_output_name(template: &str, record: &Value, index: usize) -> Result<String> {
    let mut out = String::with_capacity(template.len() + 16);
    let mut cursor = 0usize;

    while let Some(open_rel) = template[cursor..].find('{') {
        let open = cursor + open_rel;
        out.push_str(&template[cursor..open]);

        let after_open = open + 1;
        if let Some(close_rel) = template[after_open..].find('}') {
            let close = after_open + close_rel;
            let key = &template[after_open..close];

            let replacement = if key == "index" {
                index.to_string()
            } else {
                let value = record.get(key).ok_or_else(|| {
                    anyhow!("record missing field '{}' required by output template", key)
                })?;
                value_to_filename_fragment(value).ok_or_else(|| {
                    anyhow!(
                        "record field '{}' must be string/number/bool for output template",
                        key
                    )
                })?
            };

            out.push_str(&replacement);
            cursor = close + 1;
        } else {
            out.push_str(&template[open..]);
            cursor = template.len();
            break;
        }
    }

    if cursor < template.len() {
        out.push_str(&template[cursor..]);
    }

    let out = sanitize_filename(&out);

    if out.contains("..") {
        bail!("output file contains unsafe '..': {}", out);
    }

    Ok(out)
}
