use thiserror::Error;

#[derive(Debug, Error)]
pub enum GgitError {
    #[error("could not run `git`. Make sure Git is installed and available on PATH")]
    MissingGit,
}
