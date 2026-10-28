//! Where the app keeps its data.

use std::path::PathBuf;

/// Environment variable that overrides the data directory (e2e tests,
/// portable setups). Unset means the OS app-data directory.
pub const DATA_DIR_ENV: &str = "FREELANCE_CRM_DATA_DIR";

pub fn data_dir(override_dir: Option<std::ffi::OsString>, default: PathBuf) -> PathBuf {
    override_dir
        .filter(|dir| !dir.is_empty())
        .map(PathBuf::from)
        .unwrap_or(default)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uses_the_os_directory_by_default() {
        assert_eq!(
            data_dir(None, PathBuf::from("/os/app")),
            PathBuf::from("/os/app")
        );
    }

    #[test]
    fn env_override_wins() {
        assert_eq!(
            data_dir(Some("/tmp/e2e".into()), PathBuf::from("/os/app")),
            PathBuf::from("/tmp/e2e")
        );
    }

    #[test]
    fn empty_override_is_ignored() {
        assert_eq!(
            data_dir(Some("".into()), PathBuf::from("/os/app")),
            PathBuf::from("/os/app")
        );
    }
}
