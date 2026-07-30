// Localized MSVC link.exe emits a normal "creating library" progress line on stdout.
#![cfg_attr(target_env = "msvc", allow(linker_messages))]

pub mod adapters;
pub mod application;
pub mod commands;
pub mod domain;
pub mod ports;
pub mod repositories;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let state = tauri::async_runtime::block_on(application::state::AppState::setup())
                .map_err(|e| -> Box<dyn std::error::Error> {
                    format!("SkillArk startup failed: {e}").into()
                })?;
            app.manage(state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_bootstrap_status,
            commands::discover_agents,
            commands::cancel_agent_discovery,
            commands::get_agent_overrides,
            commands::save_agent_override,
            commands::delete_agent_override,
            commands::import_skill_from_directory,
            commands::import_skill_from_zip,
            commands::list_skills,
            commands::delete_skill,
            commands::list_skill_versions,
            commands::preview_link,
            commands::import_link_candidate,
            commands::list_deployments,
            commands::plan_deployment,
            commands::execute_deployment,
            commands::verify_deployments,
            commands::uninstall_deployment,
            commands::list_operations,
            commands::backup_database,
            commands::list_workspaces,
            commands::create_project_workspace,
            commands::delete_project_workspace,
            commands::resolve_project_target_paths,
            commands::resolve_global_target_paths,
            commands::get_skill_detail,
            commands::get_disabled_agents,
            commands::set_agent_disabled,
        ])
        .run(tauri::generate_context!())
        .expect("failed to run SkillArk");
}
