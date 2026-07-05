//! The registry: builtin packs merged with user overrides, per language.

use std::collections::HashMap;
use std::path::Path;

use crate::config::loader::{ConfigError, SnippetSet, parse_snippet_file};
use crate::config::schema::SnippetFile;
use crate::lang::profile::LanguageProfile;

/// One language's profile plus its merged snippet set.
#[derive(Debug)]
pub struct LanguageSnippets {
    /// The static language profile.
    pub profile: &'static LanguageProfile,
    /// Builtin snippets merged with any user overrides.
    pub set: SnippetSet,
}

/// All loaded languages, ready for lookup by id, extension, or LSP id.
#[derive(Debug)]
pub struct Registry {
    languages: HashMap<&'static str, LanguageSnippets>,
}

impl Registry {
    /// Load builtin packs only (no user overrides). Used by tests.
    pub fn with_builtins() -> Result<Self, ConfigError> {
        Self::with_user_dir(None)
    }

    /// Load builtin packs, then merge `<dir>/<lang>.toml` user overrides.
    ///
    /// For a profile with `inherits = Some(parent)` the source order is:
    /// parent builtin pack, own builtin pack, own user file — so the child's
    /// builtins override the parent's, and user overrides win over both.
    /// Only the parent's *builtin* pack is inherited: user files of the
    /// parent language do NOT affect the child (a user `c.toml` never leaks
    /// into `cpp`; override `cpp.toml` directly instead).
    ///
    /// A missing directory or missing per-language file is not an error;
    /// an unreadable or invalid file is.
    pub fn with_user_dir(dir: Option<&Path>) -> Result<Self, ConfigError> {
        let mut languages = HashMap::new();
        for profile in crate::lang::profile::PROFILES {
            let mut sources: Vec<(String, SnippetFile)> = Vec::new();

            if let Some(parent_id) = profile.inherits {
                let parent =
                    crate::lang::profile::find(parent_id).ok_or_else(|| ConfigError::Invalid {
                        trigger: "meta".to_owned(),
                        origin: format!("builtin:{}", profile.id),
                        message: format!("profile inherits unknown language `{parent_id}`"),
                    })?;
                let parent_origin = format!("builtin:{}", parent.id);
                let parent_file = parse_snippet_file(&parent_origin, parent.snippet_source)?;
                // The parent pack declares the PARENT's language id.
                check_language(&parent_origin, &parent_file, parent.id)?;
                sources.push((parent_origin, parent_file));
            }

            let builtin_origin = format!("builtin:{}", profile.id);
            let builtin = parse_snippet_file(&builtin_origin, profile.snippet_source)?;
            check_language(&builtin_origin, &builtin, profile.id)?;

            sources.push((builtin_origin, builtin));
            if let Some(dir) = dir {
                let path = dir.join(format!("{}.toml", profile.id));
                match std::fs::read_to_string(&path) {
                    Ok(text) => {
                        let origin = path.display().to_string();
                        let user = parse_snippet_file(&origin, &text)?;
                        check_language(&origin, &user, profile.id)?;
                        sources.push((origin, user));
                    }
                    // A missing directory or per-language file is not an error.
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                    Err(source) => return Err(ConfigError::Io { path, source }),
                }
            }

            let refs: Vec<(&str, &SnippetFile)> = sources
                .iter()
                .map(|(origin, file)| (origin.as_str(), file))
                .collect();
            let set = SnippetSet::compile(&refs)?;
            languages.insert(profile.id, LanguageSnippets { profile, set });
        }
        Ok(Self { languages })
    }

    /// Look up a language by id, file extension, or LSP language id.
    pub fn language(&self, id_or_ext: &str) -> Option<&LanguageSnippets> {
        crate::lang::profile::find(id_or_ext).and_then(|p| self.languages.get(p.id))
    }

    /// All loaded languages, in unspecified order.
    pub fn languages(&self) -> impl Iterator<Item = &LanguageSnippets> {
        self.languages.values()
    }
}

