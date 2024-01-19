//! Data structures that represent a ZSpell index.

#![allow(clippy::new_without_default)]

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// The version of the index serialization format
pub const INDEX_VERSION: u8 = 1;

/// The main index entrypoint
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Index {
    pub schema_version: u8,
    // TODO: chrono datetime
    pub updated: DateTime<Utc>,
    /// Used only for cached versions to check whether the index should be updated
    pub retrieved: Option<Box<str>>,
    pub items: Box<[IndexEntry]>,
}

impl Index {
    pub fn new() -> Self {
        Self {
            schema_version: INDEX_VERSION,
            updated: Utc::now(),
            retrieved: None,
            items: Box::new([]),
        }
    }
}

/// A single item within an index
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct IndexEntry {
    /// Language code of the dictionary
    pub lang: Box<str>,
    /// These may include the following
    ///
    /// - A `source-x` tag to identify where the dictionary came from (required)
    /// - A `size-{compact,medium,large}` reference
    ///
    /// These tags are used to determine when a dictionary is overwritten
    pub tags: Box<[Box<str>]>,
    /// True if this dictionary meant to extend a base dictionary. E.g. jargon dictionaries.
    pub is_ext: bool,
    /// UUID for this entry
    pub id: Uuid,
    /// The dictionary source files, which differ based on format
    #[serde(flatten)]
    pub format: DictionaryFormat,
    /// The license
    pub lic: Downloadable,
}

/// Dictionary source files
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "fmt")]
#[serde(rename_all = "lowercase")]
pub enum DictionaryFormat {
    /// The Hunspell dictionary format
    Hunspell {
        aff: Downloadable,
        dic: Downloadable,
    },
    /// A list of words with no special meanings. One word per line.
    Wordlist(Downloadable),
}

/// A file that can be downloaded
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Downloadable {
    /// A list of URLs that can be used, in order of precedence
    pub urls: Box<[Box<str>]>,
    /// A hash of the file. This should be in the form `sha256:1234abcd...`
    pub hash: Box<str>,
    /// The size of the file, in bytes
    pub size: u64,
}
