#![feature(nll)]

extern crate afterparty;
#[macro_use]
extern crate clap;
extern crate git2;
extern crate github_rs;
extern crate hyper;
extern crate lettre;
#[cfg(feature = "use_noria")]
extern crate noria;
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
mod github;
mod history;
mod repo;
mod slack;
mod taste;
mod taster;

use afterparty::{Delivery, Event, Hub};
use hyper::server::Server;
use std::sync::mpsc::channel;
use std::sync::Mutex;

use common::{Commit, Push};
use taster::Taster;

pub fn main() {
    let args = args::parse_args();
    let log = common::new_logger();

    // We currently always need a GitHub hook secret
    if let None = args.github_hook_secret {
        panic!("--secret must be set when in continuous webhook handler mode");
    }

    let mut t = Taster::new(args.clone());

    // Bootstrap with current HEAD commits
    t.bootstrap();

    let (tx, rx) = channel();
    let txl = Mutex::new(tx.clone());

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
                    info!(
                        log,
                        "Handling {} commits pushed by {}",
                        commits.len(),
                        pusher.name
                    );

                    // Data structures to represent info from webhook
                    let commits: Vec<Commit> = commits
                        .iter()
                        .map(|c| Commit {
                            id: git2::Oid::from_str(&c.id).unwrap(),
                            msg: c.message.clone(),
                            url: c.url.clone(),
                        }).collect();
                    let hc = Commit {
                        id: git2::Oid::from_str(&head_commit.id).unwrap(),
                        msg: head_commit.message.clone(),
                        url: head_commit.url.clone(),
                    };
                    let push = Push {
                        head_commit: hc.clone(),
                        push_ref: Some(_ref.clone()),
                        pusher: Some(pusher.name.clone()),
                        owner_name: Some(repository.owner.name.clone()),
                        repo_name: Some(repository.name.clone()),
                    };

                    let txl = txl.lock().unwrap();
                    {
                        txl.send((push, commits)).unwrap();
                    }
                }
                _ => (),
            }
        },
    );

    let srvc = Server::http(&args.listen_addr).unwrap().handle(hub);

    info!(
        common::new_logger(),
        "Taster listening on {}",
        args.listen_addr
    );
    srvc.unwrap();

    while let Ok((push, commits)) = rx.recv() {
        t.notify_pending(&push, &push.head_commit);

        t.taste(push, commits);
    }
}
