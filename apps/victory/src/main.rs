//! `victory`: the command-line front end.
//!
//! Every command reads the user's own files and writes only to stdout or to
//! the user's cache directory (outside the repository).

mod census;
mod inspect;
mod run;
mod shader;
mod translate;

use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand, ValueEnum};

#[derive(Parser)]
#[command(
    name = "victory",
    version,
    about = "Run an Xbox Series title natively on Windows"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Summarise one PE image: headers, sections, imports, TLS, build id.
    Inspect {
        file: PathBuf,
        /// List every imported function.
        #[arg(long)]
        imports: bool,
        /// List every exported function.
        #[arg(long)]
        exports: bool,
    },
    /// Aggregate the imports of every PE image under the given paths and
    /// show which ones our runtime covers. Defaults to the configured
    /// game and test-title directories.
    Census {
        paths: Vec<PathBuf>,
        /// Do not count host pass-through as covered.
        #[arg(long)]
        strict: bool,
        /// List every missing `dll!function`, most used first.
        #[arg(long)]
        missing: bool,
    },
    /// List the `dll!export` pairs an import library describes.
    LibExports {
        lib: PathBuf,
        /// List every export, not just per-DLL counts.
        #[arg(long)]
        list: bool,
    },
    /// Find shader containers in files or directories (also inside engine
    /// archives, if uncompressed) and report their layout.
    ShaderInfo {
        paths: Vec<PathBuf>,
        /// Print one line per container found.
        #[arg(long)]
        verbose: bool,
        /// Also list the decoded native instructions (implies --verbose).
        #[arg(long)]
        disasm: bool,
    },
    /// Run every shader found (containers, SGD5 files) through the
    /// translator and report what it cannot handle yet.
    Translate {
        paths: Vec<PathBuf>,
        /// Check emitted DXIL with dxil.dll's validator (`VICTORY_DXIL`,
        /// else the newest Windows SDK's).
        #[arg(long)]
        validate: bool,
        /// Also create a host D3D12 pipeline for each emitted shader.
        #[arg(long)]
        pipelines: bool,
        /// Print one line per program and outcome.
        #[arg(long)]
        list: bool,
        /// Worker threads (default: every logical core).
        #[arg(long)]
        jobs: Option<usize>,
    },
    /// Prepare, map and bind a title executable; with --execute, run it.
    Run {
        exe: PathBuf,
        /// Runtime backend to use when executing (`victory` or `xcl`).
        #[arg(long, value_enum, default_value_t = Backend::Victory)]
        backend: Backend,
        /// What unimplemented imports do when called.
        #[arg(long, value_enum, default_value_t = Unimplemented::Trap)]
        unimplemented: Unimplemented,
        /// Disable host pass-through (everything must be implemented by us).
        #[arg(long)]
        strict: bool,
        /// Actually start the title. Without it, stop after binding.
        #[arg(long)]
        execute: bool,
        /// The console model the title sees (device type, memory).
        #[arg(long, value_enum, default_value_t = Console::SeriesX)]
        console: Console,
        /// Arguments for the title, after `--` (it reads the whole command
        /// line; a title that relaunches itself passes some).
        #[arg(last = true)]
        title_args: Vec<String>,
    },
    /// Delegate directly to the integrated XCL runtime tools.
    Xcl {
        #[command(subcommand)]
        action: XclAction,
    },
    /// Show the resolved configuration and check it.
    Config,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
enum Backend {
    Victory,
    Xcl,
}

