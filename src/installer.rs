//! Interactive installer module for setting up Node.js runtime and openclaw.

use anyhow::{Context, Result};
use std::io::{self, BufRead, Write};
use std::process::Command;

/// Package manager choice
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PackageManager {
    Pnpm,
    Npm,
}

impl PackageManager {
    pub fn name(&self) -> &'static str {
        match self {
            PackageManager::Pnpm => "pnpm",
            PackageManager::Npm => "npm",
        }
    }

    pub fn install_openclaw_cmd(&self) -> (&'static str, &'static [&'static str]) {
        match self {
            PackageManager::Pnpm => ("pnpm", &["add", "-g", "openclaw@latest"]),
            PackageManager::Npm => ("npm", &["install", "-g", "openclaw@latest"]),
        }
    }
}

/// Prompt user to select a package manager
pub fn prompt_package_manager_selection() -> Result<PackageManager> {
    println!();
    println!("Select a package manager to install openclaw:");
    println!();
    println!("  [1] pnpm (Recommended)");
    println!("  [2] npm");
    println!();
    print!("Enter choice [1-2]: ");
    io::stdout().flush()?;

    let stdin = io::stdin();
    let mut input = String::new();
    stdin.lock().read_line(&mut input)?;

    let choice = input.trim();
    match choice {
        "1" | "" => Ok(PackageManager::Pnpm),
        "2" => Ok(PackageManager::Npm),
        _ => {
            println!("Invalid choice, defaulting to pnpm");
            Ok(PackageManager::Pnpm)
        }
    }
}

/// Prompt user for yes/no confirmation
pub fn prompt_confirm(message: &str, default_yes: bool) -> Result<bool> {
    let hint = if default_yes { "[Y/n]" } else { "[y/N]" };
    print!("{} {}: ", message, hint);
    io::stdout().flush()?;

    let stdin = io::stdin();
    let mut input = String::new();
    stdin.lock().read_line(&mut input)?;

    let choice = input.trim().to_lowercase();
    match choice.as_str() {
        "" => Ok(default_yes),
        "y" | "yes" => Ok(true),
        "n" | "no" => Ok(false),
        _ => Ok(default_yes),
    }
}

/// Check if running in an interactive terminal
pub fn is_interactive() -> bool {
    atty::is(atty::Stream::Stdin) && atty::is(atty::Stream::Stdout)
}

/// Install pnpm using the official installer
pub fn install_pnpm() -> Result<()> {
    println!("Installing pnpm...");

    let status = Command::new("sh")
        .arg("-c")
        .arg("curl -fsSL https://get.pnpm.io/install.sh | sh -")
        .status()
        .context("Failed to run pnpm installer")?;

    if !status.success() {
        anyhow::bail!("pnpm installation failed");
    }

    // Source the environment to get pnpm in PATH
    println!("pnpm installed successfully.");
    println!();
    println!("Installing Node.js 22 via pnpm...");

    // Try to find pnpm in common locations
    let pnpm_path = find_pnpm_path()?;

    let status = Command::new(&pnpm_path)
        .args(["env", "use", "--global", "22"])
        .status()
        .context("Failed to install Node.js via pnpm")?;

    if !status.success() {
        anyhow::bail!("Node.js installation via pnpm failed");
    }

    println!("Node.js 22 installed successfully.");
    Ok(())
}

/// Find pnpm executable path after installation
fn find_pnpm_path() -> Result<String> {
    // Check if pnpm is in PATH
    if which::which("pnpm").is_ok() {
        return Ok("pnpm".to_string());
    }

    // Check common installation locations
    if let Some(home) = dirs::home_dir() {
        let pnpm_home = home.join(".local/share/pnpm/pnpm");
        if pnpm_home.exists() {
            return Ok(pnpm_home.to_string_lossy().to_string());
        }

        // Also check the bin directory
        let pnpm_bin = home.join(".local/share/pnpm/pnpm");
        if pnpm_bin.exists() {
            return Ok(pnpm_bin.to_string_lossy().to_string());
        }
    }

    // Try sourcing the shell config and running pnpm
    Ok("pnpm".to_string())
}

/// Install openclaw using the selected package manager
pub fn install_openclaw(pm: PackageManager) -> Result<()> {
    println!();
    println!("Installing openclaw via {}...", pm.name());

    let (cmd, args) = pm.install_openclaw_cmd();

    // For pnpm, we may need to use the full path
    let cmd_path = if pm == PackageManager::Pnpm {
        find_pnpm_path().unwrap_or_else(|_| cmd.to_string())
    } else {
        cmd.to_string()
    };

    let status = Command::new(&cmd_path)
        .args(args)
        .status()
        .context(format!("Failed to run {} install", pm.name()))?;

    if !status.success() {
        anyhow::bail!("openclaw installation failed");
    }

    println!("openclaw installed successfully.");
    Ok(())
}

