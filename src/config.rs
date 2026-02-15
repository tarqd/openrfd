use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// Repository-level configuration stored in `rfd.toml`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub rfd: RfdConfig,
    #[serde(default)]
    pub github: GithubConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RfdConfig {
    /// Default format for new RFDs: "md"
    #[serde(default = "default_format")]
    pub default_format: String,

    /// Main branch name
    #[serde(default = "default_main_branch")]
    pub main_branch: String,

    /// Zero-pad width for RFD numbers
    #[serde(default = "default_pad_width")]
    pub pad_width: u32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GithubConfig {
    /// GitHub OAuth App or GitHub App client ID
    #[serde(default)]
    pub app_client_id: String,

    /// If set, enables PKCE web flow instead of device flow
    #[serde(default)]
    pub redirect_uri: Option<String>,
}

fn default_format() -> String {
    "md".into()
}

fn default_main_branch() -> String {
    "main".into()
}

fn default_pad_width() -> u32 {
    4
}

impl Default for RfdConfig {
    fn default() -> Self {
        Self {
            default_format: default_format(),
            main_branch: default_main_branch(),
            pad_width: default_pad_width(),
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            rfd: RfdConfig::default(),
            github: GithubConfig::default(),
        }
    }
}

impl Config {
    /// Load config from `rfd.toml` in the given directory.
    pub fn load(repo_root: &Path) -> Result<Self> {
        let path = repo_root.join("rfd.toml");
        if !path.exists() {
            return Ok(Config::default());
        }
        let contents =
            std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
        let config: Config =
            toml::from_str(&contents).with_context(|| format!("parsing {}", path.display()))?;
        Ok(config)
    }

    /// Write config to `rfd.toml` in the given directory.
    pub fn save(&self, repo_root: &Path) -> Result<()> {
        let path = repo_root.join("rfd.toml");
        let contents = toml::to_string_pretty(self).context("serializing config")?;
        std::fs::write(&path, contents).with_context(|| format!("writing {}", path.display()))?;
        Ok(())
    }

    /// Pad an RFD number to the configured width.
    pub fn pad_number(&self, num: u32) -> String {
        format!("{:0>width$}", num, width = self.rfd.pad_width as usize)
    }

    /// Get the branch name for an RFD.
    pub fn branch_name(&self, num: u32) -> String {
        format!("rfd/{}", self.pad_number(num))
    }

    /// Get the RFD directory path.
    pub fn rfd_dir(&self, repo_root: &Path) -> PathBuf {
        repo_root.join("rfd")
    }

    /// Get the path to a specific RFD directory.
    pub fn rfd_path(&self, repo_root: &Path, num: u32) -> PathBuf {
        repo_root.join("rfd").join(self.pad_number(num))
    }

    /// Get the templates directory path.
    pub fn templates_dir(&self, repo_root: &Path) -> PathBuf {
        repo_root.join("templates")
    }

    /// Get the static files directory path.
    pub fn static_dir(&self, repo_root: &Path) -> PathBuf {
        repo_root.join("static")
    }

    /// Get the custom ref name for an RFD's metadata store.
    /// Returns e.g. `refs/rfd/0042`.
    pub fn ref_name(&self, num: u32) -> String {
        format!("refs/rfd/{}", self.pad_number(num))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pad_number_default_width() {
        let config = Config::default();
        assert_eq!(config.pad_number(1), "0001");
        assert_eq!(config.pad_number(42), "0042");
        assert_eq!(config.pad_number(9999), "9999");
        assert_eq!(config.pad_number(10000), "10000");
    }

    #[test]
    fn test_pad_number_custom_width() {
        let mut config = Config::default();
        config.rfd.pad_width = 6;
        assert_eq!(config.pad_number(1), "000001");
        assert_eq!(config.pad_number(42), "000042");
    }

    #[test]
    fn test_branch_name() {
        let config = Config::default();
        assert_eq!(config.branch_name(42), "rfd/0042");
        assert_eq!(config.branch_name(1), "rfd/0001");
    }

    #[test]
    fn test_ref_name() {
        let config = Config::default();
        assert_eq!(config.ref_name(42), "refs/rfd/0042");
        assert_eq!(config.ref_name(1), "refs/rfd/0001");
    }

    #[test]
    fn test_rfd_path() {
        let config = Config::default();
        let root = Path::new("/repo");
        assert_eq!(config.rfd_path(root, 42), PathBuf::from("/repo/rfd/0042"));
        assert_eq!(config.rfd_dir(root), PathBuf::from("/repo/rfd"));
    }

    #[test]
    fn test_default_values() {
        let config = Config::default();
        assert_eq!(config.rfd.default_format, "md");
        assert_eq!(config.rfd.main_branch, "main");
        assert_eq!(config.rfd.pad_width, 4);
        assert!(config.github.app_client_id.is_empty());
        assert!(config.github.redirect_uri.is_none());
    }

    #[test]
    fn test_load_save_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let mut config = Config::default();
        config.rfd.pad_width = 6;
        config.github.app_client_id = "test-client-id".into();
        config.github.redirect_uri = Some("http://localhost:3000/callback".into());

        config.save(dir.path()).unwrap();
        let loaded = Config::load(dir.path()).unwrap();

        assert_eq!(loaded.rfd.pad_width, 6);
        assert_eq!(loaded.rfd.default_format, "md");
        assert_eq!(loaded.github.app_client_id, "test-client-id");
        assert_eq!(
            loaded.github.redirect_uri.as_deref(),
            Some("http://localhost:3000/callback")
        );
    }

    #[test]
    fn test_load_missing_file_returns_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let config = Config::load(dir.path()).unwrap();
        assert_eq!(config.rfd.pad_width, 4);
        assert_eq!(config.rfd.main_branch, "main");
    }

    #[test]
    fn test_load_partial_toml() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("rfd.toml"),
            "[rfd]\npad_width = 5\n",
        )
        .unwrap();
        let config = Config::load(dir.path()).unwrap();
        assert_eq!(config.rfd.pad_width, 5);
        assert_eq!(config.rfd.main_branch, "main"); // default preserved
    }
}
