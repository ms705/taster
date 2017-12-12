use std::collections::HashMap;

use distributary::{ControllerHandle, Mutator, RemoteGetter, ZookeeperAuthority};
use git2;
use slog;

use super::{BranchHistoryEntry, HistoryDB};

pub struct SoupHistoryDB {
    soup: ControllerHandle<ZookeeperAuthority>,
    log: slog::Logger,

    recipe: String,
    tables: HashMap<String, Mutator>,
    views: HashMap<String, RemoteGetter>,
}

impl SoupHistoryDB {
    pub fn new(zk_addr: &str, log: Option<slog::Logger>) -> Self {
        let log = match log {
            None => slog::Logger::root(slog::Discard, o!()),
            Some(l) => l,
        };

        let recipe = "CREATE TABLE benchmarks (id int, name varchar(100), metric varchar(100));";

        debug!(log, "Finding Soup via Zookeeper...");

        let zk_auth = ZookeeperAuthority::new(&format!("{}", zk_addr));

        debug!(log, "Connecting to Soup...");
        let mut ch = ControllerHandle::new(zk_auth);

        debug!(log, "Installing recipe in Soup...");
        ch.install_recipe(recipe.to_owned());

        SoupHistoryDB {
            soup: ch,
            log: log,

            recipe: recipe.to_owned(),
            tables: HashMap::default(),
            views: HashMap::default(),
        }
    }
}

impl HistoryDB for SoupHistoryDB {
    fn put(
        &mut self,
        branch: &str,
        commit: git2::Oid,
        entry: BranchHistoryEntry,
    ) -> Result<(), String> {
        unimplemented!();
    }

    fn get_commit(
        &self,
        branch: &str,
        commit: Option<&git2::Oid>,
    ) -> Result<BranchHistoryEntry, String> {
        unimplemented!();
    }

    fn get_branch_history(&self, branch: &str) -> Result<Vec<BranchHistoryEntry>, String> {
        unimplemented!();
    }
}
