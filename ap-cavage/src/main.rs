use serde::{Serialize, Deserialize};
use url::Url;

use clap::Parser;
use clap_repl::ClapEditor;
use console::style;
use util::requester::SignedRequester;

mod config;
mod util;

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Name of the person to greet
    #[arg(short, long)]
    config: String,
}

#[derive(Debug, Parser)]
#[command(name = "")] // This name will show up in clap's error messages, so it is important to set it to "".
enum ShellCommand {
    Keys {
        #[arg(short, long)]
        set: Option<usize>,
    },
    Get {
        #[arg(short, long)]
        url: String,
    },
    Post {
        #[arg(short, long)]
        url: String,
        #[arg(long)]
        payload: Option<String>,
        #[arg(long)]
        payload_file: Option<String>,
    },
    Action {
        #[arg(short, long)]
        url: String,
        #[arg(long)]
        action: String,
        #[arg(long)]
        id: String,
        #[arg(long)]
        target: String,
    },
    Response {
        #[arg(short, long)]
        url: String,
        #[arg(long)]
        action: String,
        #[arg(long)]
        id: String,
        #[arg(long)]
        req_id: String,
        #[arg(long)]
        req_type: String,
        #[arg(long)]
        req_actor: String,
        #[arg(long)]
        req_object: String,
    }
}

// Actually, AS Objct
#[derive(Serialize, Deserialize, Debug)]
struct Action {
    #[serde(rename = "@context")]
    context: String,
    id: String,
    r#type: String,
    actor: String,
    object: String,
}

impl Action {
    pub fn new(id: String, r#type: String, actor: String, object: String) -> Action {
        Action {
            context: "https://www.w3.org/ns/activitystreams".to_owned(),
            id,
            r#type,
            actor,
            object,
        }
    }
}

// Actually, AS Object, but Undo / Accept
#[derive(Serialize, Deserialize, Debug)]
struct ActionResponse {
    #[serde(rename = "@context")]
    context: String,
    id: String,
    r#type: String,
    actor: String,
    object: Action,
}

impl ActionResponse {
    pub fn new(id: String, r#type: String, actor: String, object: Action) -> ActionResponse {
        ActionResponse {
            context: "https://www.w3.org/ns/activitystreams".to_owned(),
            id,
            r#type,
            actor,
            object,
        }
    }
}

struct SignedRequesterEntry {
    pub requester: SignedRequester,
    pub actor: String,
    pub key_id: String,
}

fn parse_and_print_result(result: anyhow::Result<String>) {
    match result {
        Ok(result) => {
            println!("Request successful! (so please do not execute the command again)");
            match serde_json::from_str::<serde_json::Value>(&result) {
                Ok(result) => {
                    println!("{:#?}", result);
                }
                Err(e) => {
                    println!("Failed to parse received JSON: {:?}\npayload: {}", e, result);
                }
            }
        }
        Err(e) => {
            println!("{}", style(e).red());
        }
    }
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    let cfg = config::Config::from_file(&args.config);
    let signed_requesters: Vec<SignedRequesterEntry> = cfg.key.iter().map(|k| {
        SignedRequesterEntry {
            requester: SignedRequester::new(&k.pem, &k.id),
            actor: k.actor.clone(),
            key_id: k.id.clone(),
        }
    }).collect();

    // Use `ClapEditor` instead of the `rustyline::DefaultEditor`.
    let mut rl = ClapEditor::<ShellCommand>::new();
    let mut keyslot = 0;
    loop {
        // Use `read_command` instead of `readline`.
        let Some(command) = rl.read_command() else {
            continue;
        };
        match command {
            ShellCommand::Keys { set } => {
                match set {
                    None => {
                        for (idx, sr) in signed_requesters.iter().enumerate() {
                            println!("{}: {}", idx, sr.key_id);
                        }
                    }
                    Some(x) => {
                        if x >= signed_requesters.len() {
                            println!("Invalid keyslot");
                            continue;
                        }
                        keyslot = x;
                    }
                }

            }
            ShellCommand::Get{url} => {
                let url = match Url::parse(&url) {
                    Ok(url) => url,
                    Err(e) => {
                        println!("URL failed: {}", e);
                        continue;
                    }
                };
                let sr = &signed_requesters[keyslot];
                let result = sr.requester.get(url).await;
                parse_and_print_result(result);
            }
            ShellCommand::Post { url, payload, payload_file } => {
                let pl = match (payload, payload_file) {
                    (Some(payload), _) => {
                        payload
                    },
                    (None, Some(file)) => {
                        match std::fs::read_to_string(file) {
                            Ok(str) => str,
                            Err(e) => {
                                println!("Failed to read file! {:?}", e);
                                continue;
                            }
                        }
                    },
                    (None, None) => {
                        println!("--payload or --payload-file is required. Please use --help to get detailed information.");
                        continue;
                    }
                };

                let url = match Url::parse(&url) {
                    Ok(url) => url,
                    Err(e) => {
                        println!("URL failed: {:?}", e);
                        continue;
                    }
                };
                let payload = match serde_json::from_str::<serde_json::Value>(&pl) {
                    Ok(payload) => payload,
                    Err(e) => {
                        println!("JSON failed: {:?}", e);
                        continue;
                    }
                };

                let sr = &signed_requesters[keyslot];
                let result = sr.requester.post(url, payload).await;
                parse_and_print_result(result);
            }
            ShellCommand::Action { url, action, id, target} => {
                let url = match Url::parse(&url) {
                    Ok(url) => url,
                    Err(e) => {
                        println!("URL failed: {}", e);
                        continue;
                    }
                };
                let sr = &signed_requesters[keyslot];
                let action = Action::new(
                    id,
                    action,
                    sr.actor.clone(),
                    target,
                );
                let result = sr.requester.post(url, serde_json::to_value(action).unwrap()).await;
                parse_and_print_result(result);
            }
            ShellCommand::Response { url, action, id, req_id, req_type, req_actor, req_object } => {
                let url = match Url::parse(&url) {
                    Ok(url) => url,
                    Err(e) => {
                        println!("URL failed: {}", e);
                        continue;
                    }
                };
                let sr = &signed_requesters[keyslot];
                let action = ActionResponse::new(
                    id,
                    action,
                    sr.actor.clone(),
                    Action::new(
                        req_id,
                        req_type,
                        req_actor,
                        req_object,
                    ),
                );
                let result = sr.requester.post(url, serde_json::to_value(action).unwrap()).await;
                parse_and_print_result(result);
            }
        }
    }
}
