use std::collections::HashMap;

use distributary::{ControllerHandle, Mutator, RemoteGetter, ZookeeperAuthority};
use git2;

use super::{BranchHistoryEntry, HistoryDB};

pub struct SoupHistoryDB {
    soup: ControllerHandle<ZookeeperAuthority>,

    recipe: String,
    tables: HashMap<String, Mutator>,
    views: HashMap<String, RemoteGetter>,
}

impl SoupHistoryDB {
    pub fn new(zk_addr: &str) -> Self {
        let recipe = "CREATE TABLE benchmarks (id int, name varchar(100), metric varchar(100));";

        let zk_auth = ZookeeperAuthority::new(&format!("{}/taster", zk_addr));
        let mut ch = ControllerHandle::new(zk_auth);

        ch.install_recipe(recipe.to_owned());

        SoupHistoryDB {
            soup: ch,

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
