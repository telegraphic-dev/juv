//! JDK discovery, caching, and auto-provisioning via Adoptium.
//!
//! Discovery probes standard JDK locations (JAVA_HOME, PATH, JBang, SDKMAN,
//! mise, Gradle, system dirs) and symlinks found JDKs into
//! `~/.cache/jbx/jdks/<major>/` for fast future lookups.
//!
//! If no matching JDK is found, it can be auto-provisioned from the Adoptium
//! (Eclipse Temurin) API.

use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::Duration;

use anyhow::{anyhow, Context};
use flate2::read::GzDecoder;
use sha2::{Digest, Sha256};

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Find a JDK for the given major version.
///
/// Probes the jbx symlink cache first, then discovers JDKs from well-known
/// locations. If `auto_install` is true and no JDK is found, downloads from
/// Adoptium.
///
/// Returns the JDK root directory (containing `bin/java`).
pub fn find_jdk(major_version: u32, auto_install: bool) -> anyhow::Result<PathBuf> {
    let jdk_dir = jdk_cache_dir()?;

    // 1. Check the symlink cache. Ignore stale/bad entries such as /usr,
    // which can contain bin/java via PATH but is not a JDK root.
    let cached = jdk_dir.join(major_version.to_string());
    if looks_like_jdk_root(&cached) {
        return Ok(cached);
    } else if cached.exists() || cached.is_symlink() {
        let _ = remove_stale_cache_entry(&cached);
    }

    // 2. Scan all known locations, build a map of major → root
    let discovered = discover_all_jdks()?;

    // 3. Symlink all discovered JDKs into the cache
    for (major, root) in &discovered {
        let link = jdk_dir.join(major.to_string());
        // Only create if not already present
        if !link.exists() {
            let _ = create_symlink_dir(root, &link);
        }
    }

    // 4. Re-check cache. If cache linking failed (for example on a read-only
    // filesystem), use the discovered JDK root directly instead of returning a
    // non-existent cache path.
    if let Some(root) = discovered.get(&major_version) {
        let cached_path = jdk_dir.join(major_version.to_string());
        if looks_like_jdk_root(&cached_path) {
            return Ok(cached_path);
        }
        return Ok(root.clone());
    }

    // 5. Auto-provision from Adoptium
    if auto_install {
        let installed = install_from_adoptium(major_version)?;
        // Symlink into cache — installed dir IS the cache entry
        return Ok(installed);
    }

    Err(anyhow!(
        "JDK {major_version} not found. Install it manually or run with auto-provisioning enabled."
    ))
}

/// List all discovered and cached JDKs. Returns (major_version, jdk_root) pairs.
pub fn list_jdks() -> anyhow::Result<Vec<(u32, PathBuf)>> {
    let mut result = Vec::new();

    // Gather from cache
    let jdk_dir = jdk_cache_dir()?;
    if jdk_dir.exists() {
        for entry in fs::read_dir(&jdk_dir)? {
            let entry = entry?;
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if let Ok(major) = name_str.parse::<u32>() {
                let root = entry.path();
                if looks_like_jdk_root(&root) {
                    result.push((major, root));
                }
            }
        }
    }

    // Also discover un-cached JDKs
    let discovered = discover_all_jdks()?;
    for (major, root) in discovered {
        if !result.iter().any(|(m, _)| *m == major) {
            result.push((major, root));
        }
    }

    result.sort_by_key(|(m, _)| *m);
    Ok(result)
}

/// Install a JDK from Adoptium. Returns the JDK root path.
pub fn install_jdk(major_version: u32) -> anyhow::Result<PathBuf> {
    install_from_adoptium(major_version)
}

// ---------------------------------------------------------------------------
// JDK Discovery
// ---------------------------------------------------------------------------

