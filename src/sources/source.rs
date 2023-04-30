use crate::config::SnowflakeRef;

/// Foreign post
// ⚠️ BLAZINGLY FAST ⚠️
#[derive(Debug)]
pub struct ForeignPost<'a, T: std::fmt::Display> {
    /// Post ID. Should be unique
    pub id: SnowflakeRef<'a>,
    /// Post source ID.
    pub source_id: SnowflakeRef<'a>,

    /// Post text. May be empty
    pub text: &'a str,
    /// Post media.
    pub media: Vec<ForeignMedia<'a>>,

    /// Source name
    pub source: &'a str,
    /// Source url
    pub url: T,
}

/// Foreign media info
#[derive(Debug)]
pub enum ForeignMedia<'a> {
    /// A photo URL. JPEG, PNG, etc. NOT GIF
    Photo(&'a str),
    /// A video URL. MP4 or GIF only
    #[allow(dead_code)] // allowed for future
    Video(&'a str),
}