/// Verify a pack's `meta.language` matches the profile it is loaded for.
fn check_language(origin: &str, file: &SnippetFile, expected: &str) -> Result<(), ConfigError> {
    if file.meta.language == expected {
        Ok(())
    } else {
        Err(ConfigError::Invalid {
            trigger: "meta".to_owned(),
            origin: origin.to_owned(),
            message: format!(
                "meta.language is `{}` but this pack is loaded for `{expected}`",
                file.meta.language
            ),
        })
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    /// Create a unique scratch directory for one test.
    fn scratch_dir(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("codemmet-registry-{name}-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("scratch dir should be creatable");
        dir
    }

    #[test]
    fn with_builtins_loads_every_profile() {
        let registry = Registry::with_builtins().expect("builtin packs should load");
        assert_eq!(
            registry.languages().count(),
            crate::lang::profile::PROFILES.len()
        );
        for key in ["rust", "rs", "python", "py", "c", "hpp", "jsx", "mts", "go"] {
            let lang = registry.language(key);
            assert!(lang.is_some(), "language({key}) should resolve");
        }
        let rust = registry.language("rs").expect("rs should resolve");
        assert_eq!(rust.profile.id, "rust");
        assert!(rust.set.get("fori").is_some());
        assert!(registry.language("cobol").is_none());
    }

    #[test]
    fn child_inherits_parent_builtins_and_overrides_them() {
        let registry = Registry::with_builtins().expect("builtin packs should load");
        let cpp = &registry.language("cpp").expect("cpp should resolve").set;
        // Inherited from C untouched.
        assert!(cpp.get("fori").is_some(), "cpp should inherit C's fori");
        // Added by the C++ pack.
        assert!(cpp.get("class").is_some(), "cpp should define class");
        // Overridden: C++ replaces C's typedef-struct with a plain struct.
        let cpp_struct = cpp.get("struct").expect("cpp struct should resolve");
        assert!(
            !format!("{:?}", cpp_struct.template).contains("typedef struct"),
            "cpp struct should be the C++ override, not C's typedef form"
        );
        // The parent language itself is unaffected by the child pack.
        let c = &registry.language("c").expect("c should resolve").set;
        assert!(c.get("class").is_none(), "C must not gain cpp's class");
    }

    #[test]
    fn parent_user_files_do_not_affect_child() {
        let dir = scratch_dir("parent-user");
        std::fs::write(
            dir.join("c.toml"),
            concat!(
                "[meta]\nlanguage = \"c\"\n",
                "[snippets.userc]\ndescription = \"user C only\"\nbody = \"user_c();$0\"\n",
                "[snippets.fori]\ndisabled = true\n",
            ),
        )
        .expect("user pack should be writable");

        let registry = Registry::with_user_dir(Some(&dir)).expect("user dir should load");
        let c = &registry.language("c").expect("c should resolve").set;
        assert!(c.get("userc").is_some());
        assert!(c.get("fori").is_none(), "user file disables C's fori");
        // cpp inherits only the C *builtin* pack: the user's c.toml neither
        // adds `userc` to cpp nor disables cpp's inherited `fori`.
        let cpp = &registry.language("cpp").expect("cpp should resolve").set;
        assert!(cpp.get("userc").is_none(), "user c.toml must not leak");
        assert!(cpp.get("fori").is_some(), "cpp keeps the builtin fori");

        std::fs::remove_dir_all(&dir).expect("scratch dir should be removable");
    }

    #[test]
    fn user_dir_overrides_and_disables_builtins() {
        let dir = scratch_dir("override");
        std::fs::write(
            dir.join("rust.toml"),
            concat!(
                "[meta]\nlanguage = \"rust\"\n",
                "[snippets.fn]\ndescription = \"user fn\"\nbody = \"fn $1() {}\"\n",
                "[snippets.main]\ndisabled = true\n",
            ),
        )
        .expect("user pack should be writable");

        let registry = Registry::with_user_dir(Some(&dir)).expect("user dir should load");
        let rust = &registry.language("rust").expect("rust should resolve").set;
        let fn_snippet = rust.get("fn").expect("fn should survive the merge");
        assert_eq!(fn_snippet.description.as_deref(), Some("user fn"));
        assert!(rust.get("main").is_none(), "main should be disabled");
        // Untouched builtins survive, and python is unaffected.
        assert!(rust.get("fori").is_some());
        let python = &registry
            .language("python")
            .expect("python should resolve")
            .set;
        assert!(python.get("main").is_some());

        std::fs::remove_dir_all(&dir).expect("scratch dir should be removable");
    }

    #[test]
    fn user_file_with_wrong_language_is_error() {
        let dir = scratch_dir("wrong-lang");
        std::fs::write(dir.join("rust.toml"), "[meta]\nlanguage = \"python\"\n")
            .expect("user pack should be writable");

        let err = Registry::with_user_dir(Some(&dir)).expect_err("mismatch should be rejected");
        match err {
            ConfigError::Invalid {
                origin, message, ..
            } => {
                assert!(origin.ends_with("rust.toml"), "origin was {origin}");
                assert!(message.contains("meta.language"), "message was {message}");
            }
            other => panic!("expected Invalid, got {other:?}"),
        }

        std::fs::remove_dir_all(&dir).expect("scratch dir should be removable");
    }

    #[test]
    fn user_file_with_bad_toml_is_error() {
        let dir = scratch_dir("bad-toml");
        std::fs::write(dir.join("python.toml"), "not toml [").expect("file should be writable");

        let err = Registry::with_user_dir(Some(&dir)).expect_err("bad TOML should be rejected");
        assert!(
            matches!(err, ConfigError::Toml { ref origin, .. } if origin.ends_with("python.toml")),
            "expected Toml error, got {err:?}"
        );

        std::fs::remove_dir_all(&dir).expect("scratch dir should be removable");
    }

    #[test]
    fn nonexistent_user_dir_is_fine() {
        let dir = std::env::temp_dir().join("codemmet-registry-does-not-exist");
        let registry =
            Registry::with_user_dir(Some(&dir)).expect("missing dir should not be an error");
        assert_eq!(
            registry.languages().count(),
            crate::lang::profile::PROFILES.len()
        );
    }
}
