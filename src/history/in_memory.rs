use std::collections::HashMap;

use git2;

use super::{BranchHistoryEntry, HistoryDB};

pub struct InMemoryHistoryDB {
    db: HashMap<String, HashMap<git2::Oid, BranchHistoryEntry>>,
    heads: HashMap<String, git2::Oid>,
}

impl InMemoryHistoryDB {
    pub fn new() -> Self {
        InMemoryHistoryDB {
            db: HashMap::new(),
            heads: HashMap::new(),
        }
    }
}

impl HistoryDB for InMemoryHistoryDB {
    fn put(
        &mut self,
        branch: &str,
        commit: git2::Oid,
        entry: BranchHistoryEntry,
    ) -> Result<(), String> {
        let bh = self.db.entry(branch.to_string()).or_insert(HashMap::new());
        bh.insert(commit, entry);
        self.heads.insert(branch.to_owned(), commit);

        Ok(())
    }

    fn get_commit(
        &self,
        branch: &str,
        commit: Option<&git2::Oid>,
    ) -> Result<BranchHistoryEntry, String> {
        let commit = match commit {
            None => {
                // use HEAD
                self.heads.get(branch).unwrap()
            }
            Some(c) => c,
        };

        match self.db.get(branch) {
            None => Err(format!("no history found for branch {}", branch)),
            Some(ref bh) => match bh.get(commit) {
                None => Err(format!(
                    "commit {} not found in history for branch {}!",
                    commit,
                    branch
                )),
                Some(ch) => Ok(ch.clone()),
            },
        }
    }

    fn get_branch_history(&self, _branch: &str) -> Result<Vec<BranchHistoryEntry>, String> {
        unimplemented!();
    }
}
