//! Runtime detection module for checking Node.js and package manager availability.

/// Runtime detector for Node.js and package managers
pub struct RuntimeDetector {
    node_available: bool,
    npm_available: bool,
    pnpm_available: bool,
}

impl RuntimeDetector {
    /// Create a new RuntimeDetector and probe for available runtimes
    pub fn new() -> Self {
        Self {
            node_available: which::which("node").is_ok(),
            npm_available: which::which("npm").is_ok(),
            pnpm_available: which::which("pnpm").is_ok(),
        }
    }

    /// Check if Node.js is available
    pub fn has_node(&self) -> bool {
        self.node_available
    }

    /// Check if npm is available
    #[allow(dead_code)]
    pub fn has_npm(&self) -> bool {
        self.npm_available
    }

    /// Check if pnpm is available
    #[allow(dead_code)]
    pub fn has_pnpm(&self) -> bool {
        self.pnpm_available
    }

    /// Check if any package manager is available
    #[allow(dead_code)]
    pub fn has_package_manager(&self) -> bool {
        self.npm_available || self.pnpm_available
    }

    /// Get the preferred package manager (pnpm preferred)
    #[allow(dead_code)]
    pub fn preferred_package_manager(&self) -> Option<&'static str> {
        if self.pnpm_available {
            Some("pnpm")
        } else if self.npm_available {
            Some("npm")
        } else {
            None
        }
    }
}

impl Default for RuntimeDetector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_runtime_detector() {
        let detector = RuntimeDetector::new();
        // Just check it doesn't panic
        let _ = detector.has_node();
        let _ = detector.has_npm();
        let _ = detector.has_pnpm();
        let _ = detector.has_package_manager();
        let _ = detector.preferred_package_manager();
    }
}
