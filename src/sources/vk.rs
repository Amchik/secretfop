use reqwest::Client;
use serde::Deserialize;

use crate::config::SnowflakeRef;

use super::{ForeignMedia, ForeignPost};

pub struct VKClient {
    pub token: String,
}
pub struct VKGetPosts<'a> {
    limit: u8,
    id: SnowflakeRef<'a>,
    token: &'a str,
}
//pub struct VKFetchVideos<'client, 'data> {
//    token: &'client str,
//    videos: Vec<&'data mut String>,
//}
#[derive(Debug)]
pub enum VKError {
    Http(reqwest::Error),
    Scheme(serde_json::Error),
    Server { error_code: u32, error_msg: String },
    Content,
}

impl std::fmt::Display for VKError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Http(v) => v.fmt(f),
            Self::Scheme(v) => v.fmt(f),
            Self::Server {
                error_code,
                error_msg,
            } => write!(f, "API returned error {error_code}: {error_msg}"),
            Self::Content => write!(f, "API does not returned any groups"),
        }
    }
}
impl std::error::Error for VKError {}

pub struct VKGroupFeed {
    pub group_source_name: String,
    pub group_id: u64,
    pub items: Vec<VKItem>,
}
pub struct VKItem {
    pub id: u64,
    pub text: String,
    pub media: Vec<VKMedia>,
}
pub enum VKMedia {
    /// Photo, contains url to image.
    Photo(String),
}

//#[deprecated = "Please do not use this iter because it so cringe"]
//pub struct VKGroupFeedIter<'a> {
//    feed: &'a VKGroupFeed,
//    iter: std::slice::Iter<'a, VKItem>,
//}
#[derive(Debug)]
pub struct VKItemURL {
    group_id: u64,
    item_id: u64,
}

#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
enum VKResponse {
    Response {
        items: Vec<VKResponseItem>,
        groups: Vec<VKGroup>,
    },
    Error {
        error_code: u32,
        error_msg: String,
    },
}
#[derive(Deserialize)]
struct VKResponseItem {
    id: u64,
    marked_as_ads: i32,
    text: String,
    attachments: Vec<VKResponseMedia>,
}
#[derive(Deserialize)]
struct VKGroup {
    id: u64,
    name: String,
}
#[derive(Deserialize)]
struct VKResponseMedia {
    #[serde(default)]
    photo: Option<VKResponsePhoto>,
}
#[derive(Deserialize)]
struct VKResponsePhoto {
    sizes: Vec<VKPhotoSizes>,
}
#[derive(Deserialize)]
struct VKPhotoSizes {
    r#type: char,
    url: String,
}

impl VKClient {
    pub fn new(token: String) -> Self {
        Self { token }
    }

    //    pub fn fetch_videos<'a>(&self) -> VKFetchVideos<'_, 'a> {
    //        VKFetchVideos {
    //            token: &self.token,
    //            videos: Vec::new(),
    //        }
    //    }

    pub fn get_posts<'a>(&'a self, id: SnowflakeRef<'a>) -> VKGetPosts<'a> {
        VKGetPosts {
            limit: 5,
            id,
            token: &self.token,
        }
    }
}
impl VKGroupFeed {
    pub fn as_iter(&self) -> impl DoubleEndedIterator<Item = ForeignPost<'_, VKItemURL>> {
        self.items.iter().map(|item: &VKItem| ForeignPost {
            id: SnowflakeRef::Number(item.id),
            source_id: SnowflakeRef::Number(self.group_id),
            text: &item.text,
            media: item
                .media
                .iter()
                .map(|f| match f {
                    VKMedia::Photo(v) => ForeignMedia::Photo(v),
                })
                .collect(),
            source: &self.group_source_name,
            url: VKItemURL {
                group_id: self.group_id,
                item_id: item.id,
            },
        })
    }
}

