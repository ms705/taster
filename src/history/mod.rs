use std::collections::HashMap;

use git2;

use taste::BenchmarkResult;

pub mod in_memory;
#[cfg(soup)]
pub mod soup;

// (benchmark, (metric, result))
// TODO(malte): proper types
type BranchHistoryEntry = HashMap<String, HashMap<String, BenchmarkResult<f64>>>;

// TODO(malte): should not need to be `Send` once we have a tasting queue
pub trait HistoryDB: Send {
    fn put(
        &mut self,
        branch: &str,
        commit: git2::Oid,
        entry: BranchHistoryEntry,
    ) -> Result<(), String>;

    fn get_commit(
        &self,
        branch: &str,
        commit: Option<&git2::Oid>,
    ) -> Result<BranchHistoryEntry, String>;

    fn get_branch_history(&self, branch: &str) -> Result<Vec<BranchHistoryEntry>, String>;
}
