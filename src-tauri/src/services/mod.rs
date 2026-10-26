pub mod desktop;
pub mod editor;
pub mod image_import;
pub mod markdown_document;
pub mod ocr;
pub mod pins;
pub mod references;
pub mod region_capture;
pub mod screen_capture;
pub mod settings;
pub mod templates;
pub mod window_capture;
pub mod workflow;

pub mod clipper;
pub mod scrolling;
#[cfg(target_os = "windows")]
pub mod scrolling_windows;
pub mod stitching;

pub mod ocr_index;

pub mod embeddings;
pub mod math_ocr;
pub mod similarity;
pub mod similarity_ai;
pub mod similarity_index;
pub mod sync;

pub mod pdf;

pub mod pdf_open;

pub mod zotero_local;

pub mod zotero_session;

pub mod zotero_annotations;
pub mod zotero_write_model;
