use slack_hook::{Attachment, AttachmentBuilder, Field, PayloadBuilder, Slack, SlackText, SlackLink};
use slack_hook::SlackTextContent::{Text, Link};

use taste::{BenchmarkResult, TastingResult};

pub struct SlackNotifier {
    conn: Slack,
    channel: String,
    github_repo: String,
}

impl SlackNotifier {
    pub fn new(hook_url: &str, channel: &str, repo_url: &str) -> SlackNotifier {
        SlackNotifier {
            conn: Slack::new(hook_url).unwrap(),
            channel: String::from(channel),
            github_repo: String::from(repo_url),
        }
    }

    pub fn notify(&self, res: &TastingResult) -> Result<(), String> {
        let payload = PayloadBuilder::new()
            .text(vec![Text("I've tasted ".into()),
                       Text(format!("\"{}\" -- ", res.commit_msg.lines().next().unwrap()).into()),
                       Link(SlackLink::new(&format!("{}/commit/{}",
                                                    self.github_repo,
                                                    res.commit_id),
                                           &res.commit_id[0..6]))]
                .as_slice())
            .attachments(vec![result_to_attachment(&res)])
            .channel(self.channel.clone())
            .username("taster")
            .icon_emoji(":tea:")
            .build()
            .unwrap();

        match self.conn.send(&payload) {
            Ok(_) => Ok(()),
            Err(e) => Err(format!("{:?}", e)),
        }
    }
}

fn result_to_attachment(res: &TastingResult) -> Attachment {
    let color = if !res.build || !res.bench {
        "danger"
    } else {
        "good"
    };

    let title = if !res.build {
        "Build failure!"
    } else if !res.bench {
        "Benchmark failure!"
    } else {
        "Performance results:"
    };

    let taste = if !res.build || !res.bench {
        "was inedible"
    } else {
        "tasted nice"
    };

    let mut allfields = Vec::new();
    for res in &res.results {
        let mut nv = res.iter()
            .map(|(k, v)| {
                let val = match v {
                    &BenchmarkResult::Neutral(ref s, _) => s,
                    _ => unimplemented!(),
                };
                Field {
                    title: k.clone(),
                    value: SlackText::new(val.clone()),
                    short: Some(true),
                }
            })
            .collect::<Vec<_>>();
        nv.sort_by(|a, b| b.title.cmp(&a.title));
        allfields.extend(nv);
    }

    AttachmentBuilder::new(format!("It {}.", taste))
        .color(color)
        .title(title)
        .fields(allfields)
        .build()
        .unwrap()
}
