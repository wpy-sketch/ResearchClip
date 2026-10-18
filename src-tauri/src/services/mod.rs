pub mod desktop;
pub mod editor;
pub mod image_import;
pub mod markdown_document;
pub mod ocr;
pub mod pins;
pub mod region_capture;
pub mod screen_capture;
pub mod settings;
pub mod templates;
pub mod workflow;

pub mod scrolling;
#[cfg(target_os = "windows")]
pub mod scrolling_windows;
pub mod stitching;
