//! The registry: builtin packs merged with user overrides, per language.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

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
    /// The `inherits` chain is followed transitively, outermost first: for a
    /// profile with `inherits = Some(parent)` the source order is every
    /// ancestor's builtin pack (grandparent before parent), then the profile's
    /// own builtin pack, then its user file — so nearer packs override farther
    /// ones, and user overrides win over all builtins. Only *builtin* packs are
    /// inherited: user files of an ancestor language do NOT affect the child (a
    /// user `c.toml` never leaks into `cpp`; override `cpp.toml` directly).
    ///
    /// A missing directory or missing per-language file is not an error;
    /// an unreadable or invalid file is.
    pub fn with_user_dir(dir: Option<&Path>) -> Result<Self, ConfigError> {
        Self::with_dirs(dir, None)
    }

    /// Load builtin packs, then a user override directory, then a project
    /// override directory.
    ///
    /// Sources merge in increasing priority: inherited ancestor builtins, the
    /// language's own builtin, `<user>/<lang>.toml`, and finally
    /// `<project>/<lang>.toml`. So a project pack overrides a user pack, which
    /// overrides the builtins, and a project pack may `disabled = true` any
    /// trigger. Both directories are optional; a missing directory or
    /// per-language file is not an error, but an unreadable or invalid one is.
    pub fn with_dirs(user: Option<&Path>, project: Option<&Path>) -> Result<Self, ConfigError> {
        let dirs: Vec<&Path> = [user, project].into_iter().flatten().collect();
        Self::with_dirs_layered(&dirs)
    }

    /// Load builtin packs, then an ordered list of override directories.
    ///
    /// `override_dirs` is lowest-priority first: each `<dir>/<lang>.toml` is
    /// merged over the ones before it, so the *last* directory wins per
    /// trigger and may `disabled = true` any trigger from an earlier tier.
    /// This is how path-scoped resolution composes its tiers — user global,
    /// user path-mapped, then layered project packs (root-most to nearest);
    /// see [`resolve_override_dirs`]. A missing directory or per-language file
    /// is not an error, but an unreadable or invalid one is.
    pub fn with_dirs_layered(override_dirs: &[&Path]) -> Result<Self, ConfigError> {
        let mut languages = HashMap::new();
        for profile in crate::lang::profile::PROFILES {
            let mut sources: Vec<(String, SnippetFile)> = Vec::new();

            // Inherited builtin packs, outermost ancestor first.
            for ancestor in inheritance_chain(profile)? {
                let origin = format!("builtin:{}", ancestor.id);
                let file = parse_snippet_file(&origin, ancestor.snippet_source)?;
                check_language(&origin, &file, ancestor.id)?;
                sources.push((origin, file));
            }

            let builtin_origin = format!("builtin:{}", profile.id);
            let builtin = parse_snippet_file(&builtin_origin, profile.snippet_source)?;
            check_language(&builtin_origin, &builtin, profile.id)?;
            sources.push((builtin_origin, builtin));

            // Override directories in increasing priority (last wins).
            for dir in override_dirs {
                push_override_file(&mut sources, dir, profile.id)?;
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

/// Push `<dir>/<lang_id>.toml` onto `sources` if it exists.
///
/// A missing directory or file is not an error; an unreadable or invalid file
/// is. The pack must declare `language = "<lang_id>"`.
fn push_override_file(
    sources: &mut Vec<(String, SnippetFile)>,
    dir: &Path,
    lang_id: &str,
) -> Result<(), ConfigError> {
    let path = dir.join(format!("{lang_id}.toml"));
    match std::fs::read_to_string(&path) {
        Ok(text) => {
            let origin = path.display().to_string();
            let file = parse_snippet_file(&origin, &text)?;
            check_language(&origin, &file, lang_id)?;
            sources.push((origin, file));
            Ok(())
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(source) => Err(ConfigError::Io { path, source }),
    }
}

/// Find the nearest `.codemmet/snippets` directory at or above `start`.
///
/// Walks upward from `start` and returns the first ancestor holding a
/// `.codemmet/snippets` directory (so the nearest project wins), or `None` up
/// to the filesystem root.
pub fn find_project_snippets_dir(start: &Path) -> Option<PathBuf> {
    start.ancestors().find_map(|dir| {
        let candidate = dir.join(".codemmet").join("snippets");
        candidate.is_dir().then_some(candidate)
    })
}

/// Every `.codemmet/snippets` directory at or above `start`, ordered
/// **root-most first**.
///
/// This is the layered generalization of [`find_project_snippets_dir`]: a
/// monorepo can carry a repo-wide pack at its root and a refining pack in a
/// subproject. Because the list runs root-most → nearest and
/// [`Registry::with_dirs_layered`] merges later over earlier, the nearest
/// pack wins per trigger while still inheriting everything the outer packs
/// define (merge up the tree, nearest wins).
pub fn find_project_snippets_dirs(start: &Path) -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = start
        .ancestors()
        .filter_map(|dir| {
            let candidate = dir.join(".codemmet").join("snippets");
            candidate.is_dir().then_some(candidate)
        })
        .collect();
    // `ancestors()` yields nearest-first; the loader wants lowest priority
    // (root-most) first.
    dirs.reverse();
    dirs
}

/// Resolve the full ordered list of override directories for a file at
/// `start`, lowest priority first.
///
/// The tiers, in increasing priority, are:
///
/// 1. the user global directory (`user_global`, e.g. `--config` or the
///    platform default),
/// 2. personal path-mapped directories whose prefix matches `start`
///    (shallow prefix → deep prefix; see [`PathMapping`](crate::config::paths::PathMapping)),
/// 3. layered project packs from the filesystem root down to `start`
///    (only when `include_project`).
///
/// Feeding the result to [`Registry::with_dirs_layered`] realizes the
/// precedence `builtin < user < user-path-mapped < project`.
pub fn resolve_override_dirs(
    start: &Path,
    user_global: Option<&Path>,
    mapping: &crate::config::paths::PathMapping,
    include_project: bool,
) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Some(user) = user_global {
        dirs.push(user.to_path_buf());
    }
    dirs.extend(mapping.dirs_for(start));
    if include_project {
        dirs.extend(find_project_snippets_dirs(start));
    }
    dirs
}

/// Resolve the `.codemmet/snippets` directory a new snippet should be written
/// to, walking up from `start`.
///
/// Prefers an existing `.codemmet/snippets` at or above `start` (so a capture
/// lands in the project pack already in use). When none exists, it is placed
/// under the nearest ancestor that looks like a project root — one holding a
/// `.git` entry — else directly under `start`. The directory is *not* created;
/// the caller creates it when it writes.
pub fn resolve_capture_dir(start: &Path) -> PathBuf {
    if let Some(dir) = find_project_snippets_dir(start) {
        return dir;
    }
    let root = start
        .ancestors()
        .find(|dir| dir.join(".git").exists())
        .unwrap_or(start);
    root.join(".codemmet").join("snippets")
}

/// The ancestor profiles `profile` inherits, transitively, outermost first.
///
/// Errors if an `inherits` id is unknown or the chain contains a cycle.
fn inheritance_chain(
    profile: &'static LanguageProfile,
) -> Result<Vec<&'static LanguageProfile>, ConfigError> {
    let mut chain = Vec::new();
    let mut seen = vec![profile.id];
    let mut current = profile;
    while let Some(parent_id) = current.inherits {
        if seen.contains(&parent_id) {
            return Err(ConfigError::Invalid {
                trigger: "meta".to_owned(),
                origin: format!("builtin:{}", profile.id),
                message: format!("inheritance cycle through `{parent_id}`"),
            });
        }
        let parent = crate::lang::profile::find(parent_id).ok_or_else(|| ConfigError::Invalid {
            trigger: "meta".to_owned(),
            origin: format!("builtin:{}", current.id),
            message: format!("profile inherits unknown language `{parent_id}`"),
        })?;
        seen.push(parent_id);
        chain.push(parent);
        current = parent;
    }
    // Collected nearest-first; reverse so the outermost ancestor comes first.
    chain.reverse();
    Ok(chain)
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
    fn project_dir_overrides_user_dir() {
        let user = scratch_dir("tier-user");
        let project = scratch_dir("tier-project");
        // User pack sets `pr`; project pack overrides it and disables `epr`.
        std::fs::write(
            user.join("rust.toml"),
            "[meta]\nlanguage = \"rust\"\n[snippets.pr]\ndescription = \"user pr\"\nbody = \"user!()\"\n",
        )
        .expect("user pack should be writable");
        std::fs::write(
            project.join("rust.toml"),
            "[meta]\nlanguage = \"rust\"\n[snippets.pr]\ndescription = \"project pr\"\nbody = \"project!()\"\n[snippets.epr]\ndisabled = true\n",
        )
        .expect("project pack should be writable");

        let rust = Registry::with_dirs(Some(&user), Some(&project))
            .expect("dirs should load")
            .language("rust")
            .expect("rust resolves")
            .set
            .clone();
        assert_eq!(
            rust.get("pr").and_then(|s| s.description.as_deref()),
            Some("project pr"),
            "project pack should win over the user pack"
        );
        assert!(rust.get("epr").is_none(), "project pack should disable epr");

        std::fs::remove_dir_all(&user).ok();
        std::fs::remove_dir_all(&project).ok();
    }

    #[test]
    fn find_project_snippets_dirs_lists_root_most_first() {
        let root = scratch_dir("layered-list");
        let root_pack = root.join(".codemmet").join("snippets");
        let sub = root.join("crate").join("svc");
        let sub_pack = sub.join(".codemmet").join("snippets");
        std::fs::create_dir_all(&root_pack).expect("root pack dir");
        std::fs::create_dir_all(&sub_pack).expect("sub pack dir");
        let start = sub.join("src");
        std::fs::create_dir_all(&start).expect("start dir");

        assert_eq!(
            find_project_snippets_dirs(&start),
            vec![root_pack, sub_pack.clone()],
            "root-most first"
        );
        // The nearest-only helper still returns the deepest.
        assert_eq!(
            find_project_snippets_dir(&start).as_deref(),
            Some(sub_pack.as_path())
        );

        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn layered_project_packs_merge_up_the_tree_nearest_wins() {
        let root = scratch_dir("layered-merge");
        let root_pack = root.join(".codemmet").join("snippets");
        let sub = root.join("svc");
        let sub_pack = sub.join(".codemmet").join("snippets");
        std::fs::create_dir_all(&root_pack).expect("root pack dir");
        std::fs::create_dir_all(&sub_pack).expect("sub pack dir");
        // Root defines `base` and `shared`; the subproject overrides `shared`
        // and adds `svc`. The subproject must inherit `base`.
        std::fs::write(
            root_pack.join("rust.toml"),
            "[meta]\nlanguage = \"rust\"\n[snippets.base]\nbody = \"base()\"\n[snippets.shared]\ndescription = \"root\"\nbody = \"root!()\"\n",
        )
        .expect("root pack");
        std::fs::write(
            sub_pack.join("rust.toml"),
            "[meta]\nlanguage = \"rust\"\n[snippets.shared]\ndescription = \"sub\"\nbody = \"sub!()\"\n[snippets.svc]\nbody = \"svc()\"\n",
        )
        .expect("sub pack");

        let start = sub.join("src");
        std::fs::create_dir_all(&start).expect("start dir");
        let dirs = find_project_snippets_dirs(&start);
        let refs: Vec<&Path> = dirs.iter().map(PathBuf::as_path).collect();
        let rust = Registry::with_dirs_layered(&refs)
            .expect("layered load")
            .language("rust")
            .expect("rust resolves")
            .set
            .clone();
        assert!(rust.get("base").is_some(), "inherits root's base");
        assert!(rust.get("svc").is_some(), "gains the subproject's svc");
        assert_eq!(
            rust.get("shared").and_then(|s| s.description.as_deref()),
            Some("sub"),
            "nearest (subproject) wins the shared trigger"
        );

        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn resolve_override_dirs_orders_tiers_user_mapped_project() {
        use crate::config::paths::PathMapping;
        let root = scratch_dir("resolve-order");
        let proj = root.join(".codemmet").join("snippets");
        std::fs::create_dir_all(&proj).expect("project pack dir");
        let start = root.join("src");
        std::fs::create_dir_all(&start).expect("start dir");
        let user = root.join("user-global");

        let mapping = PathMapping::parse(
            "t",
            &format!(
                "[[map]]\nprefix = {:?}\ndir = {:?}\n",
                root.display(),
                root.join("mapped").display()
            ),
        )
        .expect("mapping parses");

        let dirs = resolve_override_dirs(&start, Some(&user), &mapping, true);
        assert_eq!(
            dirs,
            vec![user, root.join("mapped"), proj],
            "user < mapped < project, lowest first"
        );

        // With project excluded, only user + mapped remain.
        let dirs = resolve_override_dirs(&start, Some(&root.join("user-global")), &mapping, false);
        assert_eq!(dirs.len(), 2);

        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn find_project_snippets_dir_walks_up_to_nearest() {
        let root = scratch_dir("discover");
        let snippets = root.join(".codemmet").join("snippets");
        std::fs::create_dir_all(&snippets).expect("snippets dir should be creatable");
        let nested = root.join("crate").join("src");
        std::fs::create_dir_all(&nested).expect("nested dir should be creatable");

        assert_eq!(
            find_project_snippets_dir(&nested).as_deref(),
            Some(snippets.as_path())
        );
        assert!(find_project_snippets_dir(std::env::temp_dir().as_path()).is_none());

        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn resolve_capture_dir_prefers_existing_then_git_root() {
        let root = scratch_dir("capture-resolve");
        let nested = root.join("crate").join("src");
        std::fs::create_dir_all(&nested).expect("nested dir should be creatable");

        // No existing pack, no .git: falls back to the start directory itself.
        assert_eq!(
            resolve_capture_dir(&nested),
            nested.join(".codemmet").join("snippets")
        );

        // A .git at the root makes that the project root for a fresh pack.
        std::fs::create_dir_all(root.join(".git")).expect(".git should be creatable");
        assert_eq!(
            resolve_capture_dir(&nested),
            root.join(".codemmet").join("snippets")
        );

        // An existing pack wins over both.
        let existing = root.join(".codemmet").join("snippets");
        std::fs::create_dir_all(&existing).expect("existing pack dir should be creatable");
        assert_eq!(resolve_capture_dir(&nested), existing);

        std::fs::remove_dir_all(&root).ok();
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
