use std::{
    fs::{self, File},
    io::{self, BufReader},
    path::PathBuf,
    process::ExitCode,
};

use clap::Parser;
use config::{Config, SocialAccount};
use futures::future::join_all;
use sources::vk::VKGroupFeed;
use telegram::{TelegramClient, TelegramError};
use tokio::time;

use crate::{config::CacheRecords, sources::vk::VKClient};

mod config;
mod sources;
mod telegram;

#[derive(Parser)]
#[command(version, about)]
struct Args {
    /// Path to configuration file.
    #[arg(long, default_value = ".secretfop.yml")]
    config: PathBuf,

    /// Path to cache file.
    #[arg(long, default_value = ".cache.secretfop.json")]
    cache: PathBuf,

    /// Populate cache, but not post
    #[arg(long)]
    populate: bool,
}

#[tokio::main]
async fn main() -> ExitCode {
    let Args {
        config,
        cache,
        populate,
    } = Args::parse();

    let cfg: Config = {
        let file = match File::open(config) {
            Ok(f) => BufReader::new(f),
            Err(e) => {
                eprintln!("Failed to open config: {e}");
                return ExitCode::FAILURE;
            }
        };

        match serde_yaml::from_reader(file) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("Failed to parse config: {e}");
                return ExitCode::FAILURE;
            }
        }
    };

    let cache_records: CacheRecords = 'brk: {
        let file = match File::open(&cache) {
            Ok(f) => BufReader::new(f),
            Err(e) if e.kind() == io::ErrorKind::NotFound => break 'brk CacheRecords::new(),
            Err(e) => {
                eprintln!("Failed to open cache file: {e}");
                return ExitCode::FAILURE;
            }
        };

        match serde_json::from_reader(file) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("Warning: Failed to parse cache file: {e}");

                CacheRecords::new()
            }
        }
    };
    let mut new_cache_records = cache_records.clone();

    let vk = VKClient::new(cfg.vk_token);

    let feeds: Vec<VKGroupFeed> = {
        let jobs = cfg
            .vk
            .iter()
            .map(|SocialAccount { id, .. }| vk.get_posts(id.as_ref()).send());

        join_all(jobs)
            .await
            .into_iter()
            .filter_map(|v| match v {
                Ok(v) => Some(v),
                Err(e) => {
                    eprintln!("Failed to fetch posts: {e}");
                    None
                }
            })
            .collect()
    };
    let posts = feeds.iter().flat_map(VKGroupFeed::as_iter).filter(|f| {
        !f.media.is_empty()
            && cache_records
                .vk
                .get(&f.source_id.to_string())
                .map(|r| f.id > *r)
                .unwrap_or(true)
    });

    let telegram = TelegramClient::new(cfg.telegram_token, cfg.telegram_channel);

    if populate {
        for post in posts.rev() {
            new_cache_records
                .vk
                .entry(post.source_id.to_string())
                .and_modify(|k| {
                    if post.id > *k {
                        *k = post.id.unwrap_number();
                    }
                })
                .or_insert_with(|| post.id.unwrap_number());
        }
    } else {
        for post in posts.rev() {
            let res = {
                let res = telegram.send_message().by_foreign(&post).send().await;

                if let Err(TelegramError::RateLimited { timeout }) = res {
                    time::sleep(timeout).await;
                    telegram.send_message().by_foreign(&post).send().await
                } else {
                    res
                }
            };
            if let Err(e) = res {
                eprintln!("Failed to post to telegram: {e}");
            } else {
                new_cache_records
                    .vk
                    .entry(post.source_id.to_string())
                    .and_modify(|k| {
                        if post.id > *k {
                            *k = post.id.unwrap_number();
                        }
                    })
                    .or_insert_with(|| post.id.unwrap_number());
            }
        }
    }

    if let Ok(data) = serde_json::to_string(&new_cache_records) {
        if let Err(e) = fs::write(cache, data) {
            eprintln!("Failed to write to cache: {e}");
        }
    } else {
        eprintln!("Failed to serialize data to cache (why?..)");
    }

    ExitCode::SUCCESS
}
