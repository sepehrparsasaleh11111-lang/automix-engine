pub mod error;
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
