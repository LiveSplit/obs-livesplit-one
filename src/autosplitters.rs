use std::{fs::File, io::{Write, Read}, path::{PathBuf, Path}, str::from_utf8};
use quick_xml::{de::from_str, DeError};
use serde::Deserialize;
use reqwest::{blocking::get, header::{CONTENT_DISPOSITION, HeaderMap}};
use hyperx::header::{DispositionParam, Header};

const AUTO_SPLITTER_LIST_FILE_NAME: &str = "LiveSplit.AutoSplitters.xml";

pub struct AutoSplitterListManager {
    list: Result<AutoSplitterList, (GetAutoSplitterListFromGithubError, GetAutoSplitterListFromFileError)>,
    list_xml_string: Option<String>
}

pub enum GetAutoSplitterListFromGithubError {
    NetError(reqwest::Error),
    DeserializationError(DeError)
}

pub enum GetAutoSplitterListFromFileError {
    IoError(std::io::Error),
    DeserializationError(DeError)
}

#[derive(Deserialize, Clone)]
pub struct AutoSplitterList {
    #[serde(rename = "AutoSplitter")]
    pub auto_splitters: Vec<AutoSplitter>
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
    pub website: Option<String>
}

#[derive(Deserialize, Clone)]
pub struct Games {
    #[serde(rename = "Game")]
    pub games: Vec<String>
}

#[derive(Deserialize, Clone)]
pub struct Urls {
    #[serde(rename = "URL")]
    pub urls: Vec<String>
}

impl AutoSplitterListManager {
    pub fn new() -> Self {
        let result = Self::get_auto_splitter_list();
        
        let (list, list_xml_string) = match result {
            Ok(value) => { (Ok(value.1), Some(value.0)) }
            Err(e) => { (Err(e), None) }
        };
        
        Self {
            list,
            list_xml_string
        }
    }
    
    pub fn is_ok(&self) -> Result<(), &(GetAutoSplitterListFromGithubError, GetAutoSplitterListFromFileError)> {
        match &self.list {
            Ok(_) => { Ok(()) }
            Err(err) => { Err(err) }
        }
    } 

    pub fn save_auto_splitter_list_to_disk(&self) -> bool {
        let file = File::create(Path::join(&*super::OBS_MODULE_CONFIG_PATH, AUTO_SPLITTER_LIST_FILE_NAME));

        if let (Some(xml_string), Ok(mut file)) = (self.list_xml_string.clone(), file) {
            return match file.write_all(xml_string.as_bytes()) {
                Ok(_) => { true }
                Err(_) => { false }
            };
        };

        false
    }
    
    pub fn get_auto_splitter_website_for_game(&self, game_name: String) -> Option<String> {
        match &self.list {
            Ok(result) => { 
                match result.auto_splitters.iter().find(
                    |&x| x.games.games.contains(&game_name)) {
                    Some(auto_splitter) => { auto_splitter.website.clone() }
                    None => { None }
                }
            }
            Err(_) => { None }
        }
    }

    pub fn get_auto_splitter_for_game(&self, game_name: String) -> Option<&AutoSplitter> {
        match &self.list {
            Ok(result) => {
                return result.auto_splitters.iter().find(|&x| x.games.games.contains(&game_name))
            }
            Err(_) => { None }
        }
    }
    
    //todo
    pub fn download_auto_splitter_for_game(&self, game_name: String) -> Option<PathBuf> {
        match &self.list {
            Ok(result) => {
                return match result.auto_splitters.iter().find(|&x| x.games.games.contains(&game_name)) {
                    Some(auto_splitter) => { Self::download_auto_splitter(auto_splitter) }
                    None => { None }
                }
            }
            Err(_) => { None }
        }
    }
    
