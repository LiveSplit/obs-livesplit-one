use anyhow::{Context, Error, Result};
use livesplit_core::auto_splitting::list::{
    AutoSplitter, Downloader as ListDownloader, List, DEFAULT_LIST_URL,
};
use log::{error, info, warn};
use reqwest::Url;
use std::{
    ffi::CStr,
    fs,
    path::{Path, PathBuf},
    sync::{
        atomic::{self},
        OnceLock,
    },
};

use crate::ffi::obs_module_get_config_path;

const LIST_FILE_NAME: &str = "LiveSplit.AutoSplitters.xml";

pub fn get_module_config_path() -> &'static PathBuf {
    static OBS_MODULE_CONFIG_PATH: OnceLock<PathBuf> = OnceLock::new();

    OBS_MODULE_CONFIG_PATH.get_or_init(|| {
        let mut buffer = PathBuf::new();

        unsafe {
            let config_path_ptr = obs_module_get_config_path(
                crate::OBS_MODULE_POINTER.load(atomic::Ordering::Relaxed),
                cstr!(c""),
            );

            if let Ok(config_path) = CStr::from_ptr(config_path_ptr).to_str() {
                buffer.push(config_path);
            }
        }

        buffer
    })
}

static LIST_SOURCE: OnceLock<String> = OnceLock::new();

pub fn get_list() -> List<'static> {
    LIST_SOURCE
        .get()
        .map_or_else(List::empty, |source| List::new(source))
}

pub fn get_for_game(game_name: &str) -> Option<AutoSplitter<'static>> {
    lookup(get_list(), game_name)
}

fn lookup<'a>(list: List<'a>, game_name: &str) -> Option<AutoSplitter<'a>> {
    match list.get_for_game(game_name) {
        Ok(splitter) => splitter,
        Err(error) => {
            error!("Failed looking up the auto splitter for `{game_name}`: {error}");
            None
        }
    }
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
    client: ListDownloader,
    // OBS invokes these operations through synchronous callbacks. Keep the
    // runtime and persistence here, while livesplit-core only provides async
    // downloads into memory.
    runtime: tokio::runtime::Runtime,
}

impl Downloader {
    fn new() -> Self {
        Self {
            client: ListDownloader::new().expect("Failed creating the auto splitter HTTP client"),
            runtime: tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("Failed creating the auto splitter download runtime"),
        }
    }

    fn download_list(&self, folder: &Path) -> Result<String, [Error; 2]> {
        let download_error = match self
            .runtime
            .block_on(self.client.download_list(DEFAULT_LIST_URL))
        {
            Ok(source) => return Ok(source),
            Err(error) => Error::new(error),
        };

        match fs::read_to_string(folder.join(LIST_FILE_NAME)) {
            Ok(source) => {
                warn!("Failed downloading the auto splitters list. Using the cached version: {download_error:#}");
                Ok(source)
            }
            Err(error) => Err([download_error, error.into()]),
        }
    }

    pub fn download_for_game(
        &self,
        list: List<'_>,
        game_name: &str,
        folder: &Path,
    ) -> Option<PathBuf> {
        let splitter = lookup(list, game_name)?;

        if !splitter.is_using_auto_splitting_runtime() {
            return None;
        }

        let urls = splitter.urls();

        if !urls.iter().any(|url| {
            download_path(url, folder).is_ok_and(|path| {
                path.extension()
                    .is_some_and(|extension| extension == "wasm")
            })
        }) {
            error!("The auto splitter for `{game_name}` has no WebAssembly module URL.");
            return None;
        }

        let mut wasm_path = None;

        for url in urls {
            match self.download_file(url, folder) {
                Ok(path) => {
                    if wasm_path.is_none()
                        && path
                            .extension()
                            .is_some_and(|extension| extension == "wasm")
                    {
                        wasm_path = Some(path);
                    }
                }
                Err(error) => error!("Failed downloading `{url}`: {error:#}"),
            }
        }

        wasm_path
    }

    fn download_file(&self, url: &str, folder: &Path) -> Result<PathBuf> {
        let path = download_path(url, folder)?;
        let bytes = self
            .runtime
            .block_on(self.client.download_file(url))
            .context("Failed downloading the file.")?;

        fs::write(&path, bytes).context("Failed writing the file.")?;
        Ok(path)
    }
}

fn download_path(url: &str, folder: &Path) -> Result<PathBuf> {
    let url = Url::parse(url).context("Failed parsing the URL.")?;
    let file_name = url
        .path_segments()
        .and_then(|mut segments| segments.next_back())
        .context("There is no file name in the URL.")?;

    let file_name = percent_encoding::percent_decode_str(file_name).decode_utf8_lossy();

    anyhow::ensure!(
        !file_name.is_empty()
            && !matches!(file_name.as_ref(), "." | "..")
            && !file_name.contains(['/', '\\', ':', '\0']),
        "The URL does not contain a safe file name."
    );

    Ok(folder.join(file_name.as_ref()))
}

