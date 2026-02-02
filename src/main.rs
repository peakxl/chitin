use anyhow::{Context, Result};
use clap::Parser;
use std::path::PathBuf;
use std::process::Command;

mod cache;
mod installer;
mod runtime;

use cache::HelpCache;
use runtime::RuntimeDetector;

const OPENCLAW_VERSION: &str = "2026.2.1";
const CHITIN_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Rebrand help text for chitin CLI
/// - Replace version line with chitin version (remove random message)
/// - Replace "openclaw" with "chitin" in Usage and Examples sections only
fn rebrand_help(text: &str) -> String {
    let mut result = String::new();
    let mut in_examples = false;

    for line in text.lines() {
        let rebranded_line = if line.starts_with("🦞 OpenClaw") || line.starts_with("OpenClaw") {
            // Replace version line
            format!("chitin {} (openclaw {})", CHITIN_VERSION, OPENCLAW_VERSION)
        } else if line.starts_with("Usage:") {
            // Replace in usage line
            line.replace("openclaw", "chitin")
        } else if line.starts_with("Examples:") {
            in_examples = true;
            line.to_string()
        } else if line.starts_with("Docs:") {
            in_examples = false;
            line.to_string()
        } else if in_examples {
            // Replace openclaw with chitin in examples
            line.replace("openclaw", "chitin")
        } else {
            line.to_string()
        };

        result.push_str(&rebranded_line);
        result.push('\n');
    }

    // Remove trailing newline if original didn't have one
    if !text.ends_with('\n') && result.ends_with('\n') {
        result.pop();
    }

    result
}

#[derive(Parser, Debug)]
#[command(
    name = "chitin",
    about = "Chitin - Fast CLI for OpenClaw",
    disable_help_flag = true,
    disable_version_flag = true
)]
struct Cli {
    /// Print help information
    #[arg(short, long)]
    help: bool,

    /// Print version information
    #[arg(short = 'V', long)]
    version: bool,

    /// Remaining arguments to pass to openclaw
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    args: Vec<String>,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    if cli.version {
        print_version();
        return Ok(());
    }

    if cli.help || cli.args.is_empty() {
        return print_help();
    }

    // Pass through to Node.js openclaw for all other commands
    delegate_to_node(&cli.args)
}

fn print_version() {
    println!("openclaw {}", OPENCLAW_VERSION);
    println!("chitin {}", CHITIN_VERSION);
}

fn print_help() -> Result<()> {
    let cache = HelpCache::new()?;

    // Try to use cached help first
    if let Some(help_text) = cache.get_cached_help(OPENCLAW_VERSION, CHITIN_VERSION)? {
        print!("{}", help_text);
        return Ok(());
    }

    // Need to generate help from Node.js
    let detector = RuntimeDetector::new();

    if !detector.has_node() {
        return prompt_install_runtime();
    }

    // Run the Node.js openclaw to get help
    let help_text = run_node_help()?;

    // Rebrand and cache for next time
    let rebranded = rebrand_help(&help_text);
    cache.save_help(&rebranded, OPENCLAW_VERSION, CHITIN_VERSION)?;

    print!("{}", rebranded);
    Ok(())
}