/// Pre-cache the help output after installation
pub fn precache_help() -> Result<()> {
    println!();
    println!("Pre-caching help output...");

    let shim_path = match which::which("openclaw") {
        Ok(p) => p,
        Err(_) => {
            println!("Note: Could not find openclaw. Help will be cached on first use.");
            return Ok(());
        }
    };

    let cache = crate::cache::HelpCache::new()?;

    // Cache main help only (subcommands cached on first use)
    let output = Command::new(&shim_path)
        .arg("--help")
        .output()
        .context("Failed to run openclaw --help for caching")?;

    if !output.status.success() {
        println!("Note: Could not pre-cache help. It will be cached on first use.");
        return Ok(());
    }

    let help_text = String::from_utf8_lossy(&output.stdout).to_string();
    let rebranded = crate::rebrand_help(&help_text);
    cache.save_help(&rebranded, crate::OPENCLAW_VERSION, crate::CHITIN_VERSION)?;

    println!("Help cached successfully.");
    Ok(())
}

/// Run the full interactive installation flow
pub fn run_interactive_install() -> Result<()> {
    println!();
    println!("OpenClaw requires Node.js >= 22 and a package manager.");
    println!();

    if !is_interactive() {
        // Non-interactive mode: print instructions and exit
        eprintln!("Running in non-interactive mode. Please install manually:");
        eprintln!();
        eprintln!("Option 1 (Recommended): Install pnpm + Node.js");
        eprintln!("  curl -fsSL https://get.pnpm.io/install.sh | sh -");
        eprintln!("  pnpm env use --global 22");
        eprintln!("  pnpm add -g openclaw@latest");
        eprintln!();
        eprintln!("Option 2: Install Node.js via system package manager");
        eprintln!("  # Debian/Ubuntu:");
        eprintln!("  curl -fsSL https://deb.nodesource.com/setup_22.x | sudo -E bash -");
        eprintln!("  sudo apt-get install -y nodejs");
        eprintln!("  npm install -g openclaw@latest");
        eprintln!();
        eprintln!("Then run 'openclaw onboard' to get started.");
        std::process::exit(1);
    }

    // Check what's already installed
    let has_node = which::which("node").is_ok();
    let has_pnpm = which::which("pnpm").is_ok();
    let has_npm = which::which("npm").is_ok();

    if has_node && (has_pnpm || has_npm) {
        // Node and a package manager exist, just need to install openclaw
        let pm = if has_pnpm {
            println!("Found Node.js and pnpm installed.");
            PackageManager::Pnpm
        } else {
            println!("Found Node.js and npm installed.");
            PackageManager::Npm
        };

        if prompt_confirm("Install openclaw now?", true)? {
            install_openclaw(pm)?;
            precache_help()?;
            println!();
            println!("Installation complete! Run 'openclaw onboard' to get started.");
            return Ok(());
        } else {
            println!("Installation cancelled.");
            std::process::exit(0);
        }
    }

    if has_node {
        // Has Node but no package manager - unusual but handle it
        println!("Found Node.js but no package manager (pnpm/npm).");
        let pm = prompt_package_manager_selection()?;

        if pm == PackageManager::Pnpm && prompt_confirm("Install pnpm now?", true)? {
            // Install pnpm without Node.js setup
            let status = Command::new("sh")
                .arg("-c")
                .arg("curl -fsSL https://get.pnpm.io/install.sh | sh -")
                .status()
                .context("Failed to install pnpm")?;

            if !status.success() {
                anyhow::bail!("pnpm installation failed");
            }
        }

        install_openclaw(pm)?;
        precache_help()?;
        println!();
        println!("Installation complete! Run 'openclaw onboard' to get started.");
        return Ok(());
    }

    // No Node.js - need full installation
    println!("Node.js is not installed.");
    let pm = prompt_package_manager_selection()?;

    if pm == PackageManager::Pnpm {
        if prompt_confirm("Install pnpm and Node.js 22 now?", true)? {
            install_pnpm()?;
            install_openclaw(pm)?;
            precache_help()?;
            println!();
            println!("Installation complete! Run 'openclaw onboard' to get started.");
        } else {
            println!("Installation cancelled.");
            std::process::exit(0);
        }
    } else {
        // npm selected - need to install Node.js first
        println!();
        println!("To use npm, you need to install Node.js first.");
        println!();
        println!("Install Node.js using your system package manager:");
        println!();
        println!("  # Debian/Ubuntu:");
        println!("  curl -fsSL https://deb.nodesource.com/setup_22.x | sudo -E bash -");
        println!("  sudo apt-get install -y nodejs");
        println!();
        println!("  # macOS (Homebrew):");
        println!("  brew install node@22");
        println!();
        println!("  # Or download from: https://nodejs.org/");
        println!();
        println!("After installing Node.js, run this command again.");
        std::process::exit(1);
    }

    Ok(())
}
