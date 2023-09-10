use anyhow::{format_err, Context, Error, Result};
use livesplit_core::util::PopulateString;
use log::{error, info};
use quick_xml::de;
use reqwest::{blocking::Client, Url};
use serde_derive::Deserialize;
use std::{
    ffi::CStr,
    fs,
    path::{Path, PathBuf},
    ptr, str,
    sync::{
        atomic::{self, AtomicPtr},
        OnceLock,
    },
};

use crate::{ffi::obs_module_get_config_path, ffi_types::obs_module_t};

const LIST_FILE_NAME: &str = "LiveSplit.AutoSplitters.xml";

static OBS_MODULE_POINTER: AtomicPtr<obs_module_t> = AtomicPtr::new(ptr::null_mut());

pub fn get_module_config_path() -> &'static PathBuf {
    static OBS_MODULE_CONFIG_PATH: OnceLock<PathBuf> = OnceLock::new();

    OBS_MODULE_CONFIG_PATH.get_or_init(|| {
        let mut buffer = PathBuf::new();

        unsafe {
            let config_path_ptr = obs_module_get_config_path(
                OBS_MODULE_POINTER.load(atomic::Ordering::Relaxed),
                cstr!(""),
            );
            if let Ok(config_path) = CStr::from_ptr(config_path_ptr).to_str() {
                buffer.push(config_path);
            }
        }

        buffer
    })
}

#[no_mangle]
pub extern "C" fn obs_module_set_pointer(module: *mut obs_module_t) {
    OBS_MODULE_POINTER.store(module, atomic::Ordering::Relaxed);
}

pub static LIST: OnceLock<List> = OnceLock::new();

pub fn get_list() -> &'static List {
    static EMPTY: List = List::empty();

    LIST.get().unwrap_or(&EMPTY)
}

pub fn get_downloader() -> &'static Downloader {
    static DOWNLOADER: OnceLock<Downloader> = OnceLock::new();

    DOWNLOADER.get_or_init(Downloader::new)
}

pub fn get_path() -> &'static PathBuf {
    static PATH: OnceLock<PathBuf> = OnceLock::new();

    PATH.get_or_init(|| get_module_config_path().join("auto-splitters"))
}

pub struct Downloader {
    client: Client,
}

pub struct List {
    inner: ListInner,
    source: String,
}

#[derive(Deserialize)]
struct ListInner {
    #[serde(rename = "AutoSplitter")]
    auto_splitters: Vec<AutoSplitter>,
}

#[derive(Deserialize)]
pub struct AutoSplitter {
    #[serde(rename = "Games")]
    games: Games,
    #[serde(rename = "URLs")]
    urls: Urls,
    // #[serde(rename = "Type")]
    // module_type: String,
    #[serde(rename = "ScriptType")]
    script_type: Option<String>,
    #[serde(rename = "Description")]
    pub description: String,
    #[serde(rename = "Website")]
    pub website: Option<String>,
}

#[derive(Deserialize)]
struct Games {
    #[serde(rename = "Game")]
    games: Vec<String>,
}

#[derive(Deserialize)]
struct Urls {
    #[serde(rename = "URL")]
    urls: Vec<String>,
}

impl List {
    pub const fn empty() -> Self {
        Self {
            inner: ListInner {
                auto_splitters: Vec::new(),
            },
            source: String::new(),
        }
    }

    pub fn save_to_disk(&self, folder: &Path) -> Result<()> {
        fs::write(folder.join(LIST_FILE_NAME), &self.source).map_err(Into::into)
    }

    pub fn get_website_for_game(&self, game_name: &str) -> Option<&str> {
        self.get_for_game(game_name)?.website.as_deref()
    }

    pub fn get_for_game(&self, game_name: &str) -> Option<&AutoSplitter> {
        self.inner
            .auto_splitters
            .iter()
            .find(|x| x.games.games.iter().any(|g| g == game_name))
    }
}

