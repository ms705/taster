use git2;
use slog;

#[derive(Clone, Debug)]
pub struct Commit {
    pub id: git2::Oid,
    pub msg: String,
    pub url: String,
}

#[derive(Clone, Debug)]
pub struct Push {
    pub head_commit: Commit,
    pub push_ref: Option<String>,
    pub pusher: Option<String>,
    pub owner_name: Option<String>,
    pub repo_name: Option<String>,
}

pub fn new_logger() -> slog::Logger {
    use slog::Drain;
    use slog::Logger;
    use slog_term::term_full;
    use std::sync::Mutex;
    Logger::root(Mutex::new(term_full()).fuse(), o!())
}