#[derive(Subcommand)]
enum XclAction {
    /// Inspect imports of a PE image using XCL's resolver.
    Inspect {
        file: PathBuf,
        #[arg(long)]
        json: bool,
    },
    /// Disassemble console GPU code (RDNA2) via XCL.
    Disasm {
        file: PathBuf,
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Analyze shader corpora or RPF9 archives via XCL.
    Shaders {
        path: PathBuf,
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Inspect or extract RPF9 archives via XCL.
    Rpf {
        archive: PathBuf,
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Pre-compile shaders into the shared DXIL disk cache to eliminate runtime stutter.
    Warmup {
        path: PathBuf,
        /// Maximum number of shaders to compile (`0` = all complete translations).
        #[arg(long, default_value_t = 0)]
        limit: usize,
    },
    /// Show XCL DXIL shader cache directory and process hit/miss statistics.
    CacheStats,
}

#[derive(Clone, Copy, ValueEnum)]
enum Unimplemented {
    Trap,
    Zero,
}

#[derive(Clone, Copy, ValueEnum)]
enum Console {
    SeriesX,
    SeriesS,
}

fn main() -> Result<()> {
    #[cfg(windows)]
    if let Some(boot) = std::env::var_os("XCL_HOST") {
        // SAFETY: Called at the very start of `main` before any worker threads are spawned.
        unsafe { std::env::remove_var("XCL_HOST") };
        let code = xcl::host::host_main(&PathBuf::from(boot));
        anyhow::ensure!(code == 0, "xcl host exited with code {code}");
        return Ok(());
    }

    victory_core::logging::init("info");
    // The audio recorder `run` starts for `VICTORY_AUDIO_CAPTURE`.
    #[cfg(all(windows, feature = "diagnostics"))]
    if victory_hle::modules::audio::capture::child() {
        return Ok(());
    }
    let cli = Cli::parse();
    match cli.command {
        Command::Inspect {
            file,
            imports,
            exports,
        } => inspect::run(&file, imports, exports),
        Command::Census {
            paths,
            strict,
            missing,
        } => census::run(paths, strict, missing),
        Command::LibExports { lib, list } => inspect::lib_exports(&lib, list),
        Command::ShaderInfo {
            paths,
            verbose,
            disasm,
        } => shader::run(paths, verbose || disasm, disasm),
        Command::Translate {
            paths,
            validate,
            pipelines,
            list,
            jobs,
        } => translate::run(paths, validate, pipelines, list, jobs),
        Command::Run {
            exe,
            backend,
            unimplemented,
            strict,
            execute,
            console,
            title_args,
        } => match backend {
            Backend::Victory => run::run(&exe, unimplemented, strict, execute, console),
            Backend::Xcl => run_xcl_backend(&exe, execute, &title_args),
        },
        Command::Xcl { action } => run_xcl_action(action),
        Command::Config => show_config(),
    }
}

/// Point XCL's DXIL disk cache at VIctory's configured `shaders` cache directory
/// unless the user already set `XCL_DXIL_CACHE`.
fn configure_shared_shader_cache() {
    if std::env::var_os("XCL_DXIL_CACHE").is_some() {
        return;
    }
    if let Ok(cfg) = victory_core::Config::load()
        && let Ok(dir) = cfg.cache_subdir("shaders")
    {
        // SAFETY: Called on the main CLI thread before spawning XCL child processes or workers.
        unsafe { std::env::set_var("XCL_DXIL_CACHE", dir) };
    }
}

fn run_xcl_backend(exe: &std::path::Path, execute: bool, title_args: &[String]) -> Result<()> {
    configure_shared_shader_cache();
    let code = if execute {
        xcl::launcher::run(exe, title_args)
    } else {
        xcl::inspect::run(exe, false)
    };
    anyhow::ensure!(code == 0, "xcl backend exited with code {code}");
    Ok(())
}

fn run_xcl_action(action: XclAction) -> Result<()> {
    configure_shared_shader_cache();
    let code = match action {
        XclAction::Inspect { file, json } => xcl::inspect::run(&file, json),
        XclAction::Disasm { file, args } => xcl::disasm::run(&file, &args),
        XclAction::Shaders { path, args } => xcl::shaders::run(&path, &args),
        XclAction::Rpf { archive, args } => xcl::rpf::run(&archive, &args),
        XclAction::Warmup { path, limit } => {
            let args = vec![
                "--recomp".to_owned(),
                "--compile".to_owned(),
                limit.to_string(),
            ];
            let code = xcl::shaders::run(&path, &args);
            let stats = xcl::gpu::recomp::dxc::cache_stats();
            println!(
                "dxil cache ({}): hits={}, misses={}, compilations={}, failures={}",
                xcl::gpu::recomp::dxc::cache_dir().display(),
                stats.hits,
                stats.misses,
                stats.compilations,
                stats.compilation_failures,
            );
            code
        }
        XclAction::CacheStats => {
            let stats = xcl::gpu::recomp::dxc::cache_stats();
            println!("dxil cache dir  : {}", xcl::gpu::recomp::dxc::cache_dir().display());
            println!("hits            : {}", stats.hits);
            println!("misses          : {}", stats.misses);
            println!("coalesced hits  : {}", stats.coalesced_hits);
            println!("deferred misses : {}", stats.deferred_misses);
            println!("compilations    : {}", stats.compilations);
            println!("failures        : {}", stats.compilation_failures);
            0
        }
    };
    anyhow::ensure!(code == 0, "xcl command exited with code {code}");
    Ok(())
}

fn show_config() -> Result<()> {
    let cfg = victory_core::Config::load()?;
    let status = |p: &Option<PathBuf>| match p {
        Some(p) if p.exists() => format!("{}", p.display()),
        Some(p) => format!("{}  (does not exist)", p.display()),
        None => "(not set)".to_owned(),
    };
    println!(
        "config file : {}",
        cfg.source
            .as_ref()
            .map_or("(none)".into(), |p| p.display().to_string())
    );
    println!("game_dir    : {}", status(&cfg.game_dir));
    println!("cache_dir   : {}", cfg.cache_dir.display());
    println!("data_dir    : {}", cfg.data_dir.display());
    for t in &cfg.test_titles {
        println!("test_title  : {}", status(&Some(t.clone())));
    }
    for p in cfg
        .game_dir
        .iter()
        .chain(&cfg.test_titles)
        .chain([&cfg.cache_dir, &cfg.data_dir])
    {
        if let Err(e) = victory_core::ensure_outside_repo(p) {
            println!("POLICY VIOLATION: {e}");
        }
    }
    Ok(())
}

/// Paths given on the command line, or the configured game + test titles.
pub(crate) fn default_paths(paths: Vec<PathBuf>) -> Result<Vec<PathBuf>> {
    if !paths.is_empty() {
        return Ok(paths);
    }
    let cfg = victory_core::Config::load()?;
    let all: Vec<PathBuf> = cfg.game_dir.into_iter().chain(cfg.test_titles).collect();
    anyhow::ensure!(
        !all.is_empty(),
        "no paths given and none configured (see victory.example.toml)"
    );
    Ok(all)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cli_parses_default_and_xcl_backends() {
        let default_cli = Cli::try_parse_from(["victory", "run", "game.exe"]).unwrap();
        match default_cli.command {
            Command::Run { backend, .. } => assert_eq!(backend, Backend::Victory),
            _ => panic!("expected Run command"),
        }

        let xcl_cli =
            Cli::try_parse_from(["victory", "run", "--backend", "xcl", "--execute", "game.exe"])
                .unwrap();
        match xcl_cli.command {
            Command::Run {
                backend, execute, ..
            } => {
                assert_eq!(backend, Backend::Xcl);
                assert!(execute);
            }
            _ => panic!("expected Run command"),
        }
    }

    #[test]
    fn cli_parses_xcl_subcommands() {
        let cli = Cli::try_parse_from(["victory", "xcl", "inspect", "title.exe", "--json"])
            .unwrap();
        assert!(matches!(
            cli.command,
            Command::Xcl {
                action: XclAction::Inspect { json: true, .. }
            }
        ));

        let warmup = Cli::try_parse_from(["victory", "xcl", "warmup", "shaders.rpf", "--limit", "64"])
            .unwrap();
        assert!(matches!(
            warmup.command,
            Command::Xcl {
                action: XclAction::Warmup { limit: 64, .. }
            }
        ));

        let stats = Cli::try_parse_from(["victory", "xcl", "cache-stats"]).unwrap();
        assert!(matches!(
            stats.command,
            Command::Xcl {
                action: XclAction::CacheStats
            }
        ));
    }
}