pub fn set_up() {
    let folder = get_path();

    if let Err(error) = fs::create_dir_all(folder) {
        error!("Failed creating the auto splitters folder: {error}");
    }

    match get_downloader().download_list(folder) {
        Ok(source) => {
            if let Err(error) = fs::write(folder.join(LIST_FILE_NAME), &source) {
                error!("Failed saving the list of auto splitters: {error}");
            }

            let _ = LIST_SOURCE.set(source);
            info!("Auto splitter list loaded.");
        }
        Err([download_error, cache_error]) => {
            error!("Failed downloading the list of auto splitters: {download_error:#}");
            error!("Failed loading the cached list of auto splitters: {cache_error:#}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uses_runtime_metadata_and_download_url() {
        let source = r#"<AutoSplitters><AutoSplitter>
            <Games><Game>Example &amp; Game</Game></Games>
            <URLs><URL>https://example.com/legacy.asl</URL></URLs>
            <Description>Legacy description</Description>
            <Website>https://example.com/legacy</Website>
            <AutoSplittingRuntime>
                <URL>https://example.com/game%20one.wasm?x=1&amp;y=2</URL>
                <Description>Runtime description</Description>
                <Website>https://example.com/runtime</Website>
            </AutoSplittingRuntime>
        </AutoSplitter></AutoSplitters>"#;

        let splitter = lookup(List::new(source), "Example & Game").unwrap();
        assert!(splitter.is_using_auto_splitting_runtime());
        assert_eq!(splitter.description, "Runtime description");

        assert_eq!(
            splitter.website.as_deref(),
            Some("https://example.com/runtime")
        );

        assert_eq!(
            splitter.urls(),
            ["https://example.com/game%20one.wasm?x=1&y=2"]
        );

        assert_eq!(
            download_path(&splitter.urls()[0], Path::new("splitters")).unwrap(),
            Path::new("splitters/game one.wasm")
        );
    }

    #[test]
    fn lookup_handles_missing_games_and_malformed_xml() {
        assert!(lookup(List::empty(), "Unknown").is_none());
        assert!(lookup(List::new("<AutoSplitters>"), "Unknown").is_none());
    }

    #[test]
    fn rejects_download_paths_outside_the_folder() {
        for name in [
            "",
            "..%2foutside.wasm",
            "..%5coutside.wasm",
            "C%3aoutside.wasm",
        ] {
            assert!(download_path(
                &format!("https://example.com/{name}"),
                Path::new("splitters")
            )
            .is_err());
        }
    }

    #[test]
    fn downloads_selected_runtime_module_through_core() {
        use std::{
            io::{BufRead, BufReader, Write},
            net::TcpListener,
            thread,
            time::Duration,
        };

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();

        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();

            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();

            let mut reader = BufReader::new(&stream);
            let mut line = String::new();
            reader.read_line(&mut line).unwrap();
            assert_eq!(line.trim(), "GET /game%20one.wasm?download=1 HTTP/1.1");

            loop {
                line.clear();
                reader.read_line(&mut line).unwrap();

                if line == "\r\n" || line.is_empty() {
                    break;
                }
            }

            stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 8\r\nConnection: close\r\n\r\n\0asm\x01\0\0\0").unwrap();
        });

        let source = format!(
            r#"<AutoSplitters><AutoSplitter>
            <Games><Game>Test</Game></Games>
            <URLs><URL>http://{address}/legacy.asl</URL></URLs>
            <Description>Test</Description>
            <AutoSplittingRuntime><URL>http://{address}/game%20one.wasm?download=1</URL></AutoSplittingRuntime>
        </AutoSplitter></AutoSplitters>"#
        );

        let mut downloader = Downloader::new();
        downloader.client = ListDownloader::with_client(
            reqwest::Client::builder()
                .no_proxy()
                .timeout(Duration::from_secs(5))
                .build()
                .unwrap(),
        );

        let folder =
            std::env::temp_dir().join(format!("obs-livesplit-one-download-{}", std::process::id()));

        fs::create_dir(&folder).unwrap();
        let result = downloader.download_for_game(List::new(&source), "Test", &folder);
        let bytes = result.as_ref().map(fs::read);
        let count = fs::read_dir(&folder).unwrap().count();
        fs::remove_dir_all(&folder).unwrap();

        assert_eq!(result, Some(folder.join("game one.wasm")));
        assert_eq!(bytes.unwrap().unwrap(), b"\0asm\x01\0\0\0");
        assert_eq!(count, 1);
        server.join().unwrap();
    }
}
