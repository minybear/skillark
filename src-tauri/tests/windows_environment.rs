#![cfg(windows)]

use std::path::{Component, Path, PathBuf};

use skillark_lib::{
    adapters::deployment::CopyDriver,
    domain::content_hash::hash_directory,
    ports::{DeploymentDriver, InstallRequest},
};

fn unique_root(base: &Path, tag: &str) -> PathBuf {
    let root = base.join(format!(
        "skillark-{tag}-{}-{}",
        std::process::id(),
        uuid::Uuid::new_v4()
    ));
    std::fs::create_dir_all(&root).unwrap();
    root
}

fn volume(path: &Path) -> Option<String> {
    path.components().find_map(|component| match component {
        Component::Prefix(prefix) => Some(prefix.as_os_str().to_string_lossy().into_owned()),
        _ => None,
    })
}

fn write_skill(source: &Path) -> String {
    std::fs::create_dir_all(source.join("scripts")).unwrap();
    std::fs::write(
        source.join("SKILL.md"),
        "---\nname: windows-environment\nversion: 1.0.0\n---\n# Environment matrix\n",
    )
    .unwrap();
    std::fs::write(source.join("scripts/run.txt"), "cross-volume").unwrap();
    hash_directory(source).unwrap()
}

#[test]
fn copy_deploys_and_verifies_across_available_windows_volumes() {
    let home = PathBuf::from(std::env::var_os("USERPROFILE").expect("USERPROFILE"));
    let current = std::env::current_dir().unwrap();
    if volume(&home) == volume(&current) {
        eprintln!("cross-volume case skipped: USERPROFILE and workspace share one volume");
        return;
    }

    let source_root = unique_root(&home, "cross-volume-source");
    let target_root = unique_root(&current.join("target"), "cross-volume-target");
    let source = source_root.join("source");
    let target = target_root.join("deployed");
    let expected_hash = write_skill(&source);
    let driver = CopyDriver::new();

    let result = driver
        .install(InstallRequest {
            operation_id: "windows-cross-volume".to_owned(),
            source: source.clone(),
            target: target.clone(),
            expected_hash: expected_hash.clone(),
            allow_replace_managed: false,
        })
        .expect("cross-volume copy install");

    assert_eq!(
        result.deployed_hash.as_deref(),
        Some(expected_hash.as_str())
    );
    assert_eq!(hash_directory(&target).unwrap(), expected_hash);
    assert!(target.join("SKILL.md").is_file());

    std::fs::remove_dir_all(&source_root).unwrap();
    std::fs::remove_dir_all(&target_root).unwrap();
}

// 普通 whoami 在中文 Windows 上以系统 ANSI 代码页（GBK）输出「主机名\用户名」，
// 直接按 UTF-8 解码会 panic。这里用 MultiByteToWideChar(CP_ACP) 按 ANSI 代码页
// 解码为 UTF-16，再转 Rust String。icacls 只认帐户名（本机不接受 SID），故须取用户名。
fn ansi_bytes_to_string(bytes: &[u8]) -> String {
    use windows_sys::Win32::Globalization::{MultiByteToWideChar, CP_ACP};
    let nul = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
    let bytes = &bytes[..nul];
    if bytes.is_empty() {
        return String::new();
    }
    unsafe {
        let len = MultiByteToWideChar(CP_ACP, 0, bytes.as_ptr(), bytes.len() as i32, std::ptr::null_mut(), 0);
        assert!(len > 0, "MultiByteToWideChar sizing failed");
        let mut wide = vec![0u16; len as usize];
        let written = MultiByteToWideChar(CP_ACP, 0, bytes.as_ptr(), bytes.len() as i32, wide.as_mut_ptr(), len);
        assert!(written > 0, "MultiByteToWideChar convert failed");
        String::from_utf16_lossy(&wide[..written as usize])
    }
}

#[test]
fn copy_probe_reports_acl_denied_parent_as_unwritable() {
    let home = PathBuf::from(std::env::var_os("USERPROFILE").expect("USERPROFILE"));
    let root = unique_root(&home, "acl-denied");
    let denied_parent = root.join("denied-parent");
    std::fs::create_dir_all(&denied_parent).unwrap();
    let user = ansi_bytes_to_string(
        &std::process::Command::new("whoami")
            .output()
            .expect("run whoami")
            .stdout,
    )
    .trim()
    .to_owned();

    let deny = std::process::Command::new("icacls")
        .arg(&denied_parent)
        .args(["/deny", &format!("{user}:(OI)(CI)(W)")])
        .output()
        .expect("add deny ACL");
    assert!(
        deny.status.success(),
        "icacls deny failed: {}",
        String::from_utf8_lossy(&deny.stderr)
    );

    let probe_result = CopyDriver::new().probe(&denied_parent.join("skill"));

    let restore = std::process::Command::new("icacls")
        .arg(&denied_parent)
        .args(["/remove:d", &user])
        .output()
        .expect("remove deny ACL");
    assert!(
        restore.status.success(),
        "icacls restore failed: {}",
        String::from_utf8_lossy(&restore.stderr)
    );

    let probe = probe_result.expect("probe denied parent");
    assert!(!probe.exists);
    assert!(!probe.writable);
    std::fs::remove_dir_all(&root).unwrap();
}
