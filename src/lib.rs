pub mod assets;
pub mod cli;
pub mod detector;
pub mod http;
pub mod inputs;
pub mod language;
pub mod model_slot;
pub mod types;

pub use cli::{check_one_prompt, run_check_batch, CheckBatchOutcome};
pub use detector::Detector;
pub use language::is_english;
pub use http::{build_router, build_service, mount_openapi};
pub use inputs::{collect_check_items, CheckItem};
pub use model_slot::{DetectorSlot, SharedDetector};
pub use types::{version_info, CheckRequest, CheckResult, VersionInfo, MODEL_ID};

/// Full HTTP service (API + Swagger + `/` redirect).
pub fn app_service(slot: SharedDetector, cors_origins: Vec<String>) -> salvo::Service {
    build_service(mount_openapi(build_router(slot)), cors_origins)
}
