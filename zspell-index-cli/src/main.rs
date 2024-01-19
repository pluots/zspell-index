//! The main source for index entries is <https://github.com/wooorm/dictionaries>. This tool
//! automatically updates our index based on its contents.

use anyhow::{bail, Context};
use serde::Deserialize;
use serde_json::Value;
use std::{env, fs, path::Path, time::Duration};
use zspell_index::{DictionaryFormat, Downloadable, Index, IndexEntry};

const WOOORM_API_URL: &str = "https://api.github.com/repos/wooorm/dictionaries";
const WOOORM_BRANCH_NAME: &str = "main";
const WOOORM_TAG: &str = "source-wooorm";
const APP_USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"),);
const OUTPUT_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/..");
const FILE_NAME: &str = "zspell-index.json";
const FILE_NAME_PRETTY: &str = "zspell-index-pretty.json";

/// Contents of a directory
#[derive(Debug, Deserialize)]
struct Tree(Vec<Listing>);

// FIXME: use permalinks
/// A single subdirectory or file within a [`Tree`]
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct Listing {
    name: Box<str>,
    path: Box<str>,
    size: usize,
    sha: Box<str>,
    url: Box<str>,
    html_url: Box<str>,
    git_url: Box<str>,
    #[serde(flatten)]
    contents: ListingContents,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
#[serde(rename_all = "lowercase")]
enum ListingContents {
    Dir,
    File { download_url: Box<str> },
}

fn make_client() -> ureq::Agent {
    #[allow(clippy::result_large_err)]
    fn add_headers(
        req: ureq::Request,
        next: ureq::MiddlewareNext,
    ) -> Result<ureq::Response, ureq::Error> {
        let req = req.set("Accept", "application/vnd.github+json");
        let with_header = if let Ok(var) = env::var("GITHUB_API_TOKEN") {
            req.set("Authorization", &format!("Bearer {var}"))
        } else {
            eprintln!("tip: set the GITHUB_API_TOKEN environment variable to avoid rate limiting");
            req
        };

        next.handle(with_header)
    }

    ureq::builder()
        .timeout(Duration::from_secs(10))
        .user_agent(APP_USER_AGENT)
        .middleware(add_headers)
        .build()
}

fn make_downloadable(listing: &Listing) -> anyhow::Result<Downloadable> {
    let ListingContents::File { ref download_url } = listing.contents else {
        bail!("expected a file but got a directory");
    };

    let ret = Downloadable {
        urls: Box::new([download_url.clone()]),
        // Github uses sha1 for the hash
        hash: format!("sha1:{}", listing.sha).into(),
        size: listing.size.try_into().unwrap(),
    };

    Ok(ret)
}

fn update_inner(
    lang: &str,
    dir_url: &str,
    agent: &ureq::Agent,
) -> anyhow::Result<Option<IndexEntry>> {
    let dir_tree: Tree = agent
        .get(dir_url)
        .call()
        .context("requesting directory listing")?
        .into_json()?;

    let Some(afx_entry) = dir_tree.0.iter().find(|l| l.name.ends_with(".aff")) else {
        eprintln!("skipping {lang}: no affix file");
        return Ok(None);
    };
    let Some(dic_entry) = dir_tree.0.iter().find(|l| l.name.ends_with(".dic")) else {
        eprintln!("skipping {lang}: no dictionary file");
        return Ok(None);
    };
    let Some(lic_entry) = dir_tree.0.iter().find(|l| l.name.ends_with("license")) else {
        eprintln!("skipping {lang}: no license file");
        return Ok(None);
    };

    let ret = IndexEntry {
        lang: lang.into(),
        tags: Box::new([WOOORM_TAG.into()]),
        is_ext: false,
        id: uuid::Uuid::now_v7(),
        format: DictionaryFormat::Hunspell {
            aff: make_downloadable(afx_entry)?,
            dic: make_downloadable(dic_entry)?,
        },
        lic: make_downloadable(lic_entry)?,
    };
    Ok(Some(ret))
}

fn get_latest_hash(agent: &ureq::Agent) -> anyhow::Result<Box<str>> {
    let resp: Value = agent
        .get(&format!(
            "{WOOORM_API_URL}/commits/{WOOORM_BRANCH_NAME}?per_page=1"
        ))
        .call()
        .context("requesting latest git hash")?
        .into_json()?;

    let Value::Object(mut map) = resp else {
        bail!("invalid response");
    };

    let Some(Value::String(sha)) = map.remove("sha") else {
        bail!("response is missing sha");
    };

    eprintln!("using git hash {sha}");
    Ok(sha.into())
}

fn write_to_file(
    index: &Index,
    output_path: &Path,
    output_path_pretty: &Path,
) -> anyhow::Result<()> {
    let ser = serde_json::to_string(index)?;
    let ser_pretty = serde_json::to_string_pretty(index)?;

    eprintln!("writing output to {}", output_path.display());
    fs::write(output_path, ser)?;
    eprintln!("writing pretty output to {}", output_path_pretty.display());
    fs::write(output_path_pretty, ser_pretty)?;
    Ok(())
}

fn update_from_wooorm() -> anyhow::Result<()> {
    let agent = make_client();
    let git_ref = get_latest_hash(&agent)?;
    let all_langs: Tree = agent
        .get(&format!(
            "{WOOORM_API_URL}/contents/dictionaries?ref={git_ref}"
        ))
        .call()
        .context("requesting root listing")?
        .into_json()?;

    let mut items = Vec::new();
    let mut has_errors = false;

    for dir in all_langs.0.iter() {
        let lang = &dir.name;
        let ListingContents::Dir = dir.contents else {
            continue;
        };

        eprintln!("locating dictionary {lang}");

        match update_inner(lang, &dir.url, &agent) {
            Ok(Some(item)) => items.push(item),
            Ok(None) => continue,
            Err(e) => {
                eprintln!("error with {lang}: {e}. skipping");
                has_errors = true;
                continue;
            }
        }
    }

    let mut index = Index::new();
    index.items = items.into_boxed_slice();

    let mut output_path = Path::new(OUTPUT_DIR).join(FILE_NAME);
    let mut output_path_pretty = Path::new(OUTPUT_DIR).join(FILE_NAME_PRETTY);

    if has_errors {
        eprintln!("errors encountered during update. writing incomplete files.");
        output_path.set_extension("incomplete.json");
        output_path_pretty.set_extension("incomplete.json");
    }

    write_to_file(&index, &output_path, &output_path_pretty)?;

    Ok(())
}

fn main() -> anyhow::Result<()> {
    update_from_wooorm()?;
    Ok(())
}
