pub mod capture;
mod commands;
mod domain;
#[cfg(debug_assertions)]
pub mod qa_math;
#[cfg(debug_assertions)]
pub mod qa_ocr;
#[cfg(debug_assertions)]
pub mod qa_revisions;
pub mod services;
mod storage;

pub fn run() {
    #[cfg(debug_assertions)]
    if std::env::var_os("RESEARCHCLIP_TEST_SETTINGS_DIR").is_some()
        && std::env::var_os("RESEARCHCLIP_TEST_CAPTURE_FIXTURE").is_some()
    {
        run_workflow_fixture();
        return;
    }
    run_inner(None);
}

// Separate acceptance executable only; normal app startup never seeds captures.
#[cfg(debug_assertions)]
fn run_workflow_fixture() {
    let mut rgba = Vec::with_capacity(320 * 200 * 4);
    for y in 0..200 {
        for x in 0..320 {
            rgba.extend_from_slice(&[(x % 256) as u8, (y % 256) as u8, 180, 255]);
        }
    }
    run_inner(Some(capture::RawFrame {
        width_px: 320,
        height_px: 200,
        rgba,
    }));
}
fn run_inner(fixture: Option<capture::RawFrame>) {
    use std::sync::Arc;
    use tauri::Manager;
    use tauri_plugin_global_shortcut::ShortcutState;
    let context = tauri::generate_context!();
    #[cfg(debug_assertions)]
    let context = {
        let mut context = context;
        if let Some(path) = std::env::var_os("RESEARCHCLIP_TEST_SETTINGS_DIR") {
            use std::hash::{Hash, Hasher};
            let mut hash = std::collections::hash_map::DefaultHasher::new();
            path.hash(&mut hash);
            context.config_mut().identifier =
                format!("io.researchclip.test.t{:016x}", hash.finish());
            context.config_mut().app.app_directories_override =
                Some(tauri::utils::config::AppDirectoriesOverride::Root(
                    std::path::PathBuf::from(path).join("tauri-runtime"),
                ));
        }
        context
    };
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            if let Err(error) = services::desktop::show_main(app) {
                services::desktop::notice(app, &error.message);
            }
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(
            tauri_plugin_opener::Builder::new()
                .open_js_links_on_click(false)
                .build(),
        )
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, shortcut, event| {
                    if event.state == ShortcutState::Pressed
                        && app.state::<services::desktop::DesktopService>().running()
                        && let Some(action) = app
                            .state::<Arc<services::settings::SettingsService>>()
                            .action_for(shortcut.id())
                    {
                        commands::workflow::shortcut_capture(app.clone(), action);
                    }
                })
                .build(),
        )
        .manage(services::desktop::DesktopService::default())
        .manage(Arc::new(commands::references::ReferenceNetwork::default()))
        .manage(Arc::new(
            services::window_capture::WindowCaptureService::default(),
        ))
        .manage(Arc::new(services::sync::runtime::SyncService::default()))
        .manage(Arc::new(services::ocr_index::OcrIndexService::default()))
        .manage(Arc::new(
            services::similarity_index::SimilarityIndex::default(),
        ))
        .manage(services::clipper::ClipperService::default())
        .manage(Arc::new(services::scrolling::ScrollService::default()))
        .manage(Arc::new(services::pins::PinManager::default()))
        .manage(Arc::new(std::sync::Mutex::new(
            services::markdown_document::DocumentFiles::default(),
        )))
        .manage(Arc::new(services::workflow::CaptureWorkflow::default()))
        .manage(std::sync::Arc::new(
            services::screen_capture::ScreenCaptureService::default(),
        ))
        .manage(std::sync::Arc::new(
            services::region_capture::RegionCaptureService::default(),
        ))
        .setup(move |app| {
            app.state::<Arc<services::pins::PinManager>>()
                .start_watchdog(app.handle());
            if let Some(frame) = fixture {
                app.state::<Arc<services::workflow::CaptureWorkflow>>()
                    .store(
                        "acceptance-fixture".into(),
                        "fullscreen",
                        services::screen_capture::encode_png(frame)?,
                    )?;
            }
            let path = if cfg!(debug_assertions) {
                std::env::var_os("RESEARCHCLIP_TEST_SETTINGS_DIR").map(std::path::PathBuf::from)
            } else {
                None
            }
            .unwrap_or(app.path().app_config_dir()?);
            app.manage(Arc::new(std::sync::Mutex::new(
                services::templates::TemplateFiles::new(path.join("templates-v1")),
            )));
            let settings = Arc::new(services::settings::SettingsService::load(
                path.join("settings.json"),
            ));
            let data_root = if cfg!(debug_assertions) {
                std::env::var_os("RESEARCHCLIP_TEST_SETTINGS_DIR")
                    .map(|p| std::path::PathBuf::from(p).join("workspace"))
            } else {
                None
            }
            .unwrap_or(app.path().app_data_dir()?);
            let math_root = path.join("math-ocr-v1");
            let math =
                services::math_ocr::MathOcrService::new(math_root.clone()).unwrap_or_else(|e| {
                    services::math_ocr::MathOcrService::unavailable(math_root, e.message)
                });
            app.manage(Arc::new(math));
            app.manage(Arc::new(services::similarity_ai::SimilarityAi::new(
                path.join("similarity-ai-v1.json"),
                app.path().resource_dir()?.join("similarity-ai"),
            )));
            app.manage(Arc::new(commands::library::LibraryService::new(data_root)));
            app.manage(Arc::clone(&settings));
            if let Err(error) = services::desktop::install_tray(app.handle()) {
                services::desktop::notice(app.handle(), &error);
            }
            let handle = app.handle().clone();
            tauri::async_runtime::spawn_blocking(move || {
                if settings
                    .initialize(|k, e| commands::workflow::registration(&handle, k, e))
                    .is_err()
                {
                    eprintln!("ResearchClip: settings initialization failed");
                }
            });
            Ok(())
        })
        .on_window_event(|window, event| {
            if matches!(event, tauri::WindowEvent::Destroyed) && window.label().starts_with("pin-")
            {
                window
                    .app_handle()
                    .state::<Arc<services::pins::PinManager>>()
                    .release(window.label());
            }
            if window.label() == "main"
                && let tauri::WindowEvent::CloseRequested { api, .. } = event
                && !window
                    .state::<services::desktop::DesktopService>()
                    .closing()
            {
                api.prevent_close();
                let app = window.app_handle();
                let hide = window
                    .state::<Arc<services::settings::SettingsService>>()
                    .close_to_tray()
                    && app.tray_by_id(services::desktop::TRAY_ID).is_some();
                let result = if hide {
                    services::desktop::dispatch(app, services::desktop::DesktopAction::Hide)
                } else {
                    services::desktop::request_exit(app)
                };
                if let Err(error) = result {
                    services::desktop::notice(app, &error.message);
                }
            }
            if window.label() == "main" && matches!(event, tauri::WindowEvent::Destroyed) {
                window
                    .state::<Arc<services::scrolling::ScrollService>>()
                    .cancel();
                if let Err(error) = window
                    .state::<Arc<services::pins::PinManager>>()
                    .close_all(window.app_handle())
                {
                    services::desktop::notice(window.app_handle(), &error.message);
                }
                window
                    .state::<std::sync::Arc<services::region_capture::RegionCaptureService>>()
                    .cancel_active();
                window
                    .state::<Arc<services::window_capture::WindowCaptureService>>()
                    .cancel_active();
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::similarity_ai::similarity_ai_status,
            commands::similarity_ai::choose_similarity_ai_bundle,
            commands::similarity_ai::start_similarity_ai,
            commands::similarity_ai::cancel_similarity_ai,
            commands::similarity::similarity_status,
            commands::references::lookup_reference_doi,
            commands::references::reference_page,
            commands::references::save_references,
            commands::references::associate_reference,
            commands::references::import_bibtex,
            commands::references::open_reference_doi,
            commands::similarity::start_similarity_index,
            commands::similarity::cancel_similarity_index,
            commands::similarity::find_similar_images,
            commands::similarity::duplicate_image_groups,
            commands::sync::sync_status,
            commands::sync::sync_configure,
            commands::sync::sync_start,
            commands::sync::sync_stop,
            commands::sync::sync_resolve,
            commands::sync::sync_authorize_documents,
            commands::clipper::clipper_status,
            commands::clipper::clipper_connection,
            commands::scrolling::scroll_targets,
            commands::scrolling::scroll_prepare,
            commands::scrolling::scroll_start,
            commands::scrolling::scroll_status,
            commands::scrolling::scroll_action,
            commands::revisions::annotation_history,
            commands::revisions::annotation_revision,
            commands::revisions::save_annotation_revision,
            commands::revisions::restore_annotation_revision,
            commands::ocr_index::get_ocr_content,
            commands::ocr_index::correct_ocr_content,
            commands::ocr_index::start_ocr_index,
            commands::ocr_index::ocr_index_status,
            commands::ocr_index::ocr_index_action,
            commands::ocr_index::rebuild_search_index,
            commands::math_ocr::math_ocr_status,
            commands::math_ocr::cancel_math_ocr,
            commands::math_ocr::download_math_models,
            commands::math_ocr::choose_math_runtime,
            commands::math_ocr::import_math_models,
            commands::math_ocr::recognize_math_ocr,
            commands::ocr::ocr_status,
            commands::ocr::recognize_ocr,
            commands::ocr::copy_ocr_text,
            commands::pins::create_pin,
            commands::pins::pin_image,
            commands::pins::pin_action,
            commands::pins::recover_pins,
            commands::pins::pin_status,
            commands::test_connection,
            commands::get_workspace_status,
            commands::capture::list_displays,
            commands::capture::capture_fullscreen,
            commands::window_capture::capture_window,
            commands::window_capture::window_picker_status,
            commands::window_capture::window_picker_action,
            commands::region::capture_region,
            commands::region::region_overlay_info,
            commands::region::region_overlay_image,
            commands::region::ready_region_overlay,
            commands::region::commit_region_selection,
            commands::region::cancel_region_selection,
            commands::region::region_result_png,
            commands::workflow::get_preferences,
            commands::desktop::desktop_status,
            commands::desktop::desktop_action,
            commands::desktop::complete_desktop_exit,
            commands::workflow::update_preferences,
            commands::workflow::choose_save_directory,
            commands::workflow::capture_result_png,
            commands::workflow::current_capture,
            commands::workflow::discard_capture,
            commands::workflow::copy_capture_image,
            commands::workflow::save_capture_png,
            commands::editor::save_editor_project,
            commands::editor::open_editor_project,
            commands::editor::export_editor_png,
            commands::editor::copy_editor_png,
            commands::import::choose_comparison_image,
            commands::projects::list_projects,
            commands::projects::save_project,
            commands::projects::delete_empty_project,
            commands::projects::select_library_screenshots,
            commands::projects::batch_library_screenshots,
            commands::projects::list_project_documents,
            commands::projects::assign_project_document,
            commands::projects::sync_project_document,
            commands::projects::open_project_document,
            commands::import::import_clipboard_image,
            commands::import::import_image_files,
            commands::document::save_markdown_document,
            commands::document::open_markdown_document,
            commands::document::open_markdown_external,
            commands::document::release_markdown_document,
            commands::document::rename_markdown_document,
            commands::document::list_markdown_templates,
            commands::document::load_markdown_template,
            commands::document::create_markdown_template,
            commands::document::rename_markdown_template,
            commands::document::delete_markdown_template,
            commands::library::create_screenshot,
            commands::library::library_status,
            commands::library::reconcile_library,
            commands::library::list_screenshots,
            commands::library::screenshot_page,
            commands::library::get_knowledge_card,
            commands::markdown::export_knowledge_markdown,
            commands::library::save_knowledge_card,
            commands::library::open_card_source,
            commands::library::get_screenshot,
            commands::library::screenshot_image,
            commands::library::update_screenshot,
            commands::library::delete_screenshot,
            commands::library::list_notes,
            commands::library::save_note,
            commands::library::delete_note,
            commands::library::list_tags,
            commands::library::save_tag,
            commands::library::delete_tag,
            commands::library::get_screenshot_tags,
            commands::library::set_screenshot_tags
        ])
        .build(context)
        .expect("ResearchClip failed to start")
        .run(|app, event| match event {
            tauri::RunEvent::ExitRequested { api, .. }
                if !app.state::<services::desktop::DesktopService>().closing() =>
            {
                api.prevent_exit();
                if let Err(error) = services::desktop::request_exit(app) {
                    services::desktop::notice(app, &error.message);
                }
            }
            tauri::RunEvent::Exit => {
                let _ = app
                    .state::<Arc<services::math_ocr::MathOcrService>>()
                    .cancel();
                app.state::<Arc<services::ocr_index::OcrIndexService>>()
                    .cancel();
                app.state::<services::clipper::ClipperService>().stop();
                app.remove_tray_by_id(services::desktop::TRAY_ID);
            }
            _ => {}
        });
}
