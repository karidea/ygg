#![deny(warnings)]
#![warn(rust_2018_idioms)]

use clap::Parser;
use futures::prelude::*;
use reqwest::{header, Client, Method, StatusCode};
use semver::Version;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::env;
use std::fs;
use std::io::{self, BufRead, IsTerminal, Write};
use std::path::PathBuf;
use std::str;
use url::form_urlencoded;
use thiserror::Error;

#[derive(Deserialize, Serialize, Debug)]
struct Config {
    org: String,
}

#[derive(Deserialize, Debug)]
struct Packages {
    #[allow(unused)]
    version: Option<String>,
}

#[derive(Deserialize, Debug)]
struct PackageLockJson {
    #[allow(unused)]
    packages: Option<HashMap<String, Packages>>,
    #[allow(unused)]
    #[serde(rename = "lockfileVersion")]
    lockfile_version: Option<i32>,
    #[allow(unused)]
    dependencies: Option<HashMap<String, Packages>>,
}

#[derive(Deserialize)]
struct ApiResponse {
    items: Vec<Item>,
}

#[derive(Deserialize)]
struct Item {
    repository: Repository,
}

#[derive(Deserialize)]
struct Repository {
    full_name: String,
}

#[derive(Error, Debug)]
enum YggError {
    #[error("API error: {0}")]
    ApiError(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Reqwest error: {0}")]
    Reqwest(#[from] reqwest::Error),
    #[error("UTF-8 error: {0}")]
    Utf8(#[from] std::str::Utf8Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("TOML error: {0}")]
    Toml(#[from] toml::de::Error),
    #[error("TOML serialization error: {0}")]
    TomlSer(#[from] toml::ser::Error),
    #[error("Environment variable error: {0}")]
    Env(#[from] std::env::VarError),
    #[error("File not found")]
    NotFound,
    #[error("Unexpected status: {0}")]
    UnexpectedStatus(StatusCode),
}

type Result<T> = std::result::Result<T, YggError>;

#[derive(Clone)]
struct GitHubClient {
    client: Client,
    token: String,
}

impl GitHubClient {
    fn new() -> Result<Self> {
        let token = env::var("GHP_TOKEN")?;
        let client = Client::builder()
            .user_agent("ygg/0.1")
            .https_only(true)
            .build()?;
        Ok(Self { client, token })
    }

    async fn fetch_raw_file(&self, uri: &str, cache_manager: &CacheManager) -> Result<Vec<u8>> {
        cache_manager.get_or_fetch(uri, self).await
    }
}

#[derive(Clone)]
struct CacheManager {
    cache_dir: PathBuf,
}

impl CacheManager {
    fn new(cache_dir: PathBuf) -> Self {
        Self { cache_dir }
    }

    async fn get_or_fetch(&self, uri: &str, gh_client: &GitHubClient) -> Result<Vec<u8>> {
        let cache_key = uri.replace("https://api.github.com/repos/", "").replace("/", "_");
        let cache_path = self.cache_dir.join(&cache_key);
        let etag_path = self.cache_dir.join(format!("{cache_key}.etag"));
        let notfound_path = self.cache_dir.join(format!("{cache_key}.notfound"));

        if notfound_path.exists() {
            return Err(YggError::NotFound);
        }

        let mut etag: Option<String> = None;
        if cache_path.exists() && etag_path.exists() {
            if let Ok(cached_etag) = fs::read_to_string(&etag_path) {
                let etag_str = cached_etag.trim().to_string();
                if !etag_str.is_empty() {
                    etag = Some(etag_str);
                }
            }
        }

        let mut request_builder = gh_client.client.get(uri);
        request_builder = request_builder.header("Authorization", format!("token {}", gh_client.token));
        request_builder = request_builder.header("User-Agent", "ygg/0.1");
        request_builder = request_builder.header("Accept", "application/vnd.github.v3.raw");
        request_builder = request_builder.header("X-GitHub-Api-Version", "2022-11-28");

        if let Some(e) = etag {
            request_builder = request_builder.header("If-None-Match", e);
        }

        let res = request_builder.send().await?;

        let status = res.status();

        let body_bytes = if status == StatusCode::NOT_MODIFIED {
            // Use cached raw content
            fs::read(&cache_path)?
        } else if status.is_success() {
            // Get new body and etag
            let new_etag = res.headers().get("ETag").and_then(|v| v.to_str().ok()).map(|s| s.to_string());

            let bytes = res.bytes().await?.to_vec();

            // Update cache
            let _ = fs::remove_file(&notfound_path);
            let _ = fs::write(&cache_path, &bytes);
            if let Some(e) = new_etag {
                let _ = fs::write(&etag_path, e);
            }

            bytes
        } else if status == StatusCode::NOT_FOUND {
            let _ = fs::remove_file(&cache_path);
            let _ = fs::remove_file(&etag_path);
            let _ = fs::File::create(&notfound_path);
            return Err(YggError::NotFound);
        } else {
            return Err(YggError::UnexpectedStatus(status));
        };

        Ok(body_bytes)
    }
}

/// Ygg (Yggdrasil GitHub Grep): Grep GitHub repos to audit NPM lockfile versions or search custom strings in files
#[derive(Parser, Debug, Clone)]
#[clap(version, about, long_about = None)]
struct Cli {
    /// Path of the file containing json list of repositories (required unless --query is provided)
    #[clap(short, long, default_value = "repos.json")]
    repos: String,

    /// Search query for GitHub code search (if provided, searches for repos dynamically instead of using --repos)
    #[clap(short, long)]
    query: Option<String>,

    /// Organization name for code search (used with --query)
    #[clap(short, long)]
    org: Option<String>,

    /// Package name to check versions on (required for package-lock mode)
    #[clap(short, long)]
    package: Option<String>,

    /// Optional filename to fetch and search inside (if provided, performs string search instead of package-lock parsing)
    #[clap(short, long)]
    filename: Option<String>,

    /// Search string to find in the file content (required for string search mode)
    #[clap(short = 's', long)]
    search: Option<String>,

    /// Clear cache to force fetch from GitHub
    #[clap(short = 'c', long)]
    clear_cache: bool,
}

const PARALLEL_REQUESTS: usize = 100;
const BASE_SEARCH_URL: &str = "https://api.github.com/search/code";

async fn search_repos(gh_client: &GitHubClient, query: &str, org: &str) -> Result<Vec<String>> {
    let search_query = if org.is_empty() {
        query.to_string()
    } else {
        format!("org:{org} {query}")
    };

    // Build initial URL
    let mut current_url = format!(
        "{}?q={}&per_page=100",
        BASE_SEARCH_URL,
        form_urlencoded::byte_serialize(search_query.as_bytes()).collect::<String>()
    );

    // Collect unique repository full names
    let mut unique_repos: HashSet<String> = HashSet::new();

    loop {
        let mut req_builder = gh_client.client.request(Method::GET, &current_url);
        req_builder = req_builder.header("Authorization", format!("token {}", gh_client.token));
        req_builder = req_builder.header("User-Agent", "ygg/0.1");
        req_builder = req_builder.header("Accept", "application/vnd.github.v3+json");
        req_builder = req_builder.header("X-GitHub-Api-Version", "2022-11-28");

        let resp = req_builder.send().await?;

        if !resp.status().is_success() {
            return Err(YggError::ApiError(format!("API error: {}", resp.status())));
        }

        // Extract next URL from Link header before consuming the response
        let mut next_url: Option<String> = None;
        if let Some(link_header) = resp.headers().get(header::LINK) {
            if let Ok(link_str) = link_header.to_str() {
                let links: Vec<&str> = link_str.split(',').collect();
                for link in links.iter().map(|l| l.trim()) {
                    if link.contains(r#"rel="next""#) {
                        let start = link.find('<').map_or(0, |i| i + 1);
                        let end = link.find('>');
                        if let Some(e) = end {
                            next_url = Some(link[start..e].to_string());
                            break;
                        }
                    }
                }
            }
        }

        // Now consume the response to get the body
        let api_resp: ApiResponse = resp.json().await?;

        for item in api_resp.items {
            unique_repos.insert(item.repository.full_name);
        }

        if let Some(url) = next_url {
            current_url = url;
        } else {
            break;
        }
    }

    let mut repos_vec: Vec<String> = unique_repos.into_iter().collect();
    repos_vec.sort();
    Ok(repos_vec)
}

fn load_or_prompt_org() -> Result<String> {
    let config_path = PathBuf::from(".ygg.toml");

    // Try to load existing config
    if config_path.exists() {
        let config_str = fs::read_to_string(&config_path)?;
        let config: Config = toml::from_str(&config_str)?;
        return Ok(config.org);
    }

    // No config: Prompt if interactive (stdin is a terminal)
    let mut org = String::new();
    if std::io::stdin().is_terminal() {
        print!("Enter default GitHub organization (or leave empty to skip): ");
        io::stdout().flush().ok();
        let stdin = io::stdin();
        let mut line = String::new();
        if stdin.lock().read_line(&mut line).is_ok() {
            org = line.trim().to_string();
        }
    } else {
        eprintln!("Non-interactive mode: Skipping org prompt, using empty org.");
    }

    // Create and write config
    let config = Config { org: org.clone() };
    let toml_str = toml::to_string(&config)?;
    if let Err(e) = fs::write(&config_path, toml_str) {
        eprintln!("Warning: Failed to write {}: {}. Using in-memory org.", config_path.display(), e);
    } else {
        println!("Created {} with default org: '{}'", config_path.display(), org);
    }

    Ok(org)
}

fn process_package_lock(file_str: &str, query: &str) -> String {
    let not_found = String::from("-------");

    let package_lock_json: PackageLockJson = match serde_json::from_str(file_str) {
        Ok(json) => json,
        Err(e) => {
            eprintln!("Error parsing package-lock JSON: {e}");
            return not_found;
        }
    };

    if let Some(lockfile_version) = package_lock_json.lockfile_version {
        if lockfile_version == 1 {
            if let Some(dependencies) = &package_lock_json.dependencies {
                if let Some(package) = dependencies.get(query) {
                    if let Some(version) = &package.version {
                        return version.clone();
                    }
                }
            }
            return not_found;
        }
    }

    if let Some(packages) = &package_lock_json.packages {
        let node_modules_package_name = format!("node_modules/{query}");
        if let Some(package) = packages.get(&node_modules_package_name) {
            if let Some(version) = &package.version {
                return version.clone();
            }
        }
    }

    not_found
}

fn process_string_search(file_str: &str, query: &str) -> String {
    if file_str.contains(query) {
        "found".to_string()
    } else {
        "-------".to_string()
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let mut org = cli.org.clone().unwrap_or_default();
    if cli.org.is_none() {
        org = load_or_prompt_org()?;
    }

    let gh_client = GitHubClient::new()?;

    let mut json: Vec<String> = if let Some(search_query) = &cli.query {
        // Perform dynamic repo search if --query is provided
        let repos = search_repos(&gh_client, search_query, &org).await?;
        // Write the repos to repos.json, overwriting if exists
        let json_data = serde_json::to_string_pretty(&repos)?;
        fs::write("repos.json", json_data.as_bytes())?;
        repos
    } else {
        // Otherwise, read from --repos file (defaults to repos.json)
        let repos_path = &cli.repos;
        let data = fs::read_to_string(repos_path)?;
        serde_json::from_str(&data)?
    };

    // Sort the repos for consistent output
    json.sort();

    // Determine mode
    let is_package_lock = cli.filename.is_none();
    let is_valid_package_mode = is_package_lock && cli.package.is_some();
    let is_valid_search_mode = !is_package_lock && cli.search.is_some();

    if !is_valid_package_mode && !is_valid_search_mode {
        // No valid search/audit mode specified: List repos and exit
        for repo in json {
            println!("{repo}");
        }
        return Ok(());
    }

    // Proceed with file search/processing
    let query = if is_package_lock {
        cli.package.as_ref().unwrap().clone()
    } else {
        cli.search.as_ref().unwrap().clone()
    };

    let filename = cli.filename.clone().unwrap_or_else(|| "package-lock.json".to_string());

    let uris: Vec<_> = json.iter().map(|repo| {
        format!("https://api.github.com/repos/{repo}/contents/{filename}")
    }).collect();

    let cache_dir = PathBuf::from("./.cache");

    if cli.clear_cache {
        let _ = fs::remove_dir_all(&cache_dir);
    }

    fs::create_dir_all(&cache_dir)?;

    let cache_manager = CacheManager::new(cache_dir);

    let version_results = stream::iter(uris)
        .map(|uri| {
            let gh_client = gh_client.clone();
            let cache_manager = cache_manager.clone();
            async move {
                let body_bytes = gh_client.fetch_raw_file(&uri, &cache_manager).await?;
                Ok(body_bytes)
            }
        })
        .buffered(PARALLEL_REQUESTS)
        .map_ok(|body_bytes| {  // body_bytes is Vec<u8> (raw file content)
            let file_str = match str::from_utf8(&body_bytes) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("Error converting to UTF-8: {e}");
                    return "-------".to_string();
                }
            };

            if is_package_lock {
                process_package_lock(file_str, &query)
            } else {
                process_string_search(file_str, &query)
            }
        });

    let versions: Vec<Result<String>> = version_results.collect().await;

    let mut found_items: Vec<(String, String)> = versions.iter().enumerate()
        .filter_map(|(i, version): (usize, &Result<String>)| {
            match version {
                Ok(ver) if ver != "-------" => {
                    let repos: Vec<&str> = json[i].split('/').collect();
                    Some((ver.clone(), repos[1].to_string()))
                },
                _ => None,
            }
        })
        .collect();

    if is_package_lock {
        found_items.sort_by(|a, b| {
            let v1 = Version::parse(&a.0).unwrap_or(Version::parse("0.0.0").unwrap());
            let v2 = Version::parse(&b.0).unwrap_or(Version::parse("0.0.0").unwrap());
            v1.cmp(&v2)
        });

        for (version, repo) in found_items {
            println!("{version}\t: {repo}");
        }
    } else {
        found_items.sort_by(|a, b| a.1.cmp(&b.1));

        for (_, repo) in found_items {
            println!("{repo}");
        }
    }

    Ok(())
}