/// Probe all well-known JDK locations. Returns a map of major_version → jdk_root.
fn discover_all_jdks() -> anyhow::Result<std::collections::HashMap<u32, PathBuf>> {
    let mut jdks = std::collections::HashMap::new();

    // JAVA_HOME
    if let Ok(java_home) = std::env::var("JAVA_HOME") {
        if let Some((major, root)) = probe_jdk_root(&PathBuf::from(&java_home)) {
            jdks.entry(major).or_insert(root);
        }
    }

    // PATH — find `java` binary and walk up to JDK root
    if let Ok(java) = which::which("java") {
        // java is typically at <jdk_root>/bin/java — resolve symlinks first
        if let Ok(resolved) = fs::canonicalize(&java) {
            if let Some(parent) = resolved.parent().and_then(|p| p.parent()) {
                if let Some((major, root)) = probe_jdk_root(parent) {
                    jdks.entry(major).or_insert(root);
                }
            }
        }
    }

    // JBang: ~/.jbang/jdks/<major>/
    if let Some(home) = dirs::home_dir() {
        let jbang_dir = home.join(".jbang").join("jdks");
        probe_major_version_dirs(&jbang_dir, &mut jdks);

        // SDKMAN: ~/.sdkman/candidates/java/<version>-<vendor>/
        let sdkman_dir = home.join(".sdkman").join("candidates").join("java");
        probe_versioned_dirs(&sdkman_dir, &mut jdks);

        // mise: ~/.local/share/mise/installs/java-<version>/
        let mise_dir = home
            .join(".local")
            .join("share")
            .join("mise")
            .join("installs");
        probe_mise_java_dirs(&mise_dir, &mut jdks);

        // Gradle: ~/.gradle/jdks/jdk-<version>+<build>/
        let gradle_dir = home.join(".gradle").join("jdks");
        probe_gradle_jdk_dirs(&gradle_dir, &mut jdks);
    }

    // System: /usr/lib/jvm/ (Linux)
    let jvm_dir = Path::new("/usr/lib/jvm");
    if jvm_dir.exists() {
        probe_system_jvm_dirs(jvm_dir, &mut jdks);
    }

    Ok(jdks)
}

/// Probe a directory that has major-version subdirs (like JBang: 21/, 17/).
fn probe_major_version_dirs(base: &Path, jdks: &mut std::collections::HashMap<u32, PathBuf>) {
    if !base.is_dir() {
        return;
    }
    if let Ok(entries) = fs::read_dir(base) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            // Skip "current" and non-numeric dirs
            if name_str.parse::<u32>().is_ok() {
                if let Some((major, root)) = probe_jdk_root(&entry.path()) {
                    jdks.entry(major).or_insert(root);
                }
            }
        }
    }
}

/// Probe SDKMAN-style dirs: <version>-<vendor> (e.g. 21.0.3-tem).
fn probe_versioned_dirs(base: &Path, jdks: &mut std::collections::HashMap<u32, PathBuf>) {
    if !base.is_dir() {
        return;
    }
    if let Ok(entries) = fs::read_dir(base) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if name_str == "current" {
                continue;
            }
            let root = entry.path();
            if let Some((major, root)) = probe_jdk_root(&root) {
                jdks.entry(major).or_insert(root);
            }
        }
    }
}

/// Probe mise-style dirs: java-<version> or java-<major> (symlink or real).
fn probe_mise_java_dirs(base: &Path, jdks: &mut std::collections::HashMap<u32, PathBuf>) {
    if !base.is_dir() {
        return;
    }
    if let Ok(entries) = fs::read_dir(base) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if name_str.starts_with("java-") {
                if let Some((major, root)) = probe_jdk_root(&entry.path()) {
                    jdks.entry(major).or_insert(root);
                }
            }
        }
    }
}

/// Probe Gradle-style dirs: jdk-<version>+<build> (e.g. jdk-21.0.11+10).
fn probe_gradle_jdk_dirs(base: &Path, jdks: &mut std::collections::HashMap<u32, PathBuf>) {
    if !base.is_dir() {
        return;
    }
    if let Ok(entries) = fs::read_dir(base) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if name_str.starts_with("jdk-") {
                if let Some((major, root)) = probe_jdk_root(&entry.path()) {
                    jdks.entry(major).or_insert(root);
                }
            }
        }
    }
}