//impl<'client, 'data> VKFetchVideos<'client, 'data> {
//    pub fn with_items<'a: 'data>(mut self, items: &'a mut [VKItem]) -> Self {
//        let videos = items
//            .iter_mut()
//            .map(|f| &mut f.media)
//            .flatten()
//            .filter_map(|f| match f {
//                VKMedia::VideoData(s) => Some(s),
//                _ => None,
//            });
//        for video in videos {
//            self.videos.push(video);
//        }
//
//        self
//    }
//
//    pub async fn send(&self) -> Result<(), VKError> {
//        let client = Client::new();
//        let videos = self.videos.iter().map(|f| f.as_str()).join(",");
//        let res = client.get("https://api.vk.com/method/video.get")
//            .bearer_auth(self.token)
//            .query(&["v", "5.131", "videos", &videos])
//            .send()
//            .await
//            .map_err(VKError::Http)?
//            .text()
//            .await
//            .map_err(VKError::Http)?;
//
//
//        Ok(())
//    }
//}
impl<'a> VKGetPosts<'a> {
    pub async fn send(self) -> Result<VKGroupFeed, VKError> {
        let client = Client::new();
        let id = match self.id.flatten() {
            SnowflakeRef::Number(v) => ("owner_id", format!("-{v}")), // 140 IQ negative ids
            SnowflakeRef::String(s) => ("domain", s.to_owned()),
        };
        let res = client
            .get("https://api.vk.com/method/wall.get")
            .bearer_auth(self.token)
            .query(&[
                ("count", self.limit.to_string()),
                id,
                ("extended", "1".to_string()),
                ("v", "5.131".to_string()),
            ])
            .send()
            .await
            .map_err(VKError::Http)?
            .text()
            .await
            .map_err(VKError::Http)?;

        let raw: VKResponse = serde_json::from_str(&res).map_err(VKError::Scheme)?;

        let (items, group_id, source) = match raw {
            VKResponse::Response { items, groups } if !groups.is_empty() => {
                (items, groups[0].id, format!("vk // {}", groups[0].name))
            }
            VKResponse::Error {
                error_code,
                error_msg,
            } => {
                return Err(VKError::Server {
                    error_code,
                    error_msg,
                })
            }
            _ => return Err(VKError::Content),
        };

        let feed = VKGroupFeed {
            group_source_name: source,
            group_id,
            items: items
                .into_iter()
                .filter(|i| i.marked_as_ads == 0)
                .map(|i| VKItem {
                    id: i.id,
                    text: i.text,
                    media: i
                        .attachments
                        .into_iter()
                        .filter_map(|r| r.photo)
                        .map(|r| {
                            VKMedia::Photo(
                                r.sizes
                                    .into_iter()
                                    .rev() // better first (maybe)
                                    .max_by_key(|p| match p.r#type {
                                        's' => 1,
                                        'm' => 2,
                                        'x' => 3,
                                        'y' => 4,
                                        'z' => 5,
                                        'w' => 6,
                                        _ => 0,
                                    })
                                    .map(|f| f.url)
                                    .expect("api should return at least one size for media"),
                            )
                        })
                        .collect(),
                })
                .collect(),
        };

        Ok(feed)
    }
}

//impl<'a> std::iter::Iterator for VKGroupFeedIter<'a> {
//    type Item = ForeignPost<'a, VKItemURL>;
//
//    fn next(&mut self) -> Option<Self::Item> {
//        let Some(item) = self.iter.next() else {
//            return None;
//        };
//
//        let item = ForeignPost {
//            id: SnowflakeRef::Number(item.id),
//            source_id: SnowflakeRef::Number(self.feed.group_id),
//            text: &item.text,
//            media: item
//                .media
//                .iter()
//                .map(|f| match f {
//                    VKMedia::Photo(v) => ForeignMedia::Photo(v),
//                })
//                .collect(),
//            source: &self.feed.group_source_name,
//            url: VKItemURL {
//                group_id: self.feed.group_id,
//                item_id: item.id,
//            },
//        };
//
//        Some(item)
//    }
//}
impl std::fmt::Display for VKItemURL {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "https://vk.com/wall-{}_{}", self.group_id, self.item_id)
    }
}
