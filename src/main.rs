extern crate afterparty;
#[macro_use]
extern crate clap;
extern crate git2;
extern crate github_rs;
extern crate hyper;
extern crate lettre;
extern crate regex;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate slack_hook;
#[macro_use]
extern crate slog;
extern crate slog_term;
extern crate toml;

mod args;
mod auth;
mod common;
mod config;
mod email;
mod history;
mod repo;
mod slack;
mod taste;
mod github;

use afterparty::{Delivery, Event, Hub};
use hyper::Server;
use std::collections::HashMap;
use std::error::Error;
use std::path::Path;
use std::sync::{Arc, Mutex};

use common::{Commit, Push};
use config::Config;
use history::History;

pub fn main() {
    let args = args::parse_args();
    let workdir = Path::new(&args.workdir);

    let log = common::new_logger();

    let mut history = History::new();
    let ws = repo::Workspace::new(&args.repo, workdir);

    let en = if let Some(ref addr) = args.email_notification_addr {
        Some(email::EmailNotifier::new(addr, &args.repo))
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

    if args.taste_commit.is_some() {
        let cid = if args.taste_commit.is_some() && args.taste_commit.as_ref().unwrap() == "HEAD" {
            ws.repo.head().unwrap().target().unwrap().clone()
        } else {
            git2::Oid::from_str(args.taste_commit.as_ref().unwrap()).unwrap()
        };
        match ws.repo.find_object(cid, None) {
            Err(e) => panic!(format!("{}", e.description())),
            Ok(o) => {
                let cobj = o.as_commit().unwrap();
                let hc = Commit {
                    id: cobj.id(),
                    msg: String::from(cobj.message().unwrap()),
                    url: format!("{}/commit/{}", args.repo, cobj.id()),
                };
                // fake a push
                let push = Push {
                    head_commit: hc,
                    push_ref: None,
                    pusher: None,
                    owner_name: None,
                    repo_name: None,
                };
                let res = taste::taste_commit(
                    &ws,
                    &mut history,
                    &push,
                    &push.head_commit,
                    args.improvement_threshold,
                    args.regression_threshold,
                    args.timeout,
                );
                match res {
                    Err(e) => error!(log, "failed to taste {}: {}", cid, e),
                    Ok((cfg, tr)) => {
                        // email notification
                        if en.is_some() {
                            en.as_ref()
                                .unwrap()
                                .notify(cfg.as_ref(), &tr, &push)
                                .unwrap();
                        }
                        // slack notification
                        if sn.is_some() {
                            sn.as_ref()
                                .unwrap()
                                .notify(cfg.as_ref(), &tr, &push)
                                .unwrap();
                        }
                        // We're done
                        return;
                    }
                }
            }
        };
    }

    // If we get here, we must be running in continuous mode
    if let None = args.github_hook_secret {
        panic!("--secret must be set when in continuous webhook handler mode");
    }

    // Initialize history by tasting the HEAD commit of each branch
    {
        let branches = ws.branch_heads();
        for (b, c) in branches.iter() {
            if b != "origin/master" {
                continue;
            }
            info!(log,
                "tasting HEAD of {}: {} / {}",
                b,
                c.id(),
                c.message().unwrap()
            );
            let hc = Commit {
                id: c.id(),
                msg: String::from(c.message().unwrap()),
                url: format!("{}/commit/{}", args.repo, c.id()),
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
                &ws,
                &mut history,
                &push,
                &push.head_commit,
                args.improvement_threshold,
                args.regression_threshold,
                args.timeout,
            );
            assert!(res.is_ok());
        }
    }

    let hl = Arc::new(Mutex::new(history));
    let wsl = Mutex::new(ws);

    let mv_args = args.clone();
    let mut hub = Hub::new();
    hub.handle_authenticated(
        "push",
        args.github_hook_secret.unwrap(),
        move |delivery: &Delivery| {
            match delivery.payload {
                Event::Push {
                    ref _ref,
                    ref commits,
                    ref head_commit,
                    ref pusher,
                    ref repository,
                    ..
                } => {
                    info!(log,
                        "Handling {} commits pushed by {}",
                        commits.len(),
                        pusher.name
                    );

                    // Data structures to represent info from webhook
                    let hc = Commit {
                        id: git2::Oid::from_str(&head_commit.id).unwrap(),
                        msg: head_commit.message.clone(),
                        url: head_commit.url.clone(),
                    };
                    let push = Push {
                        head_commit: hc,
                        push_ref: Some(_ref.clone()),
                        pusher: Some(pusher.name.clone()),
                        owner_name: Some(repository.owner.name.clone()),
                        repo_name: Some(repository.name.clone()),
                    };

                    let notify_pending = |push: &Push, commit: &Commit| {
                        // github status notification
                        if gn.is_some() {
                            match gn.as_ref().unwrap().notify_pending(&push, &commit) {
                                Ok(_) => (),
                                Err(e) => error!(log,
                                    "failed to deliver GitHub status notification: {:?}",
                                    e
                                ),
                            }
                        }
                    };

                    let notify = |cfg: Option<&Config>,
                                  res: &taste::TastingResult,
                                  push: &Push,
                                  commit: &Commit| {
                        // email notification
                        if en.is_some() {
                            en.as_ref().unwrap().notify(cfg, &res, &push).unwrap();
                        }
                        // slack notification
                        if sn.is_some() {
                            sn.as_ref().unwrap().notify(cfg, &res, &push).unwrap();
                        }
                        // github status notification
                        if gn.is_some() {
                            gn.as_ref()
                                .unwrap()
                                .notify(cfg, &res, &push, &commit)
                                .unwrap();
                        }
                    };

                    {
                        notify_pending(&push, &push.head_commit);
                        let ws = wsl.lock().unwrap();
                        let mut history = hl.lock().unwrap();
                        // First taste the head commit
                        ws.fetch().unwrap();
                        let head_res = taste::taste_commit(
                            &ws,
                            &mut history,
                            &push,
                            &push.head_commit,
                            mv_args.improvement_threshold,
                            mv_args.regression_threshold,
                            mv_args.timeout,
                        );
                        match head_res {
                            Err(e) => error!(log,
                                "failed to taste HEAD commit {}: {}",
                                head_commit.id,
                                e
                            ),
                            Ok((cfg, tr)) => {
                                notify(cfg.as_ref(), &tr, &push, &push.head_commit);
                                // Taste others if needed
                                if !mv_args.taste_head_only {
                                    for c in commits.iter() {
                                        if c.id == head_commit.id {
                                            // skip HEAD as we've already tested it
                                            continue;
                                        }
                                        let cur_c = Commit {
                                            id: git2::Oid::from_str(&c.id).unwrap(),
                                            msg: c.message.clone(),
                                            url: c.url.clone(),
                                        };
                                        notify_pending(&push, &cur_c);
                                        // taste
                                        let res = taste::taste_commit(
                                            &ws,
                                            &mut history,
                                            &push,
                                            &cur_c,
                                            mv_args.improvement_threshold,
                                            mv_args.regression_threshold,
                                            mv_args.timeout,
                                        );
                                        match res {
                                            Err(e) => error!(log,
                                                "failed to taste commit {}: {}",
                                                c.id,
                                                e
                                            ),
                                            Ok((cfg, tr)) => {
                                                notify(cfg.as_ref(), &tr, &push, &cur_c)
                                            }
                                        }
                                    }
                                } else if !commits.is_empty() {
                                    info!(log,
                                        "Skipping {} remaining commits in push!",
                                        commits.len() - 1
                                    );
                                }
                            }
                        }
                    }
                }
                _ => (),
            }
        },
    );

    let srvc = Server::http(&args.listen_addr).unwrap().handle(hub);

    info!(common::new_logger(), "Taster listening on {}", args.listen_addr);
    srvc.unwrap();
}