fn run_node_help() -> Result<String> {
    // Try to run the openclaw shim directly first (handles pnpm/npm shims)
    if let Ok(shim_path) = which::which("openclaw") {
        let output = Command::new(&shim_path)
            .arg("--help")
            .output()
            .context("Failed to run openclaw --help")?;

        if output.status.success() {
            return Ok(String::from_utf8_lossy(&output.stdout).to_string());
        }
    }

    // Fallback: find the .mjs file and run with node
    let openclaw_mjs = find_openclaw_mjs()?;

    let output = Command::new("node")
        .arg(&openclaw_mjs)
        .arg("--help")
        .output()
        .context("Failed to run openclaw --help")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("openclaw --help failed: {}", stderr);
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Find the openclaw.mjs entry point file
fn find_openclaw_mjs() -> Result<PathBuf> {
    let home = dirs::home_dir().context("Cannot find home directory")?;

    // pnpm global - search for the package in .pnpm store
    let pnpm_global_dir = home.join(".local/share/pnpm/global/5/.pnpm");
    if let Ok(entries) = std::fs::read_dir(&pnpm_global_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            if name.to_string_lossy().starts_with("openclaw@") {
                let mjs_path = entry
                    .path()
                    .join("node_modules")
                    .join("openclaw")
                    .join("openclaw.mjs");
                if mjs_path.exists() {
                    return Ok(mjs_path);
                }
            }
        }
    }

    // pnpm global (older layout)
    let pnpm_global = home.join(".local/share/pnpm/global/5/node_modules/openclaw/openclaw.mjs");
    if pnpm_global.exists() {
        return Ok(pnpm_global);
    }

    // npm global (Linux system)
    let npm_global = PathBuf::from("/usr/lib/node_modules/openclaw/openclaw.mjs");
    if npm_global.exists() {
        return Ok(npm_global);
    }

    // npm global (user install)
    let npm_user = home.join(".npm-global/lib/node_modules/openclaw/openclaw.mjs");
    if npm_user.exists() {
        return Ok(npm_user);
    }

    // npm prefix-based global
    let npm_prefix = home.join("node_modules/openclaw/openclaw.mjs");
    if npm_prefix.exists() {
        return Ok(npm_prefix);
    }

    anyhow::bail!("Cannot find openclaw installation. Run 'openclaw' without arguments to install.")
}

fn delegate_to_node(args: &[String]) -> Result<()> {
    let detector = RuntimeDetector::new();

    if !detector.has_node() {
        return prompt_install_runtime();
    }

    // Check if this is a help request for a subcommand
    let is_help_request = args.iter().any(|a| a == "--help" || a == "-h");

    if is_help_request {
        // Capture output and rebrand it
        return run_subcommand_help(args);
    }

    // Try to run the openclaw shim directly first (handles pnpm/npm shims)
    if let Ok(shim_path) = which::which("openclaw") {
        let status = Command::new(&shim_path)
            .args(args)
            .status()
            .context("Failed to run openclaw")?;

        std::process::exit(status.code().unwrap_or(1));
    }

    // Fallback: find the .mjs file and run with node
    let openclaw_mjs = find_openclaw_mjs()?;

    let status = Command::new("node")
        .arg(&openclaw_mjs)
        .args(args)
        .status()
        .context("Failed to run openclaw")?;

    std::process::exit(status.code().unwrap_or(1));
}

fn run_subcommand_help(args: &[String]) -> Result<()> {
    // Extract subcommand name (first arg that doesn't start with -)
    let subcommand = args
        .iter()
        .find(|a| !a.starts_with('-'))
        .map(|s| s.as_str())
        .unwrap_or("");

    let cache = HelpCache::new()?;

    // Try cache first
    if let Some(help_text) =
        cache.get_cached_subcommand_help(subcommand, OPENCLAW_VERSION, CHITIN_VERSION)?
    {
        print!("{}", help_text);
        return Ok(());
    }

    // Fetch from Node.js
    let output = if let Ok(shim_path) = which::which("openclaw") {
        Command::new(&shim_path)
            .args(args)
            .output()
            .context("Failed to run openclaw")?
    } else {
        let openclaw_mjs = find_openclaw_mjs()?;
        Command::new("node")
            .arg(&openclaw_mjs)
            .args(args)
            .output()
            .context("Failed to run openclaw")?
    };

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Rebrand and cache
    let rebranded = rebrand_help(&stdout);
    if output.status.success() && !rebranded.is_empty() {
        let _ =
            cache.save_subcommand_help(subcommand, &rebranded, OPENCLAW_VERSION, CHITIN_VERSION);
    }

    print!("{}", rebranded);
    eprint!("{}", rebrand_help(&stderr));

    std::process::exit(output.status.code().unwrap_or(1));
}

fn prompt_install_runtime() -> Result<()> {
    installer::run_interactive_install()
}
