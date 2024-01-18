use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const INDEX_VERSION: u8 = 1;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Index {
    pub schema_version: u8,
    // TODO: chrono datetime
    pub updated: String,
    /// Used only for cached versions to check whether the index should be updated
    pub retrieved: Option<String>,
    pub items: Vec<DictItem>,
}

impl Index {
    pub fn new() -> Self {
        Self {
            schema_version: INDEX_VERSION,
            updated: "abc".into(),
            retrieved: None,
            items: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DictItem {
    /// Language code of the dictionary
    pub lang: String,
    /// These may include the following
    ///
    /// - A `source-x` tag to identify where the dictionary came from (required)
    /// - A `size-{compact,medium,large}` reference
    ///
    /// These tags are used to determine when a dictionary is overwritten
    pub tags: Vec<String>,
    /// True if this dictionary meant to extend a base dictionary. E.g. jargon dictionaries.
    pub is_ext: bool,
    /// UUID for this entry
    pub id: Uuid,
    #[serde(flatten)]
    pub format: DictionaryFormat,
    pub lic: Downloadable,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "fmt")]
#[serde(rename_all = "lowercase")]
pub enum DictionaryFormat {
    /// The Hunspell dictionary format
    Hunspell {
        afx: Downloadable,
        dic: Downloadable,
    },
    /// A list of words with no special meanings. One word per line.
    Wordlist(Downloadable),
}

/// A file that can be downloaded
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Downloadable {
    pub urls: Vec<String>,
    pub hash: String,
    pub size: u64,
}
