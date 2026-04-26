# ggit

`ggit` is a Rust CLI for tracking Git repositories across your machine. It can scan a directory tree, store discovered repositories in a local registry, show status across all tracked repositories, and run safe fast-forward pulls.

## Install

From this repository:

```text
cargo install --path .
```

## Quick Start

```text
ggit scan ~/Developer
ggit status
ggit pull
```

The registry is stored at:

```text
~/.ggit/ggit.db
```

For tests and development, `GGIT_DB_PATH=/tmp/ggit.db` overrides the database path.

## Commands

### Scan

```text
ggit scan .
ggit scan ~/Developer --maxdepth 2
ggit scan . --include-hidden
ggit scan . --dry-run
```

`scan` finds directories that contain `.git`, prints their name, remote URL, and path, then stores them in the registry. It prunes common generated or heavy directories such as `node_modules`, `.venv`, `.cache`, `target`, `dist`, `.next`, `.gradle`, and `vendor`.

### Status

```text
ggit status
ggit status --filter api
ggit status --short
```

`status` shows branch, sync state, worktree state, remote URL, and path. It is local-only by default and does not fetch remotes.

### Pull

```text
ggit pull
ggit pull --filter ggit
ggit pull --dry-run
ggit pull --continue-on-error
```

`pull` processes repositories sequentially and uses:

```text
git pull --ff-only
```

That avoids creating merge commits during bulk updates. Dirty repositories are skipped, and failed pulls include the repository path and a manual next step.

### Registry Management

```text
ggit list
ggit remove --filter old-project
ggit remove --path /absolute/path
ggit clear --yes
ggit doctor
```

`doctor` checks Git availability, database access, missing repository paths, and changed remotes.

Run `ggit --help` or `ggit <command> --help` for the full command reference.
