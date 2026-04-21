//! Brosdk SDK Demo Application

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;

fn main() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    tracing::info!("Starting Brosdk Demo Application");

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(commands::AppState::default())
        .invoke_handler(tauri::generate_handler![
            commands::init_sdk,
            commands::refresh_user_sig,
            commands::create_env,
            commands::create_env_http,
            commands::start_env,
            commands::stop_env,
            commands::get_sdk_info,
            commands::list_envs,
            commands::list_envs2,
            commands::download_sdk_lib,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
