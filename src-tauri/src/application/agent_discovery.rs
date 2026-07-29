use std::{
    collections::HashMap,
    env,
    path::PathBuf,
    sync::atomic::{AtomicBool, Ordering},
};

use crate::{
    adapters::{
        agents::built_in_adapters,
        custom_agent::custom_adapters,
    },
    application::agent_overrides,
    domain::agent::AgentCandidate,
    ports::DetectionContext,
};

static DISCOVERY_CANCELLED: AtomicBool = AtomicBool::new(false);

pub fn cancel_discovery() {
    DISCOVERY_CANCELLED.store(true, Ordering::Release);
}

pub fn discover_agents(
    manual_skill_paths: HashMap<String, PathBuf>,
) -> Result<Vec<AgentCandidate>, String> {
    DISCOVERY_CANCELLED.store(false, Ordering::Release);

    // Load persisted overrides and merge skill_path_override into manual_skill_paths
    let overrides = agent_overrides::load_overrides();
    let mut merged_paths = manual_skill_paths;
    for o in &overrides {
        if let Some(ref p) = o.skill_path_override {
            merged_paths
                .entry(o.agent_type.clone())
                .or_insert_with(|| PathBuf::from(p));
        }
    }

    let context = collect_context(merged_paths)?;
    let mut candidates = Vec::new();

    for adapter in built_in_adapters() {
        if DISCOVERY_CANCELLED.load(Ordering::Acquire) {
            return Err("Agent discovery was cancelled.".to_owned());
        }
        candidates.extend(adapter.detect(&context));
    }

    let custom = custom_adapters(&overrides);
    for adapter in custom {
        if DISCOVERY_CANCELLED.load(Ordering::Acquire) {
            return Err("Agent discovery was cancelled.".to_owned());
        }
        candidates.extend(adapter.detect(&context));
    }

    candidates.sort_by(|left, right| {
        right
            .confidence
            .cmp(&left.confidence)
            .then_with(|| left.display_name.cmp(&right.display_name))
    });
    Ok(candidates)
}

pub fn collect_context(
    manual_skill_paths: HashMap<String, PathBuf>,
) -> Result<DetectionContext, String> {
    let home_dir = env::var_os("USERPROFILE")
        .or_else(|| env::var_os("HOME"))
        .map(PathBuf::from)
        .ok_or_else(|| "The current user home directory could not be resolved.".to_owned())?;

    Ok(DetectionContext {
        home_dir,
        app_data: env::var_os("APPDATA").map(PathBuf::from),
        local_app_data: env::var_os("LOCALAPPDATA").map(PathBuf::from),
        program_files: env::var_os("ProgramFiles").map(PathBuf::from),
        program_files_x86: env::var_os("ProgramFiles(x86)").map(PathBuf::from),
        path_entries: env::var_os("PATH")
            .as_deref()
            .map(env::split_paths)
            .into_iter()
            .flatten()
            .collect(),
        running_processes: running_processes(),
        manual_skill_paths,
    })
}

#[cfg(windows)]
fn running_processes() -> Vec<String> {
    use std::mem::{size_of, zeroed};
    use windows_sys::Win32::{
        Foundation::{CloseHandle, INVALID_HANDLE_VALUE},
        System::Diagnostics::ToolHelp::{
            CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
            TH32CS_SNAPPROCESS,
        },
    };

    let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) };
    if snapshot == INVALID_HANDLE_VALUE {
        return Vec::new();
    }

    let mut processes = Vec::new();
    let mut entry: PROCESSENTRY32W = unsafe { zeroed() };
    entry.dwSize = size_of::<PROCESSENTRY32W>() as u32;

    let mut has_entry = unsafe { Process32FirstW(snapshot, &mut entry) } != 0;
    while has_entry {
        let length = entry
            .szExeFile
            .iter()
            .position(|character| *character == 0)
            .unwrap_or(entry.szExeFile.len());
        processes.push(String::from_utf16_lossy(&entry.szExeFile[..length]));
        has_entry = unsafe { Process32NextW(snapshot, &mut entry) } != 0;
    }

    unsafe {
        CloseHandle(snapshot);
    }
    processes
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, time::Instant};

    use crate::domain::agent::AgentKind;

    use super::discover_agents;

    #[test]
    fn system_scan_returns_all_built_in_agents_within_budget() {
        let started = Instant::now();
        let candidates = discover_agents(HashMap::new()).expect("system discovery should not fail");
        let built_in_count = candidates
            .iter()
            .filter(|candidate| !matches!(&candidate.kind, AgentKind::Custom(_)))
            .count();

        // Persisted Custom Agent definitions are legitimate process state and
        // must not make this built-in registry test environment-dependent.
        assert_eq!(built_in_count, 4);
        // Budget is deliberately generous: under parallel `cargo test` load the
        // real PATH + process snapshot can briefly exceed a tight limit. The
        // invariant we care about is "all four adapters, sorted, and fast in
        // absolute terms" — not a sub-3s wall clock under contention.
        assert!(started.elapsed().as_secs_f32() < 10.0);
        assert!(candidates
            .windows(2)
            .all(|pair| pair[0].confidence >= pair[1].confidence));
    }
}

#[cfg(not(windows))]
fn running_processes() -> Vec<String> {
    use std::process::Command;

    Command::new("ps")
        .args(["-A", "-o", "comm="])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| {
            String::from_utf8_lossy(&output.stdout)
                .lines()
                .map(|line| line.trim().to_owned())
                .filter(|line| !line.is_empty())
                .collect()
        })
        .unwrap_or_default()
}