/// Probe /usr/lib/jvm/ entries (Debian/Ubuntu style).
fn probe_system_jvm_dirs(base: &Path, jdks: &mut std::collections::HashMap<u32, PathBuf>) {
    if let Ok(entries) = fs::read_dir(base) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            // Skip .jinfo files and misc
            if name_str.starts_with('.') || name_str.ends_with(".jinfo") {
                continue;
            }
            let root = entry.path();
            // Follow symlinks — /usr/lib/jvm often has version-prefixed symlinks
            let resolved = if root.is_symlink() {
                match fs::canonicalize(&root) {
                    Ok(r) => r,
                    Err(_) => continue,
                }
            } else {
                root
            };
            if !looks_like_jdk_root(&resolved) {
                continue;
            }
            // Prefer reading the release file over directory name parsing
            // (directory names like java-1.21.0-openjdk-arm64 are misleading)
            if let Some(major) = detect_jdk_major_version(&resolved) {
                jdks.entry(major).or_insert(resolved);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Version Detection
// ---------------------------------------------------------------------------

/// Read the `release` file in a JDK root to determine the major version.
/// Falls back to running `java -version` if the release file is missing.
pub fn detect_jdk_major_version(jdk_root: &Path) -> Option<u32> {
    // Try the release file first
    let release_file = jdk_root.join("release");
    if let Ok(content) = fs::read_to_string(&release_file) {
        for line in content.lines() {
            if let Some(value) = line.strip_prefix("JAVA_VERSION=") {
                let version = value.trim_matches('"').trim();
                return parse_major_from_version_string(version);
            }
        }
    }

    // Fallback: run java -version
    let java_bin = java_bin_path(jdk_root);
    if java_bin.exists() {
        if let Ok(output) = std::process::Command::new(&java_bin)
            .arg("-version")
            .output()
        {
            let text = String::from_utf8_lossy(&output.stderr);
            return parse_major_from_java_version_output(&text);
        }
    }

    None
}

/// Parse the major version from `java -version` output.
///
/// JVMs may prefix stderr with environment notices such as
/// `Picked up JAVA_TOOL_OPTIONS: -Xmx4g`, so only parse the quoted token on the
/// actual `... version "..."` line.
fn parse_major_from_java_version_output(output: &str) -> Option<u32> {
    output.lines().find_map(|line| {
        if !line.contains("version \"") {
            return None;
        }
        let start = line.find('"')? + 1;
        let end = line[start..].find('"')? + start;
        parse_major_from_version_string(&line[start..end])
    })
}

/// Parse the major version from a version string like "21.0.3", "1.8.0_432", "17.0.11+10".
fn parse_major_from_version_string(version: &str) -> Option<u32> {
    static LEGACY_RE: OnceLock<regex::Regex> = OnceLock::new();
    static MODERN_RE: OnceLock<regex::Regex> = OnceLock::new();

    // Java 8 reports as "1.8.x"; modern Java reports as "17.x", "25", etc.
    if version.starts_with("1.") {
        let re = LEGACY_RE.get_or_init(|| regex::Regex::new(r"^1\.(\d+)").expect("valid regex"));
        return re
            .captures(version)
            .and_then(|caps| caps.get(1).and_then(|m| m.as_str().parse().ok()));
    }

    let re = MODERN_RE.get_or_init(|| regex::Regex::new(r"^(\d+)").expect("valid regex"));
    re.captures(version)
        .and_then(|caps| caps.get(1).and_then(|m| m.as_str().parse().ok()))
}

/// Parse a JBang-style `//JAVA` directive into the requested major version.
///
/// JBang accepts selectors such as `25+` to mean Java 25 or newer. For jbx's
/// current resolver we use the leading major as the requested floor.
pub fn parse_java_version_directive(version: &str) -> anyhow::Result<u32> {
    parse_major_from_version_string(version)
        .with_context(|| format!("invalid JAVA version directive: {version}"))
}

/// Probe a JDK root directory — returns (major_version, root) if valid.
fn probe_jdk_root(jdk_root: &Path) -> Option<(u32, PathBuf)> {
    if !looks_like_jdk_root(jdk_root) {
        return None;
    }
    detect_jdk_major_version(jdk_root).map(|major| (major, jdk_root.to_path_buf()))
}

/// Check if a directory looks like a JDK root (not just any dir with java on PATH).
/// A JDK root must have direct `bin/java`, direct `bin/javac`, and a `release`
/// metadata file. Requiring `release` prevents false positives like `/usr`,
/// which may contain `/usr/bin/java` via PATH alternatives but is not JAVA_HOME.
fn looks_like_jdk_root(dir: &Path) -> bool {
    dir.is_dir()
        && java_bin_path(dir).is_file()
        && javac_bin_path(dir).is_file()
        && dir.join("release").is_file()
}

// ---------------------------------------------------------------------------
// Adoptium API — JDK Download & Install
// ---------------------------------------------------------------------------

/// Download and install a JDK from Adoptium. Returns the JDK root directory.
fn install_from_adoptium(major_version: u32) -> anyhow::Result<PathBuf> {
    let jdk_dir = jdk_cache_dir()?;
    let target_dir = jdk_dir.join(major_version.to_string());

    // Already installed?
    if looks_like_jdk_root(&target_dir) {
        return Ok(target_dir);
    }

    let (os, arch) = detect_platform()?;
    let archive_url = format!(
        "https://api.adoptium.net/v3/binary/latest/{major_version}/ga/{os}/{arch}/jdk/hotspot/normal/eclipse"
    );
    let checksum_url = format!(
        "https://api.adoptium.net/v3/assets/latest/{major_version}/hotspot?architecture={arch}&image_type=jdk&os={os}&vendor=eclipse"
    );

    eprintln!("Downloading JDK {major_version} from Adoptium...");

    let agent = format!("jbx/{}", env!("CARGO_PKG_VERSION"));
    let expected_checksum = normalize_sha256(&fetch_adoptium_checksum(&checksum_url, &agent)?)?;

    let archive_path = target_dir.with_extension(if os == "windows" {
        "zip.tmp"
    } else {
        "tar.gz.tmp"
    });
    if archive_path.exists() {
        fs::remove_file(&archive_path).with_context(|| {
            format!(
                "failed to remove stale JDK archive {}",
                archive_path.display()
            )
        })?;
    }
    if let Some(parent) = archive_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let response = ureq::AgentBuilder::new()
        .timeout_read(Duration::from_secs(300))
        .timeout_write(Duration::from_secs(30))
        .redirects(5)
        .build()
        .get(&archive_url)
        .set("User-Agent", &agent)
        .call()
        .with_context(|| format!("failed to download JDK {major_version} from Adoptium"))?;

    let mut reader = response.into_reader();
    let mut file = fs::File::create(&archive_path)
        .with_context(|| format!("failed to create JDK archive {}", archive_path.display()))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = reader
            .read(&mut buffer)
            .context("failed to read JDK archive")?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
        file.write_all(&buffer[..read])
            .context("failed to write JDK archive")?;
    }
    file.flush().context("failed to flush JDK archive")?;

    let actual_checksum = format!("{:x}", hasher.finalize());
    if actual_checksum != expected_checksum {
        let _ = fs::remove_file(&archive_path);
        return Err(anyhow!(
            "JDK archive checksum mismatch: expected {expected_checksum}, got {actual_checksum}"
        ));
    }

    // Extract into a temp dir first, then move to final location
    let tmp_dir = target_dir.with_extension("tmp");
    if tmp_dir.exists() {
        let _ = fs::remove_dir_all(&tmp_dir);
    }
    fs::create_dir_all(&tmp_dir)
        .with_context(|| format!("failed to create temp JDK dir {}", tmp_dir.display()))?;

    extract_jdk_archive(&archive_path, &tmp_dir, os)?;

    // Adoptium .tar.gz contains a single directory like jdk-25.0.3+9 — move its contents up
    let actual_root = find_extracted_jdk_root(&tmp_dir, os);

    // Move to final location
    if target_dir.exists() {
        let _ = remove_stale_cache_entry(&target_dir);
    }
    fs::rename(&actual_root, &target_dir)
        .with_context(|| format!("failed to move JDK to {}", target_dir.display()))?;

    // Clean up tmp dir if it's different from actual_root
    if tmp_dir != actual_root && tmp_dir.exists() {
        let _ = fs::remove_dir_all(&tmp_dir);
    }
    let _ = fs::remove_file(&archive_path);

    eprintln!("JDK {major_version} installed to {}", target_dir.display());
    Ok(target_dir)
}

/// Fetch the SHA-256 checksum from the Adoptium assets API.
fn fetch_adoptium_checksum(url: &str, agent: &str) -> anyhow::Result<String> {
    let response = ureq::AgentBuilder::new()
        .timeout_read(Duration::from_secs(30))
        .build()
        .get(url)
        .set("User-Agent", agent)
        .call()
        .context("failed to fetch JDK checksum metadata")?;

    let mut body_text = String::new();
    response
        .into_reader()
        .read_to_string(&mut body_text)
        .context("failed to read JDK checksum metadata")?;
    let body: serde_json::Value =
        serde_json::from_str(&body_text).context("failed to parse JDK checksum metadata JSON")?;
    body.get(0)
        .and_then(|v| v.get("binary"))
        .and_then(|v| v.get("package"))
        .and_then(|v| v.get("checksum"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow!("JDK checksum metadata did not contain binary.package.checksum"))
}

fn normalize_sha256(checksum: &str) -> anyhow::Result<String> {
    let normalized = checksum.trim().replace('-', "").to_ascii_lowercase();
    if normalized.len() != 64 || !normalized.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(anyhow!(
            "invalid SHA-256 checksum from Adoptium: {checksum}"
        ));
    }
    Ok(normalized)
}

/// Extract a JDK archive (.tar.gz on Linux/macOS, .zip on Windows) into target_dir.
fn extract_jdk_archive(archive_path: &Path, target_dir: &Path, os: &str) -> anyhow::Result<()> {
    if os == "windows" {
        // .zip extraction
        let cursor = fs::File::open(archive_path).with_context(|| {
            format!("failed to open JDK zip archive {}", archive_path.display())
        })?;
        let mut archive =
            zip::ZipArchive::new(cursor).with_context(|| "failed to read JDK zip archive")?;
        archive
            .extract(target_dir)
            .with_context(|| "failed to extract JDK zip archive")?;
    } else {
        // .tar.gz extraction
        let cursor = fs::File::open(archive_path).with_context(|| {
            format!(
                "failed to open JDK tar.gz archive {}",
                archive_path.display()
            )
        })?;
        let gz_decoder = GzDecoder::new(cursor);
        let mut archive = tar::Archive::new(gz_decoder);
        archive
            .unpack(target_dir)
            .with_context(|| "failed to extract JDK tar.gz archive")?;
    }
    Ok(())
}

/// Resolve the real JDK root within an extracted Adoptium archive.
///
/// Linux/Windows archives contain a single top-level directory that is already
/// the JDK root. macOS archives contain `<name>.jdk/Contents/Home`, and that
/// inner Home directory is the root with `bin/java`, `bin/javac`, and `release`.
fn find_extracted_jdk_root(dir: &Path, os: &str) -> PathBuf {
    let single_child = find_single_subdir(dir);
    if os == "mac" {
        if let Some(bundle) = single_child.as_ref() {
            let home = bundle.join("Contents").join("Home");
            if looks_like_jdk_root(&home) {
                return home;
            }
        }
    }
    single_child.unwrap_or_else(|| dir.to_path_buf())
}

/// If a directory contains exactly one subdirectory, return it.
/// Adoptium .tar.gz archives contain a single top-level dir like `jdk-25.0.3+9/`.
fn find_single_subdir(dir: &Path) -> Option<PathBuf> {
    let mut children = Vec::new();
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                children.push(path);
            }
        }
    }
    if children.len() == 1 {
        Some(children.into_iter().next().unwrap())
    } else {
        None
    }
}

