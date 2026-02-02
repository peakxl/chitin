//! Help cache module for storing and retrieving cached help output from Node.js openclaw.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Cache file structure - stores help for main command and all subcommands
#[derive(Serialize, Deserialize, Default)]
struct CacheFile {
    /// OpenClaw version that generated this cache
    openclaw_version: String,
    /// Chitin version that generated this cache
    chitin_version: String,
    /// Timestamp when cache was created (Unix epoch seconds)
    timestamp: u64,
    /// Help text for each command (empty string key = main help)
    commands: HashMap<String, String>,
}

/// Help cache manager
pub struct HelpCache {
    cache_path: PathBuf,
}

impl HelpCache {
    /// Create a new HelpCache instance
    pub fn new() -> Result<Self> {
        let cache_dir = Self::get_cache_dir()?;
        fs::create_dir_all(&cache_dir).context("Failed to create cache directory")?;

        Ok(Self {
            cache_path: cache_dir.join("help_cache.json"),
        })
    }

    /// Get the cache directory path
    fn get_cache_dir() -> Result<PathBuf> {
        // Prefer ~/.chitin/cache
        if let Some(home) = dirs::home_dir() {
            return Ok(home.join(".chitin").join("cache"));
        }

        // Fallback to XDG cache
        if let Some(cache) = dirs::cache_dir() {
            return Ok(cache.join("chitin"));
        }

        anyhow::bail!("Cannot determine cache directory")
    }

    /// Load the cache file, returning None if invalid or expired
    fn load_cache(
        &self,
        expected_openclaw_version: &str,
        expected_chitin_version: &str,
    ) -> Result<Option<CacheFile>> {
        if !self.cache_path.exists() {
            return Ok(None);
        }

        let content = fs::read_to_string(&self.cache_path).context("Failed to read cache file")?;

        let cache: CacheFile = match serde_json::from_str(&content) {
            Ok(c) => c,
            Err(_) => {
                // Invalid cache, delete it
                let _ = fs::remove_file(&self.cache_path);
                return Ok(None);
            }
        };

        // Check version match (both openclaw and chitin versions must match)
        if cache.openclaw_version != expected_openclaw_version {
            return Ok(None);
        }
        if !cache.chitin_version.is_empty() && cache.chitin_version != expected_chitin_version {
            return Ok(None);
        }

        // Check cache age (invalidate after 24 hours)
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::ZERO)
            .as_secs();

        let max_age = 24 * 60 * 60; // 24 hours
        if now - cache.timestamp > max_age {
            return Ok(None);
        }

        Ok(Some(cache))
    }

    /// Save the cache file
    fn save_cache(&self, cache: &CacheFile) -> Result<()> {
        let content = serde_json::to_string_pretty(cache).context("Failed to serialize cache")?;
        fs::write(&self.cache_path, content).context("Failed to write cache file")?;
        Ok(())
    }

    /// Get cached help for main command if valid
    pub fn get_cached_help(
        &self,
        openclaw_version: &str,
        chitin_version: &str,
    ) -> Result<Option<String>> {
        self.get_cached_subcommand_help("", openclaw_version, chitin_version)
    }

    /// Get cached help for a subcommand if valid
    /// Use empty string for main help
    pub fn get_cached_subcommand_help(
        &self,
        subcommand: &str,
        openclaw_version: &str,
        chitin_version: &str,
    ) -> Result<Option<String>> {
        let cache = match self.load_cache(openclaw_version, chitin_version)? {
            Some(c) => c,
            None => return Ok(None),
        };

        Ok(cache.commands.get(subcommand).cloned())
    }

    /// Save help text for main command to cache
    pub fn save_help(
        &self,
        help_text: &str,
        openclaw_version: &str,
        chitin_version: &str,
    ) -> Result<()> {
        self.save_subcommand_help("", help_text, openclaw_version, chitin_version)
    }

    /// Save help text for a subcommand to cache
    /// Use empty string for main help
    pub fn save_subcommand_help(
        &self,
        subcommand: &str,
        help_text: &str,
        openclaw_version: &str,
        chitin_version: &str,
    ) -> Result<()> {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::ZERO)
            .as_secs();

        // Load existing cache or create new one
        let mut cache = self
            .load_cache(openclaw_version, chitin_version)?
            .unwrap_or_else(|| CacheFile {
                openclaw_version: openclaw_version.to_string(),
                chitin_version: chitin_version.to_string(),
                timestamp,
                commands: HashMap::new(),
            });

        // Update timestamp and add/update the command
        cache.timestamp = timestamp;
        cache.openclaw_version = openclaw_version.to_string();
        cache.chitin_version = chitin_version.to_string();
        cache
            .commands
            .insert(subcommand.to_string(), help_text.to_string());

        self.save_cache(&cache)
    }

    /// Clear the cache
    #[allow(dead_code)]
    pub fn clear(&self) -> Result<()> {
        if self.cache_path.exists() {
            // Ignore errors if file was already deleted (race condition in tests)
            let _ = fs::remove_file(&self.cache_path);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_roundtrip() {
        let cache = HelpCache::new().unwrap();
        cache.clear().unwrap();

        // No cache initially
        assert!(cache.get_cached_help("1.0.0", "0.1.0").unwrap().is_none());

        // Save and retrieve main help
        cache.save_help("Test help text", "1.0.0", "0.1.0").unwrap();
        let cached = cache.get_cached_help("1.0.0", "0.1.0").unwrap();
        assert_eq!(cached, Some("Test help text".to_string()));

        // Wrong openclaw version returns None
        assert!(cache.get_cached_help("2.0.0", "0.1.0").unwrap().is_none());

        // Wrong chitin version returns None
        assert!(cache.get_cached_help("1.0.0", "0.2.0").unwrap().is_none());

        cache.clear().unwrap();
    }

    #[test]
    fn test_subcommand_cache() {
        let cache = HelpCache::new().unwrap();
        cache.clear().unwrap();

        // Save main and subcommand help
        cache.save_help("Main help", "1.0.0", "0.1.0").unwrap();
        cache
            .save_subcommand_help("gateway", "Gateway help", "1.0.0", "0.1.0")
            .unwrap();
        cache
            .save_subcommand_help("agent", "Agent help", "1.0.0", "0.1.0")
            .unwrap();

        // Retrieve each
        assert_eq!(
            cache.get_cached_help("1.0.0", "0.1.0").unwrap(),
            Some("Main help".to_string())
        );
        assert_eq!(
            cache
                .get_cached_subcommand_help("gateway", "1.0.0", "0.1.0")
                .unwrap(),
            Some("Gateway help".to_string())
        );
        assert_eq!(
            cache
                .get_cached_subcommand_help("agent", "1.0.0", "0.1.0")
                .unwrap(),
            Some("Agent help".to_string())
        );

        // Non-existent subcommand returns None
        assert!(
            cache
                .get_cached_subcommand_help("nonexistent", "1.0.0", "0.1.0")
                .unwrap()
                .is_none()
        );

        cache.clear().unwrap();
    }
}
