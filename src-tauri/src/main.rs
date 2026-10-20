#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    if researchclip_lib::services::window_capture::run_capture_worker() {
        return;
    }
    researchclip_lib::run();
}
