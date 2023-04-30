use std::{
    fmt::{Display, Write},
    time::Duration,
};

use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::{
    config::Snowflake,
    sources::{ForeignMedia, ForeignPost},
};

pub struct TelegramClient {
    pub token: String,
    pub channel_id: Snowflake,
}
#[derive(Debug)]
pub enum TelegramError {
    Http(reqwest::Error),
    Scheme(serde_json::Error),
    Server {
        error_code: u32,
        description: String,
    },
    RateLimited {
        timeout: Duration,
    },
}

impl Display for TelegramError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Http(e) => e.fmt(f),
            Self::Scheme(e) => e.fmt(f),
            Self::Server {
                error_code,
                description,
            } => write!(f, "API returned error {error_code}: {description}"),
            Self::RateLimited { timeout } => {
                write!(f, "ratelimited for {} seconds", timeout.as_secs())
            }
        }
    }
}
impl std::error::Error for TelegramError {}

pub struct SendMessage<'a, 'b> {
    token: &'a str,
    channel_id: &'a Snowflake,
    text: String,
    media: Vec<TelegramMedia<'b>>,
}
#[derive(Serialize)]
pub struct TelegramMedia<'a> {
    pub r#type: TelegramMediaType,
    pub media: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub caption: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parse_mode: Option<String>,
}
#[derive(Serialize)]
#[serde(rename_all = "lowercase")]
pub enum TelegramMediaType {
    Photo,
    Video,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum TelegramResponse<T> {
    Ok {
        result: T,
    },
    Err {
        error_code: u32,
        description: String,
        #[serde(default)]
        parameters: Option<TelegramRateLimitError>,
    },
}
#[derive(Deserialize)]
struct TelegramRateLimitError {
    retry_after: u64,
}

#[derive(Deserialize)]
#[non_exhaustive]
struct TelegramMessage {
    message_id: u64,
}

pub struct ProtectedString<'a>(pub &'a str);

impl<'a> Display for ProtectedString<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for c in self.0.chars() {
            match c {
                '&' => f.write_str("&amp;")?,
                '>' => f.write_str("&gt;")?,
                '<' => f.write_str("&lt;")?,
                _ => f.write_char(c)?,
            }
        }

        Ok(())
    }
}

impl TelegramClient {
    pub fn new(token: String, channel_id: Snowflake) -> Self {
        Self { token, channel_id }
    }

    pub fn send_message<'b>(&self) -> SendMessage<'_, 'b> {
        SendMessage {
            token: &self.token,
            channel_id: &self.channel_id,
            text: String::new(),
            media: Vec::new(),
        }
    }
}
impl<'a, 'b> SendMessage<'a, 'b> {
    pub fn by_foreign<T: Display>(mut self, foreign: &ForeignPost<'b, T>) -> Self {
        self.text = format!(
            "{}\n\nsrc: <a href=\"{}\">{}</a>",
            ProtectedString(foreign.text),
            foreign.url,
            ProtectedString(foreign.source)
        );
        self.media = foreign
            .media
            .iter()
            .map(|f| match f {
                ForeignMedia::Photo(media) => TelegramMedia {
                    r#type: TelegramMediaType::Photo,
                    media,
                    caption: None,
                    parse_mode: None,
                },
                ForeignMedia::Video(media) => TelegramMedia {
                    r#type: TelegramMediaType::Video,
                    media,
                    caption: None,
                    parse_mode: None,
                },
            })
            .collect();

        self
    }

    pub async fn send(mut self) -> Result<u64, TelegramError> {
        if self.media.is_empty() {
            unimplemented!("Sending text-only messages is not supported");
        }

        if let Some(TelegramMedia {
            caption,
            parse_mode,
            ..
        }) = self.media.get_mut(0)
        {
            *caption = Some(self.text);
            *parse_mode = Some("HTML".to_owned());
        }

        let client = Client::new();
        let res = client
            .post(format!(
                "https://api.telegram.org/bot{}/sendMediaGroup",
                self.token
            ))
            .query(&[
                ("chat_id", self.channel_id.to_string()),
                (
                    "media",
                    serde_json::to_string(&self.media).map_err(TelegramError::Scheme)?,
                ),
            ])
            .send()
            .await
            .map_err(TelegramError::Http)?
            .text()
            .await
            .map_err(TelegramError::Http)?;

        let res: TelegramResponse<Vec<TelegramMessage>> =
            serde_json::from_str(&res).map_err(TelegramError::Scheme)?;

        match res {
            TelegramResponse::Ok { result } => Ok(result[0].message_id),

            TelegramResponse::Err {
                error_code: 429,
                parameters: Some(TelegramRateLimitError { retry_after }),
                ..
            } => Err(TelegramError::RateLimited {
                timeout: Duration::from_secs(retry_after),
            }),

            TelegramResponse::Err {
                error_code,
                description,
                ..
            } => Err(TelegramError::Server {
                error_code,
                description,
            }),
        }
    }
}