    pub fn download_auto_splitter(auto_splitter: &AutoSplitter) -> Option<PathBuf> {
        
        let file_paths = &mut Vec::<PathBuf>::new();
        
        for url in &auto_splitter.urls.urls {
            let response = match get(url) {
                Ok(response) => { response }
                Err(_) => { continue }
            };
            
            let file_name = match Self::get_requested_file_name(response.headers()) {
                Some(file_name) => { file_name }
                None => {
                    log::warn!("Couldn't get name for auto splitter file: {}, defaulting to 'Unknown.wasm'", url);
                    String::from("Unknown.wasm") 
                }
            };

            let file_path = Path::join(&*super::AUTO_SPLITTERS_PATH, file_name.clone());
            
            let file = File::create(file_path.clone());
            
            if let (Ok(bytes), Ok(mut file)) = (response.bytes(), file) {
                match file.write_all(bytes.as_ref()) {
                    Ok(_) => { file_paths.push(file_path); }
                    Err(e) => { log::warn!("Something went wrong when downloading and saving auto splitter file: {}: {}", url, e) }
                };
            }
        };
        
        let auto_splitter_file_path = file_paths.iter().find(|path| {
            if let Some(extension) = path.extension() {
                return extension == "wasm";
            }
            
            false
        });
        
        match auto_splitter_file_path {
            Some(path) => { return Some(path.clone()) }
            None => { None }
        }
    }
    
    pub fn is_using_auto_splitting_runtime(auto_splitter: &AutoSplitter) -> bool {
        return auto_splitter.script_type.is_some() && auto_splitter.script_type.as_ref().unwrap() == "AutoSplittingRuntime";
    }

    fn get_auto_splitter_list() -> Result<(String, AutoSplitterList), (GetAutoSplitterListFromGithubError, GetAutoSplitterListFromFileError)> {
        let from_github_result = Self::get_auto_splitter_list_from_github();

        let from_github_error = match from_github_result {
            Ok(auto_splitters) => { return Ok(auto_splitters) }
            Err(e) => { e }
        };

        let from_file_result = Self::get_auto_splitter_list_from_file();

        let from_file_error = match from_file_result {
            Ok(auto_splitters) => { return Ok(auto_splitters) }
            Err(e) => { e }
        };

        Err((from_github_error, from_file_error))
    }

    fn get_auto_splitter_list_from_github() -> Result<(String, AutoSplitterList), GetAutoSplitterListFromGithubError> {
        let url = "https://raw.githubusercontent.com/LiveSplit/LiveSplit.AutoSplitters/master/LiveSplit.AutoSplitters.xml";

        let response = match get(url) {
            Ok(response) => { response }
            Err(e) => { return Err(GetAutoSplitterListFromGithubError::NetError(e)); }
        };

        let body = match response.text() {
            Ok(body) => { body }
            Err(e) => { return Err(GetAutoSplitterListFromGithubError::NetError(e)); }
        };

        match from_str::<AutoSplitterList>(body.as_str()) {
            Ok(auto_splitters) => { Ok((body, auto_splitters)) }
            Err(e) => { return Err(GetAutoSplitterListFromGithubError::DeserializationError(e)) }
        }
    }

    fn get_auto_splitter_list_from_file() -> Result<(String, AutoSplitterList), GetAutoSplitterListFromFileError> {
        let mut file = match File::open(Path::join(&*super::OBS_MODULE_CONFIG_PATH, AUTO_SPLITTER_LIST_FILE_NAME)) {
            Ok(file) => { file }
            Err(e) => { return Err(GetAutoSplitterListFromFileError::IoError(e)) }
        };

        let mut buffer = String::new();

        match file.read_to_string(&mut buffer) {
            Ok(_) => { }
            Err(e) => { return Err(GetAutoSplitterListFromFileError::IoError(e)) }
        }

        match from_str::<AutoSplitterList>(buffer.as_str()) {
            Ok(auto_splitters) => { Ok((buffer, auto_splitters)) }
            Err(e) => { return Err(GetAutoSplitterListFromFileError::DeserializationError(e)) }
        }
    }
    
    // TODO: Make this return Result and improve support
    fn get_requested_file_name(header_map: &HeaderMap) -> Option<String> {
        return match header_map.get(CONTENT_DISPOSITION) {
            Some(content_disposition_header) => {
                let content_disposition_params = hyperx::header::ContentDisposition::parse_header(&content_disposition_header).ok()?.parameters;

                for param in content_disposition_params {
                    match param {
                        DispositionParam::Filename(_, _, bytes) => {
                            match from_utf8(bytes.as_slice()) {
                                Ok(file_name) => { return Some(file_name.to_string()); }
                                Err(_) => { }
                            };
                        }
                        DispositionParam::Ext(_, _) => { }
                    }
                }
                
                return None;
            }
            None => { None }
        }
    }
}