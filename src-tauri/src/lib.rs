pub mod commands;
pub mod core;
pub mod db;
pub mod models;

use db::Database;
use std::fs;
use tauri::webview::PageLoadEvent;
use tauri::Manager;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .on_page_load(|webview, payload| {
            if payload.event() == PageLoadEvent::Finished && webview.window().label() == "main" {
                if let Err(error) = webview.window().show() {
                    crate::core::logger::error("App", &format!("主窗口显示失败: {error}"));
                }
            }
        })
        .setup(|app| {
            let app_data_dir = app
                .path()
                .app_data_dir()
                .unwrap_or_else(|_| std::env::temp_dir().join("skills_manager_data"));
            let _ = fs::create_dir_all(&app_data_dir);
            crate::core::logger::init_logger(&app_data_dir.join("logs"));
            let db_path = app_data_dir.join("skills_manager.db");
            let db = Database::open_file(&db_path.to_string_lossy())
                .expect("Failed to initialize SQLite database");

            app.manage(db);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::repo_cmd::repository_get,
            commands::repo_cmd::repository_add,
            commands::repo_cmd::repository_scan,
            commands::repo_cmd::repository_git_status,
            commands::repo_cmd::repository_git_pull,
            commands::project_cmd::project_list,
            commands::project_cmd::project_add,
            commands::project_cmd::project_relocate,
            commands::project_cmd::project_remove,
            commands::project_cmd::project_diagnose,
            commands::project_cmd::open_path_in_explorer,
            commands::mount_cmd::mount_skills,
            commands::mount_cmd::unmount_skills,
            commands::mount_cmd::mount_repair,
            commands::mount_cmd::mount_rollback_batch,
            commands::entry_cmd::entry_link_setup,
            commands::entry_cmd::entry_link_remove,
            commands::log_cmd::operation_log_query,
            commands::log_cmd::log_get_info,
            commands::log_cmd::log_read_text,
            commands::log_cmd::log_clear,
            commands::log_cmd::log_open_dir,
            commands::log_cmd::log_open_file,
            commands::config_cmd::config_export,
            commands::config_cmd::config_import_preview,
            commands::config_cmd::config_import_apply,
            commands::market_cmd::market_search,
            commands::market_cmd::market_install,
            commands::market_cmd::market_uninstall_check,
            commands::market_cmd::market_uninstall,
            commands::market_cmd::market_skill_detail,
            commands::skill_update_cmd::skill_check_updates,
            commands::skill_update_cmd::skill_update_single,
            commands::skill_update_cmd::skill_update_batch,
            commands::skill_update_cmd::skill_backup_list,
            commands::skill_update_cmd::skill_backup_restore,
            commands::skill_update_cmd::skill_backup_delete,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
