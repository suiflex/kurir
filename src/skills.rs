use std::{
    env, fs,
    path::{Path, PathBuf},
};

use crate::{
    Error, Harness, Scope,
    model::{RegistrationOptions, RegistrationResult},
};

/// Directory a harness loads skills from, relative to the project root for
/// [`Scope::Project`] or the home directory for [`Scope::User`]. Install a skill
/// as a subdirectory of it.
///
/// `None` means the harness documents no skill directory for that scope.
#[must_use]
pub const fn skills_dir(harness: Harness, scope: Scope) -> Option<&'static str> {
    match (harness, scope) {
        (Harness::ClaudeCode | Harness::ClaudeCodeCli, Scope::Project | Scope::User) => {
            Some(".claude/skills")
        }
        // Codex and Cursor read the shared `.agents/skills` in both scopes.
        (Harness::Codex | Harness::Cursor, Scope::Project | Scope::User)
        | (
            Harness::OpenCode | Harness::Hermes | Harness::OpenClaw | Harness::AntigravityCli,
            Scope::Project,
        ) => Some(".agents/skills"),
        (Harness::OpenCode, Scope::User) => Some(".config/opencode/skills"),
        (Harness::Hermes, Scope::User) => Some(".hermes/skills"),
        (Harness::OpenClaw, Scope::User) => Some(".openclaw/skills"),
        (Harness::AntigravityCli, Scope::User) => Some(".gemini/antigravity-cli/skills"),
        _ => None,
    }
}

/// Resolve the skills directory path for a harness and scope.
///
/// # Errors
///
/// Returns an error if the harness does not support skills for the scope or
/// target path cannot be resolved.
pub fn target_skills_dir(
    harness: Harness,
    options: &RegistrationOptions,
) -> Result<PathBuf, Error> {
    if let Some(path) = &options.config {
        return Ok(path.clone());
    }
    let relative = skills_dir(harness, options.scope).ok_or_else(|| Error::Unsupported {
        harness: harness.id().to_owned(),
        detail: format!("skills directory for {:?} scope", options.scope),
    })?;
    match options.scope {
        Scope::Project => Ok(options.cwd.join(relative)),
        Scope::User => {
            let home = env::home_dir().ok_or(Error::MissingArgument("home directory"))?;
            Ok(home.join(relative))
        }
        Scope::Local => Err(Error::Unsupported {
            harness: harness.id().to_owned(),
            detail: "local scope for skills".to_owned(),
        }),
    }
}

/// Install a skill directory into the target harness skills location.
///
/// Recursively copies the skill folder, backs up existing installations before
/// overwriting if requested, and prevents accidental conflicts.
///
/// # Errors
///
/// Returns an error when skill source is invalid, harness is unsupported,
/// or filesystem operations fail.
pub fn install_skill(
    harness: Harness,
    source_path: &Path,
    options: &RegistrationOptions,
) -> Result<RegistrationResult, Error> {
    if !source_path.exists() || !source_path.is_dir() {
        return Err(Error::InvalidArgument {
            field: "skill source path",
            value: source_path.display().to_string(),
            expected: "an existing skill directory",
        });
    }

    let skill_name = source_path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| Error::InvalidArgument {
            field: "skill directory name",
            value: source_path.display().to_string(),
            expected: "a valid directory name",
        })?
        .to_owned();

    let target_parent = target_skills_dir(harness, options)?;
    let target_dir = target_parent.join(&skill_name);

    if target_dir.exists() {
        if is_skill_identical(source_path, &target_dir) {
            return Ok(RegistrationResult {
                harness: harness.id().to_owned(),
                name: skill_name,
                target: Some(target_dir),
                changed: false,
                action: "already-configured".to_owned(),
            });
        }
        if !options.force {
            return Err(Error::Conflict { name: skill_name });
        }
    }

    if options.print {
        println!("Install skill '{}' -> {}", skill_name, target_dir.display());
    }

    if options.dry_run {
        return Ok(RegistrationResult {
            harness: harness.id().to_owned(),
            name: skill_name,
            target: Some(target_dir),
            changed: false,
            action: "dry-run".to_owned(),
        });
    }

    if target_dir.exists() {
        let backup_dir = target_parent.join(format!("{skill_name}.bak"));
        if backup_dir.exists() {
            fs::remove_dir_all(&backup_dir).map_err(|source| Error::Backup {
                path: backup_dir.display().to_string(),
                source,
            })?;
        }
        fs::rename(&target_dir, &backup_dir).map_err(|source| Error::Backup {
            path: target_dir.display().to_string(),
            source,
        })?;
    }

    copy_dir_recursive(source_path, &target_dir)?;

    Ok(RegistrationResult {
        harness: harness.id().to_owned(),
        name: skill_name,
        target: Some(target_dir),
        changed: true,
        action: "installed".to_owned(),
    })
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), Error> {
    fs::create_dir_all(dst).map_err(|source| Error::Write {
        path: dst.display().to_string(),
        source,
    })?;

    let entries = fs::read_dir(src).map_err(|source| Error::Read {
        path: src.display().to_string(),
        source,
    })?;

    for entry in entries {
        let entry = entry.map_err(|source| Error::Read {
            path: src.display().to_string(),
            source,
        })?;
        let entry_path = entry.path();
        let target_path = dst.join(entry.file_name());

        if entry_path.is_dir() {
            copy_dir_recursive(&entry_path, &target_path)?;
        } else {
            fs::copy(&entry_path, &target_path).map_err(|source| Error::Write {
                path: target_path.display().to_string(),
                source,
            })?;
        }
    }
    Ok(())
}

