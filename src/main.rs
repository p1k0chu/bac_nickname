use bac_nickname::api;
use notify::event::{EventKind, ModifyKind};
use notify::{Event, RecursiveMode, Watcher};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::env;
use std::error::Error;
use std::path::Path;
use tokio::fs;
use tokio::sync::mpsc::{self, Receiver, Sender};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();
    let config: &str = if args.len() > 1 {
        &args[1]
    } else {
        "config.json"
    };
    let config: Config = parse_config(Path::new(config)).await?;

    let (fs_events_tx, mut fs_events_rx) = mpsc::channel::<Event>(5);
    let fs_events_tx = bac_nickname::AsyncSender(fs_events_tx);
    // new mock event to garantee first iteration
    fs_events_tx.send(Event::new(EventKind::Any)).await?;

    let (discord_tx, discord_rx) = mpsc::channel::<String>(5);

    let mut watcher = notify::recommended_watcher(fs_events_tx)?;
    watcher.watch(
        Path::new(&config.advancements_dir),
        RecursiveMode::NonRecursive,
    )?;

    let mut async_handles = Vec::new();
    async_handles.push(tokio::spawn(nicknames_receiver(
        config.token.clone(),
        config.servers.clone(),
        discord_rx,
    )));

    while let Some(e) = fs_events_rx.recv().await {
        println!("Received fs event!");
        match e.kind {
            // ignore because file isn't modified
            EventKind::Access(_) => continue,
            EventKind::Remove(_) => continue,
            EventKind::Modify(v) => match v {
                ModifyKind::Data(_) => (),
                // ignore because only Data is related to file content
                _ => continue,
            },
            _ => (),
        }

        let adv_dir = config.advancements_dir.clone();
        let nickname_template = config.nickname.clone();

        async_handles.push(tokio::spawn(make_nickname(
            adv_dir,
            nickname_template,
            discord_tx.clone(),
        )));
    }

    for handle in async_handles {
        handle.await.unwrap();
    }

    Ok(())
}

async fn nicknames_receiver(token: String, servers: Vec<u64>, mut rx: Receiver<String>) {
    while let Some(nickname) = rx.recv().await {
        dbg!(&nickname);

        let mut handles = Vec::new();

        for server in &servers {
            let body = json!({"nick": nickname});
            let url = format!("https://discord.com/api/v10/guilds/{}/members/@me", server);
            dbg!(&url);
            dbg!(&body);
            handles.push(tokio::spawn(api::post(url.clone(), token.clone(), body.clone())));
        }

        for handle in handles {
            if let Ok(Some(code)) = handle.await {
                println!("{}", code);
            }
        }
    }
}

async fn make_nickname(adv_dir: String, nickname_template: String, tx: Sender<String>) {
    println!("Nickname maker!");
    let json_obj = match bac_nickname::parse_and_merge(&Path::new(&adv_dir)).await {
        Ok(x) => x,
        Err(e) => {
            eprintln!("{e}");
            return;
        }
    };
    let nickname = bac_nickname::replace_with_progress(&nickname_template, &json_obj);
    let _ = tx.send(nickname).await;
}

async fn parse_config(path: &Path) -> Result<Config, Box<dyn Error>> {
    let s = fs::read_to_string(path).await?;

    Ok(serde_json::from_str::<Config>(&s)?)
}

/// Config loaded from a config file
#[derive(Serialize, Deserialize)]
pub struct Config {
    /// discord user's token
    pub token: String,
    /// nickname pattern
    ///
    /// # Example
    /// `Name (blazeandcave:bacap/advancement_legend)/1169`
    /// where text in () is parsed and replaced with advancement with corresponding id progress
    pub nickname: String,
    /// discord server id's
    pub servers: Vec<u64>,
    /// minecraft advancements directory (inside of world folder)
    pub advancements_dir: String,
}
