use std::{fs, path::Path};

use serde_json::{Map, Value, json};

use crate::Error;

/// Load a JSON or JSONC object from `path`, returning an empty object when absent.
///
/// # Errors
///
/// Returns an error when the file cannot be read or is not a JSON object.
pub fn load_object(path: &Path) -> Result<Value, Error> {
    if !path.exists() {
        return Ok(json!({}));
    }
    let text = fs::read_to_string(path).map_err(|source| Error::Read {
        path: path.display().to_string(),
        source,
    })?;
    if text.trim().is_empty() {
        return Ok(json!({}));
    }
    let value: Value =
        serde_json::from_str(&strip_comments(&text)).map_err(|source| Error::InvalidJson {
            path: path.display().to_string(),
            source,
        })?;
    if !value.is_object() {
        return Err(Error::ConfigRootNotObject {
            path: path.display().to_string(),
        });
    }
    Ok(value)
}

/// Ensure every key in `keys` points to an object, creating missing objects.
///
/// # Errors
///
/// Returns an error when an existing path segment is not an object.
pub fn ensure_object_path<'a>(
    root: &'a mut Value,
    keys: &[&str],
    path: &Path,
) -> Result<&'a mut Map<String, Value>, Error> {
    let mut current = root;
    for (index, key) in keys.iter().enumerate() {
        let object = current
            .as_object_mut()
            .ok_or_else(|| Error::InvalidConfigField {
                path: path.display().to_string(),
                field: keys[..index].join("."),
            })?;
        let value = object.entry((*key).to_owned()).or_insert_with(|| json!({}));
        if !value.is_object() {
            return Err(Error::InvalidConfigField {
                path: path.display().to_string(),
                field: keys[..=index].join("."),
            });
        }
        current = value;
    }
    current
        .as_object_mut()
        .ok_or_else(|| Error::InvalidConfigField {
            path: path.display().to_string(),
            field: keys.join("."),
        })
}

/// Copy an existing configuration to its `.bak` sibling.
///
/// # Errors
///
/// Returns an error when the backup cannot be created.
pub fn backup(path: &Path) -> Result<(), Error> {
    if !path.exists() {
        return Ok(());
    }
    let backup = format!("{}.bak", path.display());
    fs::copy(path, &backup).map_err(|source| Error::Backup {
        path: backup,
        source,
    })?;
    Ok(())
}

/// Serialize and write a JSON object, creating parent directories as needed.
///
/// # Errors
///
/// Returns an error when serialization or filesystem I/O fails.
pub fn write_object(path: &Path, root: &Value) -> Result<(), Error> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|source| Error::Write {
            path: parent.display().to_string(),
            source,
        })?;
    }
    let rendered = serde_json::to_string_pretty(root).map_err(|source| Error::InvalidJson {
        path: path.display().to_string(),
        source,
    })?;
    fs::write(path, format!("{rendered}\n")).map_err(|source| Error::Write {
        path: path.display().to_string(),
        source,
    })
}

#[must_use]
pub fn strip_comments(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();
    let mut in_string = false;
    let mut escaped = false;
    let mut line_comment = false;
    let mut block_comment = false;

    while let Some(character) = chars.next() {
        if line_comment {
            if character == '\n' {
                line_comment = false;
                output.push(character);
            }
            continue;
        }
        if block_comment {
            if character == '*' && chars.peek() == Some(&'/') {
                chars.next();
                block_comment = false;
                output.push(' ');
            } else if character == '\n' {
                output.push('\n');
            }
            continue;
        }
        if in_string {
            output.push(character);
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == '"' {
                in_string = false;
            }
            continue;
        }
        if character == '"' {
            in_string = true;
            output.push(character);
        } else if character == '/' && chars.peek() == Some(&'/') {
            chars.next();
            line_comment = true;
        } else if character == '/' && chars.peek() == Some(&'*') {
            chars.next();
            block_comment = true;
        } else {
            output.push(character);
        }
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn comments_are_removed_without_touching_urls() {
        let input = "{\n  // comment\n  \"url\": \"https://example.test/a\" /* tail */\n}";
        let parsed: Value = serde_json::from_str(&strip_comments(input)).expect("JSONC");
        assert_eq!(parsed["url"], "https://example.test/a");
    }

    #[test]
    fn nested_object_path_is_created() {
        let mut root = json!({"keep": true});
        let path = Path::new("config.json");
        let map = ensure_object_path(&mut root, &["mcp", "servers"], path).expect("path");
        map.insert("demo".to_owned(), json!({"command": "demo"}));
        assert_eq!(root["keep"], true);
        assert_eq!(root["mcp"]["servers"]["demo"]["command"], "demo");
    }
}
