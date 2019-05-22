extern crate rifling;
#[macro_use]
extern crate clap;
extern crate git2;
extern crate github_rs;
extern crate hyper;
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
mod github;
mod history;
mod repo;
mod slack;
mod taste;
mod taster;

use hyper::rt::Future;
use hyper::server::Server;

use rifling::{Constructor, Delivery, DeliveryType, Hook};

use std::sync::mpsc::channel;
use std::sync::{Arc, Mutex};
use std::thread;

use common::{Commit, Push};
use taster::Taster;

fn parse_push(payload: &serde_json::Value) -> (Push, Vec<Commit>) {
    let payload = payload.as_object().unwrap();

    let commits = payload["commits"].as_array().unwrap();
    let head_commit = &payload["head"];
    let pusher = &payload["pusher"];
    let repository = &payload["repository"];

    let stringify = |v: &serde_json::Value| -> String { v.as_str().unwrap().to_owned() };

    // Data structures to represent info from webhook
    let commits: Vec<Commit> = commits
        .iter()
        .map(|c| Commit {
            id: git2::Oid::from_str(c["id"].as_str().unwrap()).unwrap(),
            msg: stringify(&c["message"]),
            url: stringify(&c["url"]),
        })
        .collect();
    let hc = Commit {
        id: git2::Oid::from_str(head_commit["id"].as_str().unwrap()).unwrap(),
        msg: stringify(&head_commit["message"]),
        url: stringify(&head_commit["url"]),
    };
    let push = Push {
        head_commit: hc.clone(),
        push_ref: Some(stringify(&payload["ref"])),
        pusher: Some(stringify(&pusher["name"])),
        owner_name: Some(stringify(&repository["owner"]["name"])),
        repo_name: Some(stringify(&repository["name"])),
    };

    (push, commits)
}

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
    let txl = Arc::new(Mutex::new(tx.clone()));

    let mut cons = Constructor::new();
    let hook_log = log.clone();
    let hook = Hook::new(
        "push",
        args.github_hook_secret,
        move |delivery: &Delivery| {
            if let Some(payload) = &delivery.payload {
                match delivery.delivery_type {
                    DeliveryType::GitHub => {
                        let (push, commits) = parse_push(payload);
                        info!(
                            hook_log,
                            "Handling {} commits pushed by {}",
                            commits.len(),
                            push.pusher.as_ref().unwrap_or(&"anonymous".to_owned())
                        );

                        // enqueue for tasting
                        let txl = txl.lock().unwrap();
                        {
                            txl.send((push, commits)).unwrap();
                        }
                    }
                    _ => unimplemented!(),
                }
            }
        },
    );

    cons.register(hook);

    let server = Server::bind(&args.listen_addr)
        .serve(cons)
        .map_err(|e| eprintln!("Error: {:#?}", e));

    thread::spawn(move || {
        hyper::rt::run(server);
    });

    info!(
        common::new_logger(),
        "Taster listening on {}", args.listen_addr
    );

    loop {
        if let Ok((push, commits)) = rx.recv() {
            t.notify_pending(&push, &push.head_commit);

            t.taste(push, commits);
        } else {
            panic!("Failed to receive on tasting channel!");
        }
    }
}
