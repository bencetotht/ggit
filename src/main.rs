mod cli;
mod config;
mod db;
mod error;
mod git;
mod output;
mod scan;
mod status;

use anyhow::{anyhow, Context, Result};
use clap::Parser;
use cli::{Cli, Commands};
use colored::Colorize;
use db::{Database, Repository};
use output::{repo_table, status_table};
use std::path::{Path, PathBuf};

fn main() {
    if let Err(err) = run() {
        eprintln!("{} {}", "error:".red().bold(), err);
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();
    if cli.verbose && cli.quiet {
        return Err(anyhow!("--verbose and --quiet cannot be used together"));
    }

    let db_path = config::db_path()?;
    let db = Database::open(&db_path)?;

    match cli.command {
        Commands::Scan {
            path,
            maxdepth,
            include_hidden,
            force,
            dry_run,
        } => scan_command(
            &db,
            &db_path,
            &path,
            maxdepth,
            include_hidden,
            force,
            dry_run,
            cli.verbose,
            cli.quiet,
        ),
        Commands::Pull {
            filter,
            path,
            dry_run,
            continue_on_error,
        } => pull_command(
            &db,
            filter.as_deref(),
            path.as_deref(),
            dry_run,
            continue_on_error,
            cli.verbose,
            cli.quiet,
        ),
        Commands::Status {
            filter,
            path,
            short,
            refresh_remotes,
        } => status_command(
            &db,
            filter.as_deref(),
            path.as_deref(),
            short,
            refresh_remotes,
            cli.quiet,
        ),
        Commands::List { filter } => list_command(&db, filter.as_deref()),
        Commands::Remove { filter, path, yes } => {
            remove_command(&db, filter.as_deref(), path.as_deref(), yes)
        }
        Commands::Clear { yes } => clear_command(&db, yes),
        Commands::Doctor => doctor_command(&db, &db_path),
    }
}

#[allow(clippy::too_many_arguments)]
fn scan_command(
    db: &Database,
    db_path: &Path,
    path: &Path,
    maxdepth: Option<usize>,
    include_hidden: bool,
    force: bool,
    dry_run: bool,
    verbose: bool,
    quiet: bool,
) -> Result<()> {
    let root = path
        .canonicalize()
        .with_context(|| format!("could not resolve scan path {}", path.display()))?;

    if !quiet {
        println!("{} {}", "Scanning".cyan().bold(), root.display());
    }

    let options = scan::ScanOptions {
        max_depth: maxdepth,
        include_hidden,
    };
    let result = scan::scan_repositories(&root, &options)?;

    if !quiet {
        println!();
        println!("{} {}", "Found".green().bold(), result.repositories.len());
        println!();
        println!("{}", repo_table(&result.repositories));
    }

    if !dry_run {
        for repo in &result.repositories {
            if force || db.find_by_path(&repo.path)?.is_none() {
                db.upsert_repository(repo)?;
            }
        }
    }

    if !quiet {
        if dry_run {
            println!();
            println!("{}", "Dry run: database was not changed".yellow());
        } else {
            println!();
            println!(
                "Stored {} repositories in {}",
                result.repositories.len().to_string().green(),
                db_path.display()
            );
        }
        if verbose {
            println!("Skipped {} pruned directories", result.pruned_directories);
        }
    }

    Ok(())
}

fn select_repositories(
    db: &Database,
    filter: Option<&str>,
    path: Option<&Path>,
) -> Result<Vec<Repository>> {
    let repos = if let Some(path) = path {
        let canonical = canonical_for_lookup(path);
        db.find_by_path(&canonical)?
            .map(|repo| vec![repo])
            .unwrap_or_default()
    } else if let Some(filter) = filter {
        db.filter_by_name(filter)?
    } else {
        db.list_repositories()?
    };

    if repos.is_empty() {
        if let Some(filter) = filter {
            Err(anyhow!(
                "No tracked repositories matched filter `{filter}`."
            ))
        } else if let Some(path) = path {
            Err(anyhow!(
                "No tracked repository matched path `{}`.",
                path.display()
            ))
        } else {
            Err(anyhow!(
                "No repositories are tracked yet. Run `ggit scan .` from a directory that contains Git repositories."
            ))
        }
    } else {
        Ok(repos)
    }
}

fn pull_command(
    db: &Database,
    filter: Option<&str>,
    path: Option<&Path>,
    dry_run: bool,
    continue_on_error: bool,
    verbose: bool,
    quiet: bool,
) -> Result<()> {
    let repos = select_repositories(db, filter, path)?;
    if dry_run {
        println!("Would pull {} repositories", repos.len());
        println!("{}", output::stored_repo_table(&repos));
        return Ok(());
    }

    if !quiet {
        println!("Pulling {} repositories", repos.len());
        println!();
    }

    let mut summary = PullSummary::default();
    for (index, repo) in repos.iter().enumerate() {
        if !quiet {
            println!(
                "[{}/{}] {}",
                index + 1,
                repos.len(),
                repo.name.cyan().bold()
            );
            println!("  path: {}", repo.path.display());
            println!(
                "  remote: {}",
                repo.remote_url.as_deref().unwrap_or("no remote configured")
            );
        }

        match pull_one(repo, verbose) {
            PullOutcome::Updated => {
                summary.updated += 1;
                if !quiet {
                    println!("  result: {}", "updated".green());
                }
            }
            PullOutcome::UpToDate => {
                summary.up_to_date += 1;
                if !quiet {
                    println!("  result: {}", "already up to date".green());
                }
            }
            PullOutcome::Skipped(reason) => {
                summary.skipped += 1;
                if !quiet {
                    println!("  result: {} {}", "skipped,".yellow(), reason);
                }
            }
            PullOutcome::Failed(reason) => {
                summary.failed += 1;
                if !quiet {
                    println!("  result: {} {}", "failed,".red(), reason);
                }
                if !continue_on_error {
                    print_pull_summary(&summary);
                    return Err(anyhow!(
                        "Stopping after failure in {}. Rerun with --continue-on-error to continue past failures.",
                        repo.name
                    ));
                }
            }
        }

        if !quiet {
            println!();
        }
    }

    if !quiet {
        print_pull_summary(&summary);
    }
    Ok(())
}

fn pull_one(repo: &Repository, verbose: bool) -> PullOutcome {
    if !repo.path.exists() {
        return PullOutcome::Failed("missing path".to_string());
    }
    if !git::is_git_repo(&repo.path) {
        return PullOutcome::Failed("not a git repository".to_string());
    }

    let status = match git::status_snapshot(&repo.path) {
        Ok(status) => status,
        Err(err) => return PullOutcome::Failed(err.to_string()),
    };
    if status.worktree_dirty {
        return PullOutcome::Skipped(format!(
            "dirty working tree. Commit, stash, or discard changes, then rerun `ggit pull --filter {}`.",
            repo.name
        ));
    }

    match git::pull_ff_only(&repo.path) {
        Ok(result) => {
            if verbose && !result.output.trim().is_empty() {
                println!("  git: {}", result.output.trim());
            }
            if result.updated {
                PullOutcome::Updated
            } else {
                PullOutcome::UpToDate
            }
        }
        Err(err) => {
            let _ = git::abort_merge_or_rebase_if_needed(&repo.path);
            PullOutcome::Failed(format!(
                "{} Manual step: inspect `{}` and resolve the Git state before pulling again.",
                err,
                repo.path.display()
            ))
        }
    }
}

fn status_command(
    db: &Database,
    filter: Option<&str>,
    path: Option<&Path>,
    short: bool,
    refresh_remotes: bool,
    quiet: bool,
) -> Result<()> {
    if refresh_remotes && !quiet {
        println!(
            "{}",
            "--refresh-remotes is accepted for compatibility, but status remains local-only in this version."
                .yellow()
        );
    }

    let repos = select_repositories(db, filter, path)?;
    let rows: Vec<_> = repos
        .iter()
        .map(|repo| status::status_row(repo, short))
        .collect();
    println!("{}", status_table(&rows, short));
    Ok(())
}

fn list_command(db: &Database, filter: Option<&str>) -> Result<()> {
    let repos = if let Some(filter) = filter {
        db.filter_by_name(filter)?
    } else {
        db.list_repositories()?
    };
    if repos.is_empty() {
        if let Some(filter) = filter {
            return Err(anyhow!(
                "No tracked repositories matched filter `{filter}`."
            ));
        }
        return Err(anyhow!(
            "No repositories are tracked yet. Run `ggit scan .` from a directory that contains Git repositories."
        ));
    }
    println!("{}", output::stored_repo_table(&repos));
    Ok(())
}

fn remove_command(
    db: &Database,
    filter: Option<&str>,
    path: Option<&Path>,
    yes: bool,
) -> Result<()> {
    if filter.is_none() && path.is_none() {
        return Err(anyhow!(
            "Provide --filter <name> or --path <path> to remove repositories."
        ));
    }

    let repos = select_repositories(db, filter, path)?;
    if repos.len() > 1 && !yes {
        println!("{}", output::stored_repo_table(&repos));
        return Err(anyhow!(
            "{} repositories matched. Rerun with --yes to remove them all.",
            repos.len()
        ));
    }

    for repo in &repos {
        db.remove_by_path(&repo.path)?;
        println!(
            "Removed {} ({})",
            repo.name.cyan().bold(),
            repo.path.display()
        );
    }
    Ok(())
}

fn clear_command(db: &Database, yes: bool) -> Result<()> {
    let repos = db.list_repositories()?;
    if repos.is_empty() {
        println!("No repositories are tracked.");
        return Ok(());
    }
    if !yes {
        println!("{}", output::stored_repo_table(&repos));
        return Err(anyhow!(
            "This would remove {} repositories. Rerun with `ggit clear --yes` to confirm.",
            repos.len()
        ));
    }
    db.clear()?;
    println!("Cleared {} repositories.", repos.len());
    Ok(())
}

fn doctor_command(db: &Database, db_path: &Path) -> Result<()> {
    println!("ggit doctor");
    match git::git_version() {
        Ok(version) => println!("{} git: {}", "ok".green(), version.trim()),
        Err(err) => println!("{} git: {}", "fail".red(), err),
    }

    println!("{} database: {}", "ok".green(), db_path.display());
    let repos = db.list_repositories()?;
    println!("tracked repositories: {}", repos.len());

    let mut missing = 0usize;
    let mut changed_remotes = 0usize;
    for repo in repos {
        if !repo.path.exists() {
            missing += 1;
            println!("{} missing path: {}", "warn".yellow(), repo.path.display());
            continue;
        }
        let current_remote = git::repo_remote_url(&repo.path).ok().flatten();
        if current_remote != repo.remote_url {
            changed_remotes += 1;
            println!(
                "{} remote changed for {}: stored `{}`, current `{}`",
                "warn".yellow(),
                repo.name,
                repo.remote_url.as_deref().unwrap_or("-"),
                current_remote.as_deref().unwrap_or("-")
            );
        }
    }

    println!("missing paths: {missing}");
    println!("changed remotes: {changed_remotes}");
    Ok(())
}

fn canonical_for_lookup(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

#[derive(Default)]
struct PullSummary {
    updated: usize,
    up_to_date: usize,
    skipped: usize,
    failed: usize,
}

fn print_pull_summary(summary: &PullSummary) {
    println!("Summary");
    println!("updated: {}", summary.updated);
    println!("up to date: {}", summary.up_to_date);
    println!("skipped: {}", summary.skipped);
    println!("failed: {}", summary.failed);
}

enum PullOutcome {
    Updated,
    UpToDate,
    Skipped(String),
    Failed(String),
}
