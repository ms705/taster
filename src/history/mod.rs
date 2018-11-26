use std::collections::HashMap;

use git2;

use taste::BenchmarkResult;

pub mod in_memory;
#[cfg(feature = "use_noria")]
pub mod soup;

// (benchmark, (metric, result))
// TODO(malte): proper types
type BranchHistoryEntry = HashMap<String, HashMap<String, BenchmarkResult<f64>>>;

pub trait HistoryDB {
    fn put(
        &mut self,
        branch: &str,
        commit: git2::Oid,
        entry: BranchHistoryEntry,
    ) -> Result<(), String>;

    fn get_commit(
        &mut self,
        branch: &str,
        commit: Option<&git2::Oid>,
    ) -> Result<BranchHistoryEntry, String>;

    fn get_branch_history(&mut self, branch: &str) -> Result<Vec<BranchHistoryEntry>, String>;
}