fn is_skill_identical(src: &Path, dst: &Path) -> bool {
    let skill_md_src = src.join("SKILL.md");
    let skill_md_dst = dst.join("SKILL.md");
    if !skill_md_src.exists() || !skill_md_dst.exists() {
        return false;
    }
    match (
        fs::read_to_string(&skill_md_src),
        fs::read_to_string(&skill_md_dst),
    ) {
        (Ok(content_src), Ok(content_dst)) => content_src == content_dst,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skill_directories_follow_each_harness_scope() {
        let cases = [
            (Harness::ClaudeCode, Scope::User, Some(".claude/skills")),
            (Harness::Codex, Scope::User, Some(".agents/skills")),
            (Harness::Cursor, Scope::Project, Some(".agents/skills")),
            (Harness::OpenCode, Scope::Project, Some(".agents/skills")),
            (
                Harness::OpenCode,
                Scope::User,
                Some(".config/opencode/skills"),
            ),
            (Harness::Hermes, Scope::User, Some(".hermes/skills")),
            (Harness::OpenClaw, Scope::User, Some(".openclaw/skills")),
            (
                Harness::AntigravityCli,
                Scope::User,
                Some(".gemini/antigravity-cli/skills"),
            ),
            (Harness::ClaudeCode, Scope::Local, None),
            (Harness::Zed, Scope::User, None),
        ];
        for (harness, scope, expected) in cases {
            assert_eq!(skills_dir(harness, scope), expected, "{harness} {scope:?}");
        }
    }

    #[test]
    fn install_skill_copies_directory_and_creates_backup_on_overwrite() {
        let temp = tempfile::tempdir().expect("tempdir");
        let skill_src = temp.path().join("demo-skill");
        fs::create_dir_all(&skill_src).expect("mkdir");
        fs::write(skill_src.join("SKILL.md"), "# Demo Skill").expect("write");

        let target_base = temp.path().join("target_skills");
        let options = RegistrationOptions {
            config: Some(target_base.clone()),
            ..RegistrationOptions::default()
        };

        let result = install_skill(Harness::Cursor, &skill_src, &options).expect("install");
        assert!(result.changed);
        assert_eq!(result.action, "installed");
        assert!(target_base.join("demo-skill/SKILL.md").exists());

        // Idempotent when identical
        let result2 = install_skill(Harness::Cursor, &skill_src, &options).expect("install again");
        assert!(!result2.changed);
        assert_eq!(result2.action, "already-configured");

        // Conflict when different and not force
        fs::write(skill_src.join("SKILL.md"), "# Updated Demo Skill").expect("write update");
        let conflict_err = install_skill(Harness::Cursor, &skill_src, &options);
        assert!(conflict_err.is_err());

        // Force overwrite with backup
        let mut force_options = options.clone();
        force_options.force = true;
        let result3 =
            install_skill(Harness::Cursor, &skill_src, &force_options).expect("force install");
        assert!(result3.changed);
        assert_eq!(result3.action, "installed");
        assert!(target_base.join("demo-skill.bak/SKILL.md").exists());
    }

    #[test]
    fn install_skill_dry_run_does_not_copy() {
        let temp = tempfile::tempdir().expect("tempdir");
        let skill_src = temp.path().join("dry-skill");
        fs::create_dir_all(&skill_src).expect("mkdir");
        fs::write(skill_src.join("SKILL.md"), "# Dry").expect("write");

        let target_base = temp.path().join("target_skills");
        let options = RegistrationOptions {
            config: Some(target_base.clone()),
            dry_run: true,
            ..RegistrationOptions::default()
        };

        let result = install_skill(Harness::Cursor, &skill_src, &options).expect("dry run");
        assert!(!result.changed);
        assert_eq!(result.action, "dry-run");
        assert!(!target_base.join("dry-skill").exists());
    }

    #[test]
    fn install_skill_rejects_missing_or_file_path() {
        let temp = tempfile::tempdir().expect("tempdir");
        let options = RegistrationOptions::default();
        let missing = temp.path().join("does-not-exist");
        assert!(install_skill(Harness::Cursor, &missing, &options).is_err());
    }
}
