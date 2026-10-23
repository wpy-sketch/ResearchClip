//! Internal/native research metadata access. This is not a WebView filesystem or SQL capability.
pub use crate::domain::research::*;
pub use crate::storage::Storage as ResearchRepository;
pub use crate::storage::library::Screenshot as ScreenshotAsset;
pub use crate::storage::projects::Project as ResearchProject;
pub use crate::storage::references::Reference as LiteratureReference;
pub use crate::storage::resources::*;