/// Detect the current OS and architecture for Adoptium API params.
fn detect_platform() -> anyhow::Result<(&'static str, &'static str)> {
    let os = if cfg!(target_os = "linux") {
        // Check for musl/Alpine
        if cfg!(target_env = "musl") {
            "alpine-linux"
        } else {
            "linux"
        }
    } else if cfg!(target_os = "macos") {
        "mac"
    } else if cfg!(target_os = "windows") {
        "windows"
    } else {
        return Err(anyhow!("unsupported operating system"));
    };

    let arch = if cfg!(target_arch = "x86_64") {
        "x64"
    } else if cfg!(target_arch = "aarch64") {
        "aarch64"
    } else if cfg!(target_arch = "arm") {
        "arm"
    } else if cfg!(target_arch = "riscv64") {
        "riscv64"
    } else if cfg!(target_arch = "powerpc64") && !cfg!(target_endian = "little") {
        "ppc64"
    } else if cfg!(target_arch = "powerpc64") && cfg!(target_endian = "little") {
        "ppc64le"
    } else if cfg!(target_arch = "s390x") {
        "s390x"
    } else {
        return Err(anyhow!("unsupported architecture"));
    };

    Ok((os, arch))
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// The jbx JDK cache directory: ~/.cache/jbx/jdks/
fn jdk_cache_dir() -> anyhow::Result<PathBuf> {
    let cache = dirs::cache_dir().ok_or_else(|| anyhow!("cannot determine cache directory"))?;
    Ok(cache.join("jbx").join("jdks"))
}

/// Get the java binary path for a JDK root.
pub fn java_bin_path(jdk_root: &Path) -> PathBuf {
    if cfg!(windows) {
        jdk_root.join("bin").join("java.exe")
    } else {
        jdk_root.join("bin").join("java")
    }
}

/// Get the javac binary path for a JDK root.
pub fn javac_bin_path(jdk_root: &Path) -> PathBuf {
    if cfg!(windows) {
        jdk_root.join("bin").join("javac.exe")
    } else {
        jdk_root.join("bin").join("javac")
    }
}

/// Get the javadoc binary path for a JDK root.
pub fn javadoc_bin_path(jdk_root: &Path) -> PathBuf {
    if cfg!(windows) {
        jdk_root.join("bin").join("javadoc.exe")
    } else {
        jdk_root.join("bin").join("javadoc")
    }
}

/// Remove a stale cache entry that is not a valid JDK root.
fn remove_stale_cache_entry(path: &Path) -> anyhow::Result<()> {
    if path.is_symlink() || path.is_file() {
        fs::remove_file(path)?;
    } else if path.is_dir() {
        fs::remove_dir_all(path)?;
    }
    Ok(())
}

/// Create a symlink from `link` → `target` for directories.
/// Falls back to a junction/rename if symlinks aren't supported.
fn create_symlink_dir(target: &Path, link: &Path) -> anyhow::Result<()> {
    if link.exists() {
        return Ok(());
    }
    // Remove a broken symlink so the cache entry can be replaced.
    if link.is_symlink() {
        fs::remove_file(link)
            .with_context(|| format!("failed to remove broken symlink {}", link.display()))?;
    }
    // Ensure parent exists
    if let Some(parent) = link.parent() {
        fs::create_dir_all(parent)?;
    }
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(target, link)
            .with_context(|| format!("symlink {} → {}", link.display(), target.display()))?;
    }
    #[cfg(windows)]
    {
        // On Windows, directory symlinks may require privileges. If creating the
        // symlink fails, copy the JDK root so the cache entry is still usable.
        if std::os::windows::fs::symlink_dir(target, link).is_err() {
            copy_dir_recursive(target, link)?;
        }
    }
    Ok(())
}

