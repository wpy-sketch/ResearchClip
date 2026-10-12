mod commands;
mod domain;
mod storage;

pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            commands::test_connection,
            commands::get_workspace_status
        ])
        .run(tauri::generate_context!())
        .expect("ResearchClip failed to start");
}
