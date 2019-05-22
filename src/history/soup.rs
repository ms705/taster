use std::collections::BTreeMap;

use git2;
use noria::{SyncControllerHandle, SyncTable, SyncView, ZookeeperAuthority};
use slog;

use super::{BranchHistoryEntry, HistoryDB};

pub struct SoupHistoryDB {
    soup: SyncControllerHandle<ZookeeperAuthority, tokio::runtime::TaskExecutor>,
    log: slog::Logger,

    recipe: String,
    tables: BTreeMap<String, SyncTable>,
    views: BTreeMap<String, SyncView>,
}

impl SoupHistoryDB {
    pub fn new(zk_addr: &str, log: Option<slog::Logger>) -> Self {
        let log = match log {
            None => slog::Logger::root(slog::Discard, o!()),
            Some(l) => l,
        };

        let recipe = "CREATE TABLE benchmarks (id int, name varchar(100), PRIMARY KEY (id));
                      CREATE TABLE branches (id int, name varchar(255), PRIMARY KEY (id));
                      CREATE TABLE metrics (id int, bench_id int, name varchar(100), \
                                            PRIMARY KEY (id));
                      CREATE TABLE results (commit_id varchar(40), branch_id int, metric_id int, \
                                            result float, timestamp int, PRIMARY KEY (commit_id));

                      QUERY CommitResults: SELECT commit_id, branches.name AS branch, \
                                                  benchmarks.name AS bench, metrics.name AS metric, \
                                                  result, timestamp \
                                           FROM results \
                                           JOIN branches ON (results.branch_id = branches.id) \
                                           JOIN metrics ON (results.metric_id = metrics.id) \
                                           JOIN benchmarks ON (metrics.bench_id = benchmarks.id) \
                                           WHERE commit_id = ?;
                      QUERY BranchHeads: SELECT branches.name AS branch, commit_id AS head_commit \
                                         FROM branches \
                                         JOIN results ON (branches.id = results.branch_id) \
                                         WHERE branches.name = ? \
                                         ORDER BY results.timestamp LIMIT 1;";

        debug!(log, "Finding Soup via Zookeeper...");

        let zk_auth = ZookeeperAuthority::new(&format!("{}", zk_addr))
            .expect("failed to connect to Zookeeper");

        debug!(log, "Connecting to Soup...");
        let rt = tokio::runtime::Runtime::new().unwrap();
        let executor = rt.executor();
        let mut ch = SyncControllerHandle::new(zk_auth, executor)
            .expect("failed to connect to Soup controller");

        debug!(log, "Installing recipe in Soup...");
        ch.install_recipe(&recipe.to_owned());

        let inputs = ch
            .inputs()
            .expect("couldn't get inputs from Soup")
            .into_iter()
            .map(|(n, _)| (n.clone(), ch.table(&n).unwrap().into_sync()))
            .collect::<BTreeMap<String, SyncTable>>();
        let outputs = ch
            .outputs()
            .expect("couldn't get outputs from Soup")
            .into_iter()
            .map(|(n, _)| (n.clone(), ch.view(&n).unwrap().into_sync()))
            .collect::<BTreeMap<String, SyncView>>();

        SoupHistoryDB {
            soup: ch,
            log: log,

            recipe: recipe.to_owned(),
            tables: inputs,
            views: outputs,
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
        &mut self,
        branch: &str,
        commit: Option<&git2::Oid>,
    ) -> Result<BranchHistoryEntry, String> {
        let commit = match commit {
            None => {
                let res = self
                    .views
                    .get_mut("BranchHeads")
                    .expect("no branch heads view")
                    .lookup(&[branch.into()], true);

                println!("branch head lookup res: {:?}", res);
                let rs = res.expect("Noria lookup failed");
                if let Some(ref r) = rs.first() {
                    let cid: String = r.get(1).expect("malformed record").into();
                    git2::Oid::from_str(&cid).expect("failed to parse commit ID!")
                } else {
                    return Err("no data for branch".into());
                }
            }
            Some(c) => *c,
        };

        debug!(self.log, "reading results for {}", commit);

        let res = self
            .views
            .get_mut("CommitResults")
            .expect(&format!("no commit results view"))
            .lookup(&[format!("{}", commit).into()], true);

        println!("commit res: {:?}", res);

        unimplemented!();
    }

    fn get_branch_history(&mut self, branch: &str) -> Result<Vec<BranchHistoryEntry>, String> {
        unimplemented!();
    }
}