#[cfg(windows)]
fn copy_dir_recursive(source: &Path, target: &Path) -> anyhow::Result<()> {
    fs::create_dir_all(target)
        .with_context(|| format!("failed to create directory {}", target.display()))?;
    for entry in fs::read_dir(source)
        .with_context(|| format!("failed to read directory {}", source.display()))?
    {
        let entry = entry?;
        let source_path = entry.path();
        let target_path = target.join(entry.file_name());
        if source_path.is_dir() {
            copy_dir_recursive(&source_path, &target_path)?;
        } else {
            fs::copy(&source_path, &target_path).with_context(|| {
                format!(
                    "failed to copy {} to {}",
                    source_path.display(),
                    target_path.display()
                )
            })?;
        }
    }
    Ok(())
}

/// Resolve the JDK for a given `java_version` directive.
///
/// This is the main entry point called from `build_java` / `run_java`.
/// Returns the JDK root path.
pub fn resolve_jdk(java_version: &Option<String>, auto_install: bool) -> anyhow::Result<PathBuf> {
    let major = match java_version {
        Some(v) => parse_java_version_directive(v)?,
        None => default_java_version(),
    };
    find_jdk(major, auto_install)
}

/// The default Java version when none is specified.
/// Defaults to 25 (Java 25 LTS) per jbx's baseline.
fn default_java_version() -> u32 {
    25
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_major_from_version_string() {
        assert_eq!(parse_major_from_version_string("21.0.3"), Some(21));
        assert_eq!(parse_major_from_version_string("1.8.0_432"), Some(8));
        assert_eq!(parse_major_from_version_string("17.0.11+10"), Some(17));
        assert_eq!(parse_major_from_version_string("25"), Some(25));
    }

    #[test]
    fn test_parse_java_version_directive_accepts_jbang_range_suffixes() {
        assert_eq!(parse_java_version_directive("25+").unwrap(), 25);
        assert_eq!(parse_java_version_directive("17.0.11+10").unwrap(), 17);
        assert_eq!(parse_java_version_directive("1.8+").unwrap(), 8);
    }

    #[test]
    fn test_parse_major_from_java_version_output_ignores_tool_options() {
        let output = "Picked up JAVA_TOOL_OPTIONS: -Xmx4g\nopenjdk version \"21.0.3\" 2024-04-16\nOpenJDK Runtime Environment\n";
        assert_eq!(parse_major_from_java_version_output(output), Some(21));
    }

    #[test]
    fn test_detect_platform() {
        // Just verify it returns something valid on this machine
        let (os, arch) = detect_platform().unwrap();
        assert!(matches!(os, "linux" | "mac" | "windows" | "alpine-linux"));
        assert!(matches!(
            arch,
            "x64" | "aarch64" | "arm" | "riscv64" | "ppc64" | "ppc64le" | "s390x"
        ));
    }

    #[test]
    fn test_find_extracted_jdk_root_handles_macos_bundle() {
        let tmp = tempfile::tempdir().unwrap();
        let home = tmp.path().join("jdk-25.jdk").join("Contents").join("Home");
        fs::create_dir_all(home.join("bin")).unwrap();
        fs::write(java_bin_path(&home), "").unwrap();
        fs::write(javac_bin_path(&home), "").unwrap();
        fs::write(home.join("release"), "JAVA_VERSION=\"25\"\n").unwrap();

        assert_eq!(find_extracted_jdk_root(tmp.path(), "mac"), home);
    }

    #[test]
    #[cfg(unix)]
    fn test_create_symlink_dir_replaces_broken_symlink() {
        let tmp = tempfile::tempdir().unwrap();
        let target = tmp.path().join("jdk-25");
        fs::create_dir_all(target.join("bin")).unwrap();
        fs::write(java_bin_path(&target), "").unwrap();
        fs::write(javac_bin_path(&target), "").unwrap();
        fs::write(target.join("release"), "JAVA_VERSION=\"25\"\n").unwrap();

        let link = tmp.path().join("cache").join("25");
        fs::create_dir_all(link.parent().unwrap()).unwrap();
        std::os::unix::fs::symlink(tmp.path().join("missing-jdk"), &link).unwrap();
        assert!(link.is_symlink());
        assert!(!link.exists());

        create_symlink_dir(&target, &link).unwrap();

        assert!(link.exists());
        assert!(looks_like_jdk_root(&link));
    }

    #[test]
    fn test_default_java_version() {
        assert_eq!(default_java_version(), 25);
    }
}
