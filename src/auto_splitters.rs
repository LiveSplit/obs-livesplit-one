use hyperx::header::{DispositionParam, Header};
use log::warn;
use quick_xml::{de, DeError};
use reqwest::header::{HeaderMap, CONTENT_DISPOSITION};
use serde::Deserialize;
use std::{
    fs,
    path::{Path, PathBuf},
    str,
};

const LIST_FILE_NAME: &str = "LiveSplit.AutoSplitters.xml";

pub struct ListManager {
    client: reqwest::blocking::Client,
    list: Result<List, (GetListFromGithubError, GetListFromFileError)>,
    list_xml_string: Option<String>,
}

pub enum GetListFromGithubError {
    NetError(reqwest::Error),
    DeserializationError(DeError),
}

pub enum GetListFromFileError {
    IoError(std::io::Error),
    DeserializationError(DeError),
}

#[derive(Deserialize, Clone)]
pub struct List {
    #[serde(rename = "AutoSplitter")]
    pub auto_splitters: Vec<AutoSplitter>,
}

#[derive(Deserialize, Clone)]
pub struct AutoSplitter {
    #[serde(rename = "Games")]
    pub games: Games,
    #[serde(rename = "URLs")]
    pub urls: Urls,
    #[serde(rename = "Type")]
    pub module_type: String,
    #[serde(rename = "ScriptType")]
    pub script_type: Option<String>,
    #[serde(rename = "Description")]
    pub description: String,
    #[serde(rename = "Website")]
    pub website: Option<String>,
}

#[derive(Deserialize, Clone)]
pub struct Games {
    #[serde(rename = "Game")]
    pub games: Vec<String>,
}

#[derive(Deserialize, Clone)]
pub struct Urls {
    #[serde(rename = "URL")]
    pub urls: Vec<String>,
}

impl ListManager {
    pub fn new(folder: &Path) -> Self {
        let client = reqwest::blocking::Client::new();

        let result = Self::get_list(&client, folder);

        let (list, list_xml_string) = match result {
            Ok((string, list)) => (Ok(list), Some(string)),
            Err(e) => (Err(e), None),
        };

        Self {
            client,
            list,
            list_xml_string,
        }
    }

    pub fn get_result(&self) -> Result<(), &(GetListFromGithubError, GetListFromFileError)> {
        match &self.list {
            Ok(_) => Ok(()),
            Err(err) => Err(err),
        }
    }

    pub fn save_list_to_disk(&self, folder: &Path) -> bool {
        if let Some(xml_string) = &self.list_xml_string {
            fs::write(folder.join(LIST_FILE_NAME), xml_string).is_ok()
        } else {
            false
        }
    }

    pub fn get_website_for_game(&self, game_name: &str) -> Option<&str> {
        self.get_for_game(game_name)?.website.as_deref()
    }

    pub fn get_for_game(&self, game_name: &str) -> Option<&AutoSplitter> {
        self.list
            .as_ref()
            .ok()?
            .auto_splitters
            .iter()
            .find(|x| x.games.games.iter().any(|g| g == game_name))
    }

    //todo
    pub fn download_for_game(&self, game_name: &str, folder: &Path) -> Option<PathBuf> {
        self.download(self.get_for_game(game_name)?, folder)
    }

    pub fn download(&self, auto_splitter: &AutoSplitter, folder: &Path) -> Option<PathBuf> {
        let mut file_paths = Vec::new();

        for url in &auto_splitter.urls.urls {
            let Ok(response) = self.client.get(url).send() else {
                continue;
            };

            let file_name = Self::get_requested_file_name(response.headers()).unwrap_or_else(|| {
                warn!("Couldn't get name for auto splitter file: {url}, defaulting to 'Unknown.wasm'");
                "Unknown.wasm".into()
            });

            if let Ok(bytes) = response.bytes() {
                let file_path = folder.join(&*file_name);

                match fs::write(&file_path, bytes) {
                    Ok(_) => {
                        file_paths.push(file_path);
                    }
                    Err(e) => {
                        warn!("Something went wrong when downloading and saving auto splitter file: {url}: {e}")
                    }
                }
            }
        }

        file_paths
            .into_iter()
            .find(|path| path.extension().is_some_and(|e| e == "wasm"))
    }

    pub fn is_using_auto_splitting_runtime(auto_splitter: &AutoSplitter) -> bool {
        auto_splitter
            .script_type
            .as_ref()
            .is_some_and(|t| t == "AutoSplittingRuntime")
    }

    fn get_list(
        client: &reqwest::blocking::Client,
        folder: &Path,
    ) -> Result<(String, List), (GetListFromGithubError, GetListFromFileError)> {
        let from_github_error = match Self::get_list_from_github(client) {
            Ok(auto_splitters) => return Ok(auto_splitters),
            Err(e) => e,
        };

        let from_file_error = match Self::get_list_from_file(folder) {
            Ok(auto_splitters) => return Ok(auto_splitters),
            Err(e) => e,
        };

        Err((from_github_error, from_file_error))
    }

    fn get_list_from_github(
        client: &reqwest::blocking::Client,
    ) -> Result<(String, List), GetListFromGithubError> {
        let url = "https://raw.githubusercontent.com/LiveSplit/LiveSplit.AutoSplitters/master/LiveSplit.AutoSplitters.xml";

        let body = client
            .get(url)
            .send()
            .map_err(GetListFromGithubError::NetError)?
            .text()
            .map_err(GetListFromGithubError::NetError)?;

        match de::from_str(&body) {
            Ok(auto_splitters) => Ok((body, auto_splitters)),
            Err(e) => Err(GetListFromGithubError::DeserializationError(e)),
        }
    }

    fn get_list_from_file(folder: &Path) -> Result<(String, List), GetListFromFileError> {
        let buffer = fs::read_to_string(folder.join(LIST_FILE_NAME))
            .map_err(GetListFromFileError::IoError)?;

        match de::from_str::<List>(&buffer) {
            Ok(auto_splitters) => Ok((buffer, auto_splitters)),
            Err(e) => Err(GetListFromFileError::DeserializationError(e)),
        }
    }

    // TODO: Make this return Result and improve support
    fn get_requested_file_name(header_map: &HeaderMap) -> Option<Box<str>> {
        hyperx::header::ContentDisposition::parse_header(&header_map.get(CONTENT_DISPOSITION)?)
            .ok()?
            .parameters
            .into_iter()
            .find_map(|param| {
                let DispositionParam::Filename(_, _, bytes) = param else {
                    return None;
                };
                Some(str::from_utf8(&bytes).ok()?.into())
            })
    }
}
