use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct Config {
    pub vk_token: String,
    pub twitter_token: String,
    pub telegram_token: String,
    pub telegram_channel: Snowflake,

    pub twitter: Vec<SocialAccount>,
    pub vk: Vec<SocialAccount>,
}

#[derive(Deserialize)]
pub struct SocialAccount {
    pub id: Snowflake,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

#[derive(Serialize, Deserialize, Default, Clone)]
#[serde(rename_all = "lowercase")]
pub struct CacheRecords {
    pub vk: HashMap<String, u64>,
}

/// Represents an ID that [`u64`] or [`String`].
/// Owned variant of [`SnowflakeRef`].
#[derive(Debug, Deserialize, Clone)]
#[serde(untagged)]
pub enum Snowflake {
    Number(u64),
    String(String),
}
/// Represents an ID that [`u64`] or [`str`].
#[derive(Debug)]
pub enum SnowflakeRef<'a> {
    Number(u64),
    String(&'a str),
}

impl CacheRecords {
    pub fn new() -> Self {
        Default::default()
    }
}

impl<'a> SnowflakeRef<'a> {
    pub fn unwrap_number(&self) -> u64 {
        match self {
            Self::Number(v) => *v,
            _ => panic!("called [`SnowflakeRef::unwrap_number`] on str value"),
        }
    }

    pub fn flatten(self) -> Self {
        match self {
            Self::Number(v) => Self::Number(v),
            Self::String(s) => s.parse().map(Self::Number).unwrap_or(Self::String(s)),
        }
    }
}
impl Snowflake {
    pub fn as_ref(&self) -> SnowflakeRef {
        match self {
            Self::Number(v) => SnowflakeRef::Number(*v),
            Self::String(s) => SnowflakeRef::String(s.as_str()),
        }
    }
}
impl ToString for Snowflake {
    fn to_string(&self) -> String {
        match &self {
            Self::String(s) => s.clone(),
            Self::Number(v) => v.to_string(),
        }
    }
}
impl<'a> ToString for SnowflakeRef<'a> {
    fn to_string(&self) -> String {
        match self {
            Self::String(s) => s.to_string(),
            Self::Number(v) => v.to_string(),
        }
    }
}

impl<'a> PartialEq for SnowflakeRef<'a> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Number(a), Self::Number(b)) => a == b,
            (Self::String(a), Self::String(b)) => a == b,
            (Self::String(a), Self::Number(b)) | (Self::Number(b), Self::String(a)) => {
                a.parse().map(|r: u64| r == *b).unwrap_or_default()
            }
        }
    }
}
impl<'a> PartialEq<u64> for SnowflakeRef<'a> {
    fn eq(&self, other: &u64) -> bool {
        match self {
            Self::Number(v) => v == other,
            Self::String(s) => s.parse().ok().map(|v: u64| v == *other).unwrap_or_default(),
        }
    }
}
impl<'a> Eq for SnowflakeRef<'a> {}

impl<'a> PartialOrd for SnowflakeRef<'a> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match (self, other) {
            (Self::Number(a), Self::Number(b)) => Some(a.cmp(b)),
            (Self::String(_), Self::String(_)) => None,
            (Self::String(a), Self::Number(b)) | (Self::Number(b), Self::String(a)) => a
                .parse()
                .map(|ref r: u64| Some(r.cmp(b)))
                .unwrap_or_default(),
        }
    }
}
impl<'a> PartialOrd<u64> for SnowflakeRef<'a> {
    fn partial_cmp(&self, other: &u64) -> Option<std::cmp::Ordering> {
        match self {
            Self::Number(v) => Some(v.cmp(other)),
            Self::String(s) => s
                .parse()
                .ok()
                .map(|ref v: u64| Some(v.cmp(other)))
                .unwrap_or_default(),
        }
    }
}
