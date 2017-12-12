use clap::{App, Arg, ErrorKind};
use std::net::SocketAddr;

#[cfg_attr(rustfmt, rustfmt_skip)]
const TASTER_USAGE: &'static str = "\
EXAMPLES:
  taster -w /path/to/workdir -s my_secret
  taster -l 0.0.0.0:1234 -w /path/to/workdir -s my_secret";

#[derive(Clone, Debug)]
pub struct Args {
    pub listen_addr: SocketAddr,
    pub workdir: String,

    pub repo: String,

    pub github_hook_secret: Option<String>,
    pub github_api_key: Option<String>,

    pub email_notification_addr: Option<String>,
    pub slack_hook_url: Option<String>,
    pub slack_channel: Option<String>,

    pub history_db: HistoryDBProvider,

    pub taste_head_only: bool,
    pub verbose_notify: bool,

    pub improvement_threshold: f64,
    pub regression_threshold: f64,
    pub timeout: Option<u64>,
}

#[cfg(feature = "soup")]
arg_enum!{
    #[derive(PartialEq, Debug, Clone)]
    pub enum HistoryDBProvider {
        InMemory,
        Soup,
    }
}
#[cfg(not(feature = "soup"))]
arg_enum!{
    #[derive(PartialEq, Debug, Clone)]
    pub enum HistoryDBProvider {
        InMemory,
    }
}

pub fn parse_args() -> Args {
    use std::str::FromStr;

    let args = App::new("taster")
        .version("0.0.1")
        .about("Tastes GitHub commits.")
        .arg(
            Arg::with_name("listen_addr")
                .short("l")
                .long("listen_addr")
                .takes_value(true)
                .value_name("IP:PORT")
                .default_value("0.0.0.0:4567")
                .help("Listen address and port for webhook delivery"),
        )
        .arg(
            Arg::with_name("github_repo")
                .short("r")
                .long("github_repo")
                .takes_value(true)
                .required(true)
                .value_name("GH_REPO")
                .default_value("https://github.com/ms705/taster")
                .help("GitHub repository to taste"),
        )
        .arg(
            Arg::with_name("email_addr")
                .long("email_addr")
                .takes_value(true)
                .required(false)
                .help("Email address to send notifications to"),
        )
        .arg(
            Arg::with_name("default_regression_reporting_threshold")
                .long("default_regression_reporting_threshold")
                .takes_value(true)
                .default_value("0.1")
                .help(
                    "Relative performance threshold below which a result is considered a \
                     regression that needs reporting (0.1 = +/-10%).",
                ),
        )
        .arg(
            Arg::with_name("default_improvement_reporting_threshold")
                .long("default_improvement_reporting_threshold")
                .takes_value(true)
                .default_value("0.1")
                .help(
                    "Relative performance threshold above which a result is considered an \
                     improvement that needs reporting (0.1 = +/-10%).",
                ),
        )
        .arg(
            Arg::with_name("history_db")
                .long("history-db")
                .takes_value(true)
                .possible_values(&HistoryDBProvider::variants())
                .case_insensitive(true)
                .help("History storage provider to use."),
        )
        .arg(
            Arg::with_name("secret")
                .short("s")
                .long("secret")
                .takes_value(true)
                .required(false)
                .help("GitHub webhook secret"),
        )
        .arg(
            Arg::with_name("slack_hook_url")
                .long("slack_hook_url")
                .takes_value(true)
                .required(false)
                .help("Slack webhook URL to push notifications to"),
        )
        .arg(
            Arg::with_name("slack_channel")
                .long("slack_channel")
                .takes_value(true)
                .required(false)
                .default_value("#soup-test")
                .help("Slack channel for notifications"),
        )
        .arg(
            Arg::with_name("github_api_key")
                .long("github_api_key")
                .takes_value(true)
                .required(false)
                .help("GitHub API key to provide status notifications"),
        )
        .arg(
            Arg::with_name("taste_head_only")
                .long("taste_head_only")
                .required(false)
                .help("When multiple commits are pushed, taste the head commit only"),
        )
        .arg(
            Arg::with_name("timeout")
                .long("timeout")
                .required(false)
                .takes_value(true)
                .help("Timeout (in seconds) after which benchmarks should be killed"),
        )
        .arg(
            Arg::with_name("verbose_notifications")
                .long("verbose_notifications")
                .required(false)
                .help(
                    "List all benchmarks in notifications even if the results have not changed \
                     significantly",
                ),
        )
        .arg(
            Arg::with_name("workdir")
                .short("w")
                .long("workdir")
                .takes_value(true)
                .required(true)
                .value_name("REPO_DIR")
                .help("Directory holding the workspace repo"),
        )
        .after_help(TASTER_USAGE)
        .get_matches();

    Args {
        listen_addr: SocketAddr::from_str(args.value_of("listen_addr").unwrap()).unwrap(),
        workdir: String::from(args.value_of("workdir").unwrap()),

        repo: String::from(args.value_of("github_repo").unwrap()),

        github_hook_secret: args.value_of("secret").map(String::from),
        github_api_key: args.value_of("github_api_key").map(String::from),

        email_notification_addr: args.value_of("email_addr").map(String::from),
        slack_hook_url: args.value_of("slack_hook_url").map(String::from),
        slack_channel: args.value_of("slack_channel").map(String::from),

        history_db: value_t!(args, "history_db", HistoryDBProvider).unwrap_or_else(|e| e.exit()),

        taste_head_only: args.is_present("taste_head_only"),
        verbose_notify: args.is_present("verbose_notifications"),
        improvement_threshold: value_t_or_exit!(
            args,
            "default_improvement_reporting_threshold",
            f64
        ),
        regression_threshold: value_t_or_exit!(args, "default_regression_reporting_threshold", f64),
        timeout: match value_t!(args, "timeout", u64) {
            Ok(timeout) => Some(timeout),
            Err(e) => match e.kind {
                ErrorKind::ArgumentNotFound => None,
                _ => panic!("failed to parse timeout: {:?}", e),
            },
        },
    }
}
