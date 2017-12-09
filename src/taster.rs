use args::Args;
use common::{self, Commit, Push};
use config::Config;
use email;
use github;
use history::History;
use repo::{self, Workspace};
use slack;
use taste;

use slog;
use std::path::Path;

pub struct Taster {
    args: Args,
    log: slog::Logger,

    ws: Workspace,
    history: History,

    en: Option<email::EmailNotifier>,
    gn: Option<github::GithubNotifier>,
    sn: Option<slack::SlackNotifier>,
}

impl Taster {
    pub fn new(args: Args) -> Self {
        let repo = args.repo.clone();
        let wd = args.workdir.clone();
        let workdir = Path::new(&wd);

        let en = if let Some(ref addr) = args.email_notification_addr {
            Some(email::EmailNotifier::new(addr, &repo))
        } else {
            None
        };
        let sn = if let Some(ref url) = args.slack_hook_url {
            Some(slack::SlackNotifier::new(
                url,
                args.slack_channel.as_ref().unwrap(),
                &args.repo,
                args.verbose_notify,
            ))
        } else {
            None
        };
        let gn = if let Some(ref key) = args.github_api_key {
            Some(github::GithubNotifier::new(key))
        } else {
            None
        };

        Taster {
            args: args,
            log: common::new_logger(),

            ws: repo::Workspace::new(&repo, workdir),
            history: History::new(),

            gn: gn,
            sn: sn,
            en: en,
        }
    }

    pub fn bootstrap(&mut self) {
        // Initialize history by tasting the HEAD commit of each branch
        let branches = self.ws.branch_heads();
        for (b, c) in branches.iter() {
            if b != "origin/master" {
                continue;
            }
            info!(
                self.log,
                "tasting HEAD of {}: {} / {}",
                b,
                c.id(),
                c.message().unwrap()
            );
            let hc = Commit {
                id: c.id(),
                msg: String::from(c.message().unwrap()),
                url: format!("{}/commit/{}", self.args.repo, c.id()),
            };
            // fake a push
            let push = Push {
                head_commit: hc,
                push_ref: Some(b.clone()),
                pusher: None,
                owner_name: None,
                repo_name: None,
            };
            let res = taste::taste_commit(
                &self.ws,
                &mut self.history,
                &push,
                &push.head_commit,
                self.args.improvement_threshold,
                self.args.regression_threshold,
                self.args.timeout,
            );
            assert!(res.is_ok());
        }
    }

    pub fn notify_pending(&mut self, push: &Push, commit: &Commit) {
        // github status notification
        if self.gn.is_some() {
            match self.gn.as_ref().unwrap().notify_pending(&push, &commit) {
                Ok(_) => (),
                Err(e) => error!(
                    self.log,
                    "failed to deliver GitHub status notification: {:?}",
                    e
                ),
            }
        }
    }

    pub fn notify(
        &mut self,
        cfg: Option<&Config>,
        res: &taste::TastingResult,
        push: &Push,
        commit: &Commit,
    ) {
        // email notification
        if self.en.is_some() {
            self.en.as_ref().unwrap().notify(cfg, &res, &push).unwrap();
        }
        // slack notification
        if self.sn.is_some() {
            self.sn.as_ref().unwrap().notify(cfg, &res, &push).unwrap();
        }
        // github status notification
        if self.gn.is_some() {
            self.gn
                .as_ref()
                .unwrap()
                .notify(cfg, &res, &push, &commit)
                .unwrap();
        }
    }

    pub fn taste(&mut self, push: Push, hc: Commit, commits: Vec<Commit>) {
        // First taste the head commit
        self.ws.fetch().unwrap();
        let head_res = taste::taste_commit(
            &self.ws,
            &mut self.history,
            &push,
            &push.head_commit,
            self.args.improvement_threshold,
            self.args.regression_threshold,
            self.args.timeout,
        );
        match head_res {
            Err(e) => error!(self.log, "failed to taste HEAD commit {}: {}", hc.id, e),
            Ok((cfg, tr)) => {
                self.notify(cfg.as_ref(), &tr, &push, &push.head_commit);
                // Taste others if needed
                if !self.args.taste_head_only {
                    for c in commits.iter() {
                        if c.id == hc.id {
                            // skip HEAD as we've already tested it
                            continue;
                        }
                        let cur_c = Commit {
                            id: c.id.clone(),
                            msg: c.msg.clone(),
                            url: c.url.clone(),
                        };
                        self.notify_pending(&push, &cur_c);
                        // taste
                        let res = taste::taste_commit(
                            &self.ws,
                            &mut self.history,
                            &push,
                            &cur_c,
                            self.args.improvement_threshold,
                            self.args.regression_threshold,
                            self.args.timeout,
                        );
                        match res {
                            Err(e) => error!(self.log, "failed to taste commit {}: {}", c.id, e),
                            Ok((cfg, tr)) => self.notify(cfg.as_ref(), &tr, &push, &cur_c),
                        }
                    }
                } else if !commits.is_empty() {
                    info!(
                        self.log,
                        "Skipping {} remaining commits in push!",
                        commits.len() - 1
                    );
                }
            }
        }
    }
}
