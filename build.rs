use std::process::Command;

fn main() {
    let version = get_git_tag()
        .or_else(get_date_hash_version)
        .unwrap_or_else(|| "unknown".to_string());

    println!("cargo:rustc-env=VERSION={}", version);

    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-changed=.git/refs/tags");
}

fn get_git_tag() -> Option<String> {
    let output = Command::new("git")
        .args(["describe", "--exact-match", "--tags", "HEAD"])
        .output()
        .ok()?;

    if output.status.success() {
        let tag = String::from_utf8(output.stdout).ok()?;
        Some(tag.trim().to_string())
    } else {
        None
    }
}

fn get_date_hash_version() -> Option<String> {
    let date = Command::new("date")
        .arg("+%Y%m%d")
        .output()
        .ok()
        .and_then(|output| {
            if output.status.success() {
                String::from_utf8(output.stdout).ok()
            } else {
                None
            }
        })?;

    let hash = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .and_then(|output| {
            if output.status.success() {
                String::from_utf8(output.stdout).ok()
            } else {
                None
            }
        })?;

    Some(format!("{}-{}", date.trim(), hash.trim()))
}
