pub mod analysis;
pub mod audio;
pub mod beatgrid;
pub mod error;
pub use analysis::energy::{energy_windows, peak_db_of, rms_db_of};
pub use analysis::{AnalysisConfig, AnalysisResult};
pub use beatgrid::{Beat, BeatGrid};
pub use error::AppError;

pub fn engine_name() -> &'static str {
    "openmix-core"
}

#[cfg(test)]
mod tests {
    use crate::error::AppError;

    #[test]
    fn engine_name_is_openmix_core() {
        assert_eq!(crate::engine_name(), "openmix-core");
    }

    #[test]
    fn app_error_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<AppError>();
    }
}