impl Downloader {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }

    pub fn download_list(&self, folder: &Path) -> Result<List, [Error; 2]> {
        let from_github_error = match get_list_from_github(&self.client) {
            Ok(list) => return Ok(list),
            Err(e) => e,
        };

        let from_file_error = match get_list_from_file(folder) {
            Ok(list) => return Ok(list),
            Err(e) => e,
        };

        Err([from_github_error, from_file_error])
    }

    pub fn download_for_game(
        &self,
        list: &List,
        game_name: &str,
        folder: &Path,
    ) -> Option<PathBuf> {
        self.download(list.get_for_game(game_name)?, folder)
    }

    pub fn download(&self, auto_splitter: &AutoSplitter, folder: &Path) -> Option<PathBuf> {
        let mut file_paths = Vec::new();

        for url in &auto_splitter.urls.urls {
            if let Err(e) = self
                .download_file(url, folder, &mut file_paths)
                .with_context(|| format_err!("Failed downloading `{url}`."))
            {
                error!("{e:#?}");
            }
        }

        file_paths
            .into_iter()
            .find(|path| path.extension().is_some_and(|e| e == "wasm"))
    }

    fn download_file(&self, url: &str, folder: &Path, file_paths: &mut Vec<PathBuf>) -> Result<()> {
        let url = Url::parse(url).context("Failed parsing the URL.")?;

        let file_name = url
            .path_segments()
            .and_then(|s| s.last())
            .context("There is no file name in the URL.")?;

        let file_name = percent_encoding::percent_decode_str(file_name).decode_utf8_lossy();
        let file_path = folder.join(file_name.as_str());

        let bytes = self
            .client
            .get(url)
            .send()
            .context("Failed sending the request.")?
            .error_for_status()
            .context("The response is unsuccessful.")?
            .bytes()
            .context("Failed receiving the response.")?;

        fs::write(&file_path, bytes).context("Failed writing the file.")?;

        file_paths.push(file_path);

        Ok(())
    }
}

impl AutoSplitter {
    pub fn is_using_auto_splitting_runtime(&self) -> bool {
        self.script_type
            .as_ref()
            .is_some_and(|t| t == "AutoSplittingRuntime")
    }
}

fn get_list_from_github(client: &Client) -> Result<List> {
    let url = "https://raw.githubusercontent.com/LiveSplit/LiveSplit.AutoSplitters/master/LiveSplit.AutoSplitters.xml";

    let source = client
        .get(url)
        .send()
        .context("Failed sending the request.")?
        .error_for_status()
        .context("The response was unsuccessful.")?
        .text()
        .context("Failed receiving the body as text.")?;

    Ok(List {
        inner: de::from_str(&source).context("Failed parsing the list.")?,
        source,
    })
}

fn get_list_from_file(folder: &Path) -> Result<List> {
    let source =
        fs::read_to_string(folder.join(LIST_FILE_NAME)).context("Failed reading the file.")?;

    Ok(List {
        inner: de::from_str(&source).context("Failed parsing the list.")?,
        source,
    })
}

pub fn set_up() {
    let auto_splitters_path = get_path();

    if let Err(e) = fs::create_dir_all(auto_splitters_path)
        .context("Failed creating the auto splitters folder.")
    {
        error!("{:?}", e);
    }

    match get_downloader().download_list(get_module_config_path()) {
        Ok(list) => {
            if let Err(e) = list
                .save_to_disk(auto_splitters_path)
                .context("Failed saving the list of auto splitters.")
            {
                error!("{:?}", e);
            }

            let _ = LIST.set(list);
            info!("Auto splitter list loaded.");
        }
        Err([from_github, from_file]) => {
            error!(
                "{:?}",
                from_github.context("Failed downloading the list of auto splitters.")
            );
            error!(
                "{:?}",
                from_file.context("Failed loading the list of auto splitters from the cache.")
            );
        }
    }
}
