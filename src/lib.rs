use core::fmt;
use std::{
    cmp::Ordering,
    ffi::{c_void, CStr},
    fs::{self, File},
    io::{BufWriter, Cursor},
    mem,
    os::raw::{c_char, c_int},
    path::{Path, PathBuf},
    process::Command,
    ptr,
    sync::{
        atomic::{self, AtomicPtr},
        Arc, Mutex, Weak,
    },
};

mod ffi;
mod ffi_types;

use ffi::{
    blog, gs_draw_sprite, gs_effect_get_param_by_name, gs_effect_get_technique,
    gs_effect_set_texture, gs_effect_t, gs_technique_begin, gs_technique_begin_pass,
    gs_technique_end, gs_technique_end_pass, gs_texture_create, gs_texture_destroy,
    gs_texture_set_image, gs_texture_t, obs_data_array_count, obs_data_array_item,
    obs_data_array_release, obs_data_get_array, obs_data_get_bool, obs_data_get_int,
    obs_data_get_json, obs_data_get_string, obs_data_release, obs_data_set_default_bool,
    obs_data_set_default_int, obs_data_t, obs_enter_graphics, obs_get_base_effect, obs_hotkey_id,
    obs_hotkey_register_source, obs_hotkey_t, obs_leave_graphics, obs_mouse_event,
    obs_properties_add_bool, obs_properties_add_button, obs_properties_add_editable_list,
    obs_properties_add_int, obs_properties_add_path, obs_properties_add_text,
    obs_properties_create, obs_properties_get, obs_property_set_modified_callback2,
    obs_property_set_visible, obs_property_t, obs_register_source_s, obs_source_info, obs_source_t,
    GS_DYNAMIC, GS_RGBA, LOG_WARNING, OBS_EDITABLE_LIST_TYPE_STRINGS,
    OBS_EFFECT_PREMULTIPLIED_ALPHA, OBS_ICON_TYPE_GAME_CAPTURE, OBS_PATH_FILE,
    OBS_SOURCE_CONTROLLABLE_MEDIA, OBS_SOURCE_CUSTOM_DRAW, OBS_SOURCE_INTERACTION,
    OBS_SOURCE_TYPE_INPUT, OBS_SOURCE_VIDEO,
};
use ffi_types::{
    obs_media_state, obs_module_t, obs_properties_t, LOG_DEBUG, LOG_ERROR, LOG_INFO,
    OBS_MEDIA_STATE_ENDED, OBS_MEDIA_STATE_PAUSED, OBS_MEDIA_STATE_PLAYING,
    OBS_MEDIA_STATE_STOPPED, OBS_PATH_DIRECTORY, OBS_TEXT_DEFAULT,
};

use livesplit_core::{
    layout::{self, LayoutSettings, LayoutState},
    rendering::software::Renderer,
    run::{
        parser::{composite, TimerKind},
        saver::livesplit::{save_timer, IoWrite},
    },
    Layout, Run, Segment, SharedTimer, Timer, TimerPhase,
};
use log::{debug, info, warn, Level, LevelFilter, Log, Metadata, Record};
use serde_derive::Deserialize;
use serde_json::from_str;

#[cfg(feature = "auto-splitting")]
use {
    self::ffi::{
        obs_data_set_bool, obs_data_set_string, obs_properties_add_list,
        obs_property_list_add_string, obs_property_set_description, obs_property_set_enabled,
        OBS_COMBO_FORMAT_STRING, OBS_COMBO_TYPE_LIST, OBS_TEXT_INFO,
    },
    livesplit_core::auto_splitting,
    livesplit_core::auto_splitting::settings::{Value, Widget, WidgetKind},
    log::error,
    std::{ffi::CString, sync::atomic::AtomicBool},
};

macro_rules! cstr {
    ($f:literal) => {
        concat!($f, '\0').as_ptr().cast::<std::os::raw::c_char>()
    };
}

#[cfg(feature = "auto-splitting")]
mod auto_splitters;

static OBS_MODULE_POINTER: AtomicPtr<obs_module_t> = AtomicPtr::new(ptr::null_mut());

// This function is required for the OBS module registration to happen
// It is essentially like calling OBS_DECLARE_MODULE() in C
#[no_mangle]
pub extern "C" fn obs_module_set_pointer(module: *mut obs_module_t) {
    OBS_MODULE_POINTER.store(module, atomic::Ordering::Relaxed);
}

#[no_mangle]
pub extern "C" fn obs_module_ver() -> u32 {
    (26 << 24) | (1 << 16) | 1
}

struct UnsafeMultiThread<T>(T);

unsafe impl<T> Sync for UnsafeMultiThread<T> {}
unsafe impl<T> Send for UnsafeMultiThread<T> {}

struct GlobalTimer {
    path: PathBuf,
    can_save_splits: bool,
    timer: SharedTimer,
    #[cfg(feature = "auto-splitting")]
    auto_splitter: auto_splitting::Runtime,
    #[cfg(feature = "auto-splitting")]
    auto_splitter_is_enabled: AtomicBool,
}

static TIMERS: Mutex<Vec<Weak<GlobalTimer>>> = Mutex::new(Vec::new());

struct State {
    #[cfg(feature = "auto-splitting")]
    local_auto_splitter: Option<PathBuf>,
    use_game_arguments: bool,
    game_arguments: String,
    game_working_directory: Option<PathBuf>,
    game_environment_vars: Vec<(String, String)>,
    game_path: PathBuf,
    global_timer: Arc<GlobalTimer>,
    auto_save: bool,
    layout: Layout,
    state: LayoutState,
    renderer: Renderer,
    texture: *mut gs_texture_t,
    width: u32,
    height: u32,
    activated: bool,
    #[cfg(feature = "auto-splitting")]
    auto_splitter_settings: Arc<Vec<Widget>>,
    #[cfg(feature = "auto-splitting")]
    obs_settings: *mut obs_data_t,
}

struct Settings {
    #[cfg(feature = "auto-splitting")]
    local_auto_splitter: Option<PathBuf>,
    use_game_arguments: bool,
    game_arguments: String,
    game_working_directory: Option<PathBuf>,
    game_environment_vars: Vec<(String, String)>,
    game_path: PathBuf,
    splits_path: PathBuf,
    auto_save: bool,
    layout: Layout,
    width: u32,
    height: u32,
}

#[derive(Deserialize)]
struct ObsEditableListEntry {
    value: String,
    #[serde(rename = "selected")]
    _selected: bool,
    #[serde(rename = "hidden")]
    _hidden: bool,
}

fn parse_run(path: &Path) -> Option<(Run, bool)> {
    let file_data = fs::read(path).ok()?;
    let run = composite::parse(&file_data, Some(Path::new(path))).ok()?;
    if run.run.is_empty() {
        return None;
    }
    Some((run.run, run.kind == TimerKind::LiveSplit))
}

fn log(level: Level, target: &str, args: &fmt::Arguments<'_>) {
    let str = format!("[LiveSplit One][{target}] {args}\0");
    let level = match level {
        Level::Error => LOG_ERROR,
        Level::Warn => LOG_WARNING,
        Level::Info => LOG_INFO,
        Level::Debug | Level::Trace => LOG_DEBUG,
    };
    unsafe {
        blog(level as _, cstr!("%s"), str.as_ptr());
    }
}

fn parse_layout(path: &CStr) -> Option<Layout> {
    let path = path.to_str().ok()?;
    if path.is_empty() {
        return None;
    }
    let file_data = fs::read_to_string(path).ok()?;

    if let Ok(settings) = LayoutSettings::from_json(Cursor::new(file_data.as_bytes())) {
        return Some(Layout::from_settings(settings));
    }

    layout::parser::parse(&file_data).ok()
}

fn save_splits_file(state: &State) -> bool {
    if state.global_timer.can_save_splits {
        let timer = state.global_timer.timer.read().unwrap();
        if let Ok(file) = File::create(&state.global_timer.path) {
            let _ = save_timer(&timer, IoWrite(BufWriter::new(file)));
        }
    }
    false
}

unsafe fn get_game_environment_vars(settings: *mut obs_data_t) -> Vec<(String, String)> {
    let environment_list = obs_data_get_array(settings, SETTINGS_GAME_ENVIRONMENT_LIST);
    let count = obs_data_array_count(environment_list);

    let mut vars = Vec::<(String, String)>::new();

    for i in 0..count {
        let item = obs_data_array_item(environment_list, i);
        'use_item: {
            let raw_json = obs_data_get_json(item);
            let raw_json = CStr::from_ptr(raw_json.cast()).to_string_lossy();
            let entry = match from_str::<ObsEditableListEntry>(raw_json.as_ref()) {
                Ok(entry) => entry,
                Err(e) => {
                    warn!("Couldn't read item {i} contents: {e}");
                    break 'use_item;
                }
            };

            let (key, value) = match entry.value.split_once('=') {
                Some((key, value)) => (key, value),
                None => {
                    warn!("Invalid environment variable entry: '{}'", entry.value);
                    break 'use_item;
                }
            };

            vars.push((key.to_string(), value.to_string()));
        }

        obs_data_release(item);
    }
    obs_data_array_release(environment_list);

    vars
}

unsafe fn parse_settings(settings: *mut obs_data_t) -> Settings {
    #[cfg(feature = "auto-splitting")]
    let local_auto_splitter = {
        let uses_local_auto_splitter = obs_data_get_bool(settings, SETTINGS_LOCAL_AUTO_SPLITTER);
        if uses_local_auto_splitter {
            let local_auto_splitter_path = CStr::from_ptr(
                obs_data_get_string(settings, SETTINGS_LOCAL_AUTO_SPLITTER_PATH).cast(),
            );

            Some(PathBuf::from(
                local_auto_splitter_path.to_string_lossy().into_owned(),
            ))
        } else {
            None
        }
    };

    let use_game_arguments = obs_data_get_bool(settings, SETTINGS_USE_GAME_ARGUMENTS);
    let game_arguments =
        CStr::from_ptr(obs_data_get_string(settings, SETTINGS_GAME_ARGUMENTS).cast())
            .to_string_lossy()
            .to_string();
    let game_working_directory =
        CStr::from_ptr(obs_data_get_string(settings, SETTINGS_GAME_WORKING_DIRECTORY).cast())
            .to_string_lossy();
    let game_working_directory = (!game_working_directory.is_empty())
        .then_some(PathBuf::from(game_working_directory.into_owned()));
    let game_environment_vars = get_game_environment_vars(settings);

    let game_path = CStr::from_ptr(obs_data_get_string(settings, SETTINGS_GAME_PATH).cast());
    let game_path = PathBuf::from(game_path.to_string_lossy().into_owned());

    let splits_path = CStr::from_ptr(obs_data_get_string(settings, SETTINGS_SPLITS_PATH).cast());
    let splits_path = PathBuf::from(splits_path.to_string_lossy().into_owned());

    let auto_save = obs_data_get_bool(settings, SETTINGS_AUTO_SAVE);

    let layout_path = CStr::from_ptr(obs_data_get_string(settings, SETTINGS_LAYOUT_PATH).cast());
    let layout = parse_layout(layout_path).unwrap_or_else(Layout::default_layout);

    let width = obs_data_get_int(settings, SETTINGS_WIDTH) as u32;
    let height = obs_data_get_int(settings, SETTINGS_HEIGHT) as u32;

    Settings {
        #[cfg(feature = "auto-splitting")]
        local_auto_splitter,
        use_game_arguments,
        game_arguments,
        game_working_directory,
        game_environment_vars,
        game_path,
        splits_path,
        auto_save,
        layout,
        width,
        height,
    }
}

impl State {
    unsafe fn new(
        Settings {
            #[cfg(feature = "auto-splitting")]
            local_auto_splitter,
            use_game_arguments,
            game_arguments,
            game_working_directory,
            game_environment_vars,
            game_path,
            splits_path,
            auto_save,
            layout,
            width,
            height,
        }: Settings,
        #[cfg(feature = "auto-splitting")] obs_settings: *mut obs_data_t,
    ) -> Self {
        debug!("Loading settings.");

        let global_timer = get_global_timer(splits_path);

        let state = LayoutState::default();
        let renderer = Renderer::new();

        obs_enter_graphics();
        let texture = gs_texture_create(width, height, GS_RGBA, 1, ptr::null_mut(), GS_DYNAMIC);
        obs_leave_graphics();

        #[cfg(feature = "auto-splitting")]
        if let Some(local_auto_splitter) = &local_auto_splitter {
            auto_splitter_load(&global_timer, local_auto_splitter.clone())
        }

        Self {
            #[cfg(feature = "auto-splitting")]
            local_auto_splitter,
            use_game_arguments,
            game_arguments,
            game_working_directory,
            game_environment_vars,
            game_path,
            global_timer,
            auto_save,
            layout,
            state,
            renderer,
            texture,
            width,
            height,
            activated: false,
            #[cfg(feature = "auto-splitting")]
            auto_splitter_settings: Arc::<Vec<Widget>>::default(),
            #[cfg(feature = "auto-splitting")]
            obs_settings,
        }
    }

    unsafe fn render(&mut self) {
        self.layout.update_state(
            &mut self.state,
            &self.global_timer.timer.read().unwrap().snapshot(),
        );

        self.renderer.render(&self.state, [self.width, self.height]);
        gs_texture_set_image(
            self.texture,
            self.renderer.image_data().as_ptr(),
            self.width * 4,
            false,
        );
    }
}

unsafe extern "C" fn get_name(_: *mut c_void) -> *const c_char {
    cstr!("LiveSplit One")
}

unsafe extern "C" fn split(
    data: *mut c_void,
    _: obs_hotkey_id,
    _: *mut obs_hotkey_t,
    pressed: bool,
) {
    if !pressed {
        return;
    }

    let state: &mut State = &mut *data.cast();
    if !state.activated {
        return;
    }

    state.global_timer.timer.write().unwrap().split_or_start();
}

unsafe extern "C" fn reset(
    data: *mut c_void,
    _: obs_hotkey_id,
    _: *mut obs_hotkey_t,
    pressed: bool,
) {
    if !pressed {
        return;
    }

    let state: &mut State = &mut *data.cast();
    if !state.activated {
        return;
    }

    state.global_timer.timer.write().unwrap().reset(true);

    if state.auto_save {
        save_splits_file(state);
    }
}

unsafe extern "C" fn undo(
    data: *mut c_void,
    _: obs_hotkey_id,
    _: *mut obs_hotkey_t,
    pressed: bool,
) {
    if !pressed {
        return;
    }

    let state: &mut State = &mut *data.cast();
    if !state.activated {
        return;
    }

    state.global_timer.timer.write().unwrap().undo_split();
}

unsafe extern "C" fn skip(
    data: *mut c_void,
    _: obs_hotkey_id,
    _: *mut obs_hotkey_t,
    pressed: bool,
) {
    if !pressed {
        return;
    }

    let state: &mut State = &mut *data.cast();
    if !state.activated {
        return;
    }

    state.global_timer.timer.write().unwrap().skip_split();
}

unsafe extern "C" fn pause(
    data: *mut c_void,
    _: obs_hotkey_id,
    _: *mut obs_hotkey_t,
    pressed: bool,
) {
    if !pressed {
        return;
    }

    let state: &mut State = &mut *data.cast();
    if !state.activated {
        return;
    }

    state
        .global_timer
        .timer
        .write()
        .unwrap()
        .toggle_pause_or_start();
}

unsafe extern "C" fn undo_all_pauses(
    data: *mut c_void,
    _: obs_hotkey_id,
    _: *mut obs_hotkey_t,
    pressed: bool,
) {
    if !pressed {
        return;
    }

    let state: &mut State = &mut *data.cast();
    if !state.activated {
        return;
    }

    state.global_timer.timer.write().unwrap().undo_all_pauses();
}

unsafe extern "C" fn previous_comparison(
    data: *mut c_void,
    _: obs_hotkey_id,
    _: *mut obs_hotkey_t,
    pressed: bool,
) {
    if !pressed {
        return;
    }

    let state: &mut State = &mut *data.cast();
    if !state.activated {
        return;
    }

    state
        .global_timer
        .timer
        .write()
        .unwrap()
        .switch_to_previous_comparison();
}

unsafe extern "C" fn next_comparison(
    data: *mut c_void,
    _: obs_hotkey_id,
    _: *mut obs_hotkey_t,
    pressed: bool,
) {
    if !pressed {
        return;
    }

    let state: &mut State = &mut *data.cast();
    if !state.activated {
        return;
    }

    state
        .global_timer
        .timer
        .write()
        .unwrap()
        .switch_to_next_comparison();
}

unsafe extern "C" fn toggle_timing_method(
    data: *mut c_void,
    _: obs_hotkey_id,
    _: *mut obs_hotkey_t,
    pressed: bool,
) {
    if !pressed {
        return;
    }

    let state: &mut State = &mut *data.cast();
    if !state.activated {
        return;
    }

    state
        .global_timer
        .timer
        .write()
        .unwrap()
        .toggle_timing_method();
}

unsafe extern "C" fn create(settings: *mut obs_data_t, source: *mut obs_source_t) -> *mut c_void {
    #[cfg(feature = "auto-splitting")]
    let data = Box::into_raw(Box::new(State::new(parse_settings(settings), settings))).cast();
    #[cfg(not(feature = "auto-splitting"))]
    let data = Box::into_raw(Box::new(State::new(parse_settings(settings)))).cast();

    obs_hotkey_register_source(
        source,
        cstr!("hotkey_split"),
        cstr!("Split"),
        Some(split),
        data,
    );

    obs_hotkey_register_source(
        source,
        cstr!("hotkey_reset"),
        cstr!("Reset"),
        Some(reset),
        data,
    );

    obs_hotkey_register_source(
        source,
        cstr!("hotkey_undo"),
        cstr!("Undo Split"),
        Some(undo),
        data,
    );

    obs_hotkey_register_source(
        source,
        cstr!("hotkey_skip"),
        cstr!("Skip Split"),
        Some(skip),
        data,
    );

    obs_hotkey_register_source(
        source,
        cstr!("hotkey_pause"),
        cstr!("Pause"),
        Some(pause),
        data,
    );

    obs_hotkey_register_source(
        source,
        cstr!("hotkey_undo_all_pauses"),
        cstr!("Undo All Pauses"),
        Some(undo_all_pauses),
        data,
    );

    obs_hotkey_register_source(
        source,
        cstr!("hotkey_previous_comparison"),
        cstr!("Previous Comparison"),
        Some(previous_comparison),
        data,
    );

    obs_hotkey_register_source(
        source,
        cstr!("hotkey_next_comparison"),
        cstr!("Next Comparison"),
        Some(next_comparison),
        data,
    );

    obs_hotkey_register_source(
        source,
        cstr!("hotkey_toggle_timing_method"),
        cstr!("Toggle Timing Method"),
        Some(toggle_timing_method),
        data,
    );

    data
}

unsafe extern "C" fn destroy(data: *mut c_void) {
    let state: Box<State> = Box::from_raw(data.cast());
    obs_enter_graphics();
    gs_texture_destroy(state.texture);
    obs_leave_graphics();
}

unsafe extern "C" fn get_width(data: *mut c_void) -> u32 {
    let state: &mut State = &mut *data.cast();
    state.width
}

unsafe extern "C" fn get_height(data: *mut c_void) -> u32 {
    let state: &mut State = &mut *data.cast();
    state.height
}

unsafe extern "C" fn video_render(data: *mut c_void, _: *mut gs_effect_t) {
    let state: &mut State = &mut *data.cast();
    state.render();

    let effect = obs_get_base_effect(OBS_EFFECT_PREMULTIPLIED_ALPHA);
    let tech = gs_effect_get_technique(effect, cstr!("Draw"));

    gs_technique_begin(tech);
    gs_technique_begin_pass(tech, 0);

    gs_effect_set_texture(
        gs_effect_get_param_by_name(effect, cstr!("image")),
        state.texture,
    );
    gs_draw_sprite(state.texture, 0, 0, 0);

    gs_technique_end_pass(tech);
    gs_technique_end(tech);
}

unsafe extern "C" fn mouse_wheel(
    data: *mut c_void,
    _: *const obs_mouse_event,
    _: c_int,
    y_delta: c_int,
) {
    let state: &mut State = &mut *data.cast();
    match y_delta.cmp(&0) {
        Ordering::Less => state.layout.scroll_down(),
        Ordering::Equal => {}
        Ordering::Greater => state.layout.scroll_up(),
    }
}

unsafe extern "C" fn save_splits(
    _: *mut obs_properties_t,
    _: *mut obs_property_t,
    data: *mut c_void,
) -> bool {
    let state: &mut State = &mut *data.cast();
    save_splits_file(state)
}

unsafe extern "C" fn use_game_arguments_modified(
    data: *mut c_void,
    props: *mut obs_properties_t,
    _prop: *mut obs_property_t,
    settings: *mut obs_data_t,
) -> bool {
    let state: &mut State = &mut *data.cast();

    let use_game_arguments = obs_data_get_bool(settings, SETTINGS_USE_GAME_ARGUMENTS);

    // No UI change needed
    if state.use_game_arguments == use_game_arguments {
        return false;
    }

    let game_arguments = obs_properties_get(props, SETTINGS_GAME_ARGUMENTS);
    let game_working_directory = obs_properties_get(props, SETTINGS_GAME_WORKING_DIRECTORY);
    let game_env_list = obs_properties_get(props, SETTINGS_GAME_ENVIRONMENT_LIST);

    obs_property_set_visible(game_arguments, use_game_arguments);
    obs_property_set_visible(game_working_directory, use_game_arguments);
    obs_property_set_visible(game_env_list, use_game_arguments);

    true
}

unsafe extern "C" fn start_game_clicked(
    _props: *mut obs_properties_t,
    _prop: *mut obs_property_t,
    data: *mut c_void,
) -> bool {
    let state: &mut State = &mut *data.cast();

    if !state.game_path.exists() {
        warn!("No path provided to start a game.");
        return false;
    }
    let mut command = Command::new(state.game_path.clone());

    if state.use_game_arguments {
        // Is the game arguments string empty / whitespace only?
        if !state.game_arguments.trim().is_empty() {
            debug!("Parsing game arguments");

            #[cfg(windows)]
            std::os::windows::process::CommandExt::raw_arg(&mut command, &state.game_arguments);

            #[cfg(not(windows))]
            {
                let game_arguments = match shlex::split(state.game_arguments.as_str()) {
                    Some(arguments) => arguments,
                    None => {
                        warn!("Could not parse the game command arguments");
                        return false;
                    }
                };

                if !game_arguments.is_empty() {
                    command.args(game_arguments);
                }
            }
        }

        for (key, var) in &state.game_environment_vars {
            command.env(key, var);
        }

        if let Some(game_working_directory) = &state.game_working_directory {
            if game_working_directory.exists() {
                command.current_dir(game_working_directory);
            } else {
                warn!("Provided working directory was not found, using the default one.");
            }
        } else {
            info!("No working directory provided, using the default one.");
        }
    }

    info!("Starting game...");

    let _child = command.spawn();

    #[cfg(unix)]
    {
        // For Unix systems only, spawn a new thread that waits for the process to exit.
        // This avoids keeping the process in a zombie state and never letting go of it until
        // the plugin is unloaded

        let mut child = match _child {
            Ok(child) => child,
            Err(e) => {
                warn!("Failure starting the game process {e}");
                return false;
            }
        };

        std::thread::spawn(move || match child.wait() {
            Ok(exit_status) => {
                info!("Game process exited with {}", exit_status);
            }
            Err(e) => {
                warn!("Failure waiting for the game process' exit: {e}");
            }
        });
    }

    false
}

#[cfg(feature = "auto-splitting")]
const DEFAULT_AUTO_SPLITTER_LIST_SETTING: *const c_char = cstr!("obs-livesplit-one-default-choice");
#[cfg(feature = "auto-splitting")]
const DEFAULT_AUTO_SPLITTER_SETTING_TOOLTIP: *const c_char = cstr!("Waiting for setting selection");

#[cfg(feature = "auto-splitting")]
unsafe extern "C" fn settings_list_modified(
    data: *mut c_void,
    props: *mut obs_properties_t,
    _prop: *mut obs_property_t,
    settings: *mut obs_data_t,
) -> bool {
    let default_setting_string = CStr::from_ptr(DEFAULT_AUTO_SPLITTER_LIST_SETTING);
    let list_setting_string = CStr::from_ptr(obs_data_get_string(
        settings,
        SETTINGS_AUTO_SPLITTER_SETTINGS_LIST,
    ));

    let tooltip_property = obs_properties_get(props, SETTINGS_AUTO_SPLITTER_SETTINGS_TOOLTIP);
    let enable_property = obs_properties_get(props, SETTINGS_AUTO_SPLITTER_SETTINGS_ENABLE);
    let file_select_property = obs_properties_get(props, SETTINGS_AUTO_SPLITTER_SETTINGS_FILE_SELECT);

    macro_rules! reset_auto_splitter_setting_ui {
        () => {
            obs_property_set_description(tooltip_property, DEFAULT_AUTO_SPLITTER_SETTING_TOOLTIP);
            obs_property_set_enabled(enable_property, false);
            obs_property_set_enabled(file_select_property, false);
            return true;
        };
    }

    if list_setting_string == default_setting_string {
        reset_auto_splitter_setting_ui!();
    }

    let state: &mut State = &mut *data.cast();

    let list_setting_string = list_setting_string.to_str().unwrap_or_default();

    let user_setting = state
        .auto_splitter_settings
        .iter()
        .find(|x| x.key.as_ref() == list_setting_string);

    if let Some(user_setting) = user_setting {
        match user_setting.kind {
            WidgetKind::Title { heading_level: _ } => {
                obs_property_set_enabled(enable_property, false);
                obs_property_set_enabled(file_select_property, false);
            }
            WidgetKind::Bool {
                default_value: default,
            } => {
                match state
                    .global_timer
                    .auto_splitter
                    .settings_map()
                    .unwrap_or_default()
                    .get(user_setting.key.as_ref())
                {
                    Some(Value::Bool(value)) => {
                        obs_property_set_enabled(enable_property, true);
                        obs_property_set_enabled(file_select_property, false);
                        obs_data_set_bool(settings, SETTINGS_AUTO_SPLITTER_SETTINGS_ENABLE, *value);
                    }
                    Some(_) => {
                        warn!("Unknown / unimplemented value type");
                    }
                    None => {
                        obs_property_set_enabled(enable_property, true);
                        obs_property_set_enabled(file_select_property, false);
                        obs_data_set_bool(
                            settings,
                            SETTINGS_AUTO_SPLITTER_SETTINGS_ENABLE,
                            default,
                        );
                    }
                }
            }
            WidgetKind::Choice { .. } => {
                warn!("Unimplemented setting type Choice");
            }
            WidgetKind::FileSelect { .. } => {
                match state
                    .global_timer
                    .auto_splitter
                    .settings_map()
                    .unwrap_or_default()
                    .get(user_setting.key.as_ref())
                {
                    Some(Value::String(value)) => {
                        obs_property_set_enabled(enable_property, false);
                        obs_property_set_enabled(file_select_property, true);
                        let value_cs = CString::new(&**value).unwrap_or_default();
                        obs_data_set_string(settings, SETTINGS_AUTO_SPLITTER_SETTINGS_FILE_SELECT, value_cs.as_ptr());
                    }
                    Some(_) => {
                        warn!("Unknown / unimplemented value type");
                    }
                    None => {
                        obs_property_set_enabled(enable_property, false);
                        obs_property_set_enabled(file_select_property, true);
                        obs_data_set_string(
                            settings,
                            SETTINGS_AUTO_SPLITTER_SETTINGS_FILE_SELECT,
                            ptr::null(),
                        );
                    }
                }
            }
        }

        match &user_setting.tooltip {
            Some(tooltip) => {
                let tooltip = CString::new(tooltip.to_string()).unwrap_or_default();
                obs_property_set_description(tooltip_property, tooltip.as_ptr());
            }
            None => {
                obs_property_set_description(tooltip_property, cstr!("No tooltip to show"));
            }
        }

        true
    } else {
        reset_auto_splitter_setting_ui!();
    }
}

#[cfg(feature = "auto-splitting")]
unsafe extern "C" fn settings_enable_modified(
    data: *mut c_void,
    _props: *mut obs_properties_t,
    _prop: *mut obs_property_t,
    settings: *mut obs_data_t,
) -> bool {
    let default_setting_string = CStr::from_ptr(DEFAULT_AUTO_SPLITTER_LIST_SETTING);
    let list_setting_string = CStr::from_ptr(obs_data_get_string(
        settings,
        SETTINGS_AUTO_SPLITTER_SETTINGS_LIST,
    ));

    if list_setting_string == default_setting_string {
        return false;
    }

    let state: &mut State = &mut *data.cast();

    let value = obs_data_get_bool(settings, SETTINGS_AUTO_SPLITTER_SETTINGS_ENABLE);

    obs_data_set_bool(settings, SETTINGS_AUTO_SPLITTER_SETTINGS_ENABLE, value);

    let setting_key = match list_setting_string.to_str() {
        Ok(value) => value,
        Err(_) => {
            warn!("Tried to set invalid setting key");
            return false;
        }
    };

    match state.global_timer.auto_splitter.settings_map() {
        Some(mut map) => {
            map.insert(Arc::from(setting_key), Value::Bool(value));
            state.global_timer.auto_splitter.set_settings_map(map);
        }
        None => {
            warn!("The settings map could not be loaded");
            return false;
        }
    };

    true
}

#[cfg(feature = "auto-splitting")]
unsafe extern "C" fn settings_file_select_modified(
    data: *mut c_void,
    _props: *mut obs_properties_t,
    _prop: *mut obs_property_t,
    settings: *mut obs_data_t,
) -> bool {
    let default_setting_string = CStr::from_ptr(DEFAULT_AUTO_SPLITTER_LIST_SETTING);
    let list_setting_string = CStr::from_ptr(obs_data_get_string(
        settings,
        SETTINGS_AUTO_SPLITTER_SETTINGS_LIST,
    ));

    if list_setting_string == default_setting_string {
        return false;
    }

    let state: &mut State = &mut *data.cast();

    let value = obs_data_get_string(settings, SETTINGS_AUTO_SPLITTER_SETTINGS_FILE_SELECT);

    obs_data_set_string(settings, SETTINGS_AUTO_SPLITTER_SETTINGS_FILE_SELECT, value);

    let setting_key = match list_setting_string.to_str() {
        Ok(value) => value,
        Err(_) => {
            warn!("Tried to set invalid setting key");
            return false;
        }
    };

    match state.global_timer.auto_splitter.settings_map() {
        Some(mut map) => {
            let value_str = CStr::from_ptr(value).to_string_lossy();
            map.insert(Arc::from(setting_key), Value::String(Arc::from(value_str)));
            state.global_timer.auto_splitter.set_settings_map(map);
        }
        None => {
            warn!("The settings map could not be loaded");
            return false;
        }
    };
    true
}

#[cfg(feature = "auto-splitting")]
unsafe extern "C" fn use_local_auto_splitter_modified(
    data: *mut c_void,
    props: *mut obs_properties_t,
    _prop: *mut obs_property_t,
    settings: *mut obs_data_t,
) -> bool {
    let state: &mut State = &mut *data.cast();

    let use_local_auto_splitter = obs_data_get_bool(settings, SETTINGS_LOCAL_AUTO_SPLITTER);

    // No UI change needed
    if state.local_auto_splitter.is_some() == use_local_auto_splitter {
        return false;
    }

    let auto_splitter_activate = obs_properties_get(props, SETTINGS_AUTO_SPLITTER_ACTIVATE);
    let auto_splitter_info = obs_properties_get(props, SETTINGS_AUTO_SPLITTER_INFO);
    let auto_splitter_website = obs_properties_get(props, SETTINGS_AUTO_SPLITTER_WEBSITE);

    let local_auto_splitter_path = obs_properties_get(props, SETTINGS_LOCAL_AUTO_SPLITTER_PATH);

    obs_property_set_visible(auto_splitter_info, !use_local_auto_splitter);
    obs_property_set_visible(auto_splitter_activate, !use_local_auto_splitter);
    obs_property_set_visible(auto_splitter_website, !use_local_auto_splitter);

    obs_property_set_visible(local_auto_splitter_path, use_local_auto_splitter);

    obs_property_set_description(auto_splitter_activate, cstr!("Activate"));

    update_auto_splitter_ui(
        auto_splitter_info,
        auto_splitter_website,
        auto_splitter_activate,
        state.global_timer.timer.read().unwrap().run().game_name(),
    );

    true
}

unsafe extern "C" fn splits_path_modified(
    data: *mut c_void,
    _props: *mut obs_properties_t,
    _prop: *mut obs_property_t,
    settings: *mut obs_data_t,
) -> bool {
    let splits_path = CStr::from_ptr(obs_data_get_string(settings, SETTINGS_SPLITS_PATH).cast());
    let splits_path = PathBuf::from(splits_path.to_string_lossy().into_owned());

    let state: &mut State = &mut *data.cast();

    handle_splits_path_change(state, splits_path);

    #[cfg(feature = "auto-splitting")]
    {
        let info_text = obs_properties_get(_props, SETTINGS_AUTO_SPLITTER_INFO);
        let website_button = obs_properties_get(_props, SETTINGS_AUTO_SPLITTER_WEBSITE);
        let activate_button = obs_properties_get(_props, SETTINGS_AUTO_SPLITTER_ACTIVATE);

        update_auto_splitter_ui(
            info_text,
            website_button,
            activate_button,
            state.global_timer.timer.read().unwrap().run().game_name(),
        );
        auto_splitter_update_activation_label(activate_button, state);
    }

    true
}

#[cfg(feature = "auto-splitting")]
unsafe fn update_auto_splitter_ui(
    info_text: *mut obs_property_t,
    website_button: *mut obs_property_t,
    activate_button: *mut obs_property_t,
    game_name: &str,
) {
    if let Some(auto_splitter) = auto_splitters::get_list().get_for_game(game_name) {
        obs_property_set_enabled(website_button, auto_splitter.website.is_some());

        if !auto_splitter.is_using_auto_splitting_runtime() {
            obs_property_set_enabled(activate_button, false);
            obs_property_set_description(
                info_text,
                cstr!("This game's auto splitter is incompatible with LiveSplit One."),
            );
        } else {
            obs_property_set_enabled(activate_button, true);

            let mut auto_splitter_description = auto_splitter.description.as_bytes().to_vec();
            auto_splitter_description.push(0);

            obs_property_set_description(
                info_text,
                auto_splitter_description.as_ptr().cast::<c_char>(),
            );
        }
    } else {
        obs_property_set_enabled(activate_button, false);
        obs_property_set_enabled(website_button, false);
        obs_property_set_description(
            info_text,
            cstr!("No auto splitter available for this game."),
        );
    }
}

#[cfg(feature = "auto-splitting")]
fn auto_splitter_unload(global_timer: &GlobalTimer) {
    global_timer.auto_splitter.unload().ok();

    global_timer
        .auto_splitter_is_enabled
        .store(false, atomic::Ordering::Relaxed);
}

#[cfg(feature = "auto-splitting")]
fn auto_splitter_load(global_timer: &GlobalTimer, path: PathBuf) {
    let enabled = if let Err(e) = global_timer
        .auto_splitter
        .load(path, global_timer.timer.clone())
    {
        warn!("Auto Splitter could not be loaded: {e}");
        false
    } else {
        true
    };

    global_timer
        .auto_splitter_is_enabled
        .store(enabled, atomic::Ordering::Relaxed);
}

#[cfg(feature = "auto-splitting")]
unsafe extern "C" fn auto_splitter_activate_clicked(
    _props: *mut obs_properties_t,
    prop: *mut obs_property_t,
    data: *mut c_void,
) -> bool {
    let state: &mut State = &mut *data.cast();

    state
        .global_timer
        .auto_splitter_is_enabled
        .fetch_xor(true, atomic::Ordering::Relaxed);

    auto_splitter_update_activation_label(prop, state);

    if state
        .global_timer
        .auto_splitter_is_enabled
        .load(atomic::Ordering::Relaxed)
    {
        if let Some(auto_splitter_path) = auto_splitters::get_downloader().download_for_game(
            auto_splitters::get_list(),
            state.global_timer.timer.read().unwrap().run().game_name(),
            auto_splitters::get_path(),
        ) {
            auto_splitter_load(&state.global_timer, auto_splitter_path);
        } else {
            error!("Couldn't download the auto splitter files.");
        }
    } else {
        auto_splitter_unload(&state.global_timer);
    }

    true
}

#[cfg(feature = "auto-splitting")]
unsafe fn auto_splitter_update_activation_label(
    activate_button_prop: *mut obs_property_t,
    state: &mut State,
) {
    let is_active = state
        .global_timer
        .auto_splitter_is_enabled
        .load(atomic::Ordering::Relaxed);

    obs_property_set_description(
        activate_button_prop,
        if !is_active {
            cstr!("Activate")
        } else {
            cstr!("Deactivate")
        },
    );
}

#[cfg(feature = "auto-splitting")]
unsafe extern "C" fn auto_splitter_open_website(
    _props: *mut obs_properties_t,
    _prop: *mut obs_property_t,
    data: *mut c_void,
) -> bool {
    let state: &mut State = &mut *data.cast();

    let website = auto_splitters::get_list()
        .get_website_for_game(state.global_timer.timer.read().unwrap().run().game_name());

    match website {
        Some(website) => {
            info!("Opening auto splitter website: {website}");
            match open::that(website) {
                Ok(_) => {}
                Err(e) => {
                    error!("Could not open website {e}.")
                }
            };
        }
        None => {
            warn!("This auto splitter does not have a website.")
        }
    }

    false
}

unsafe extern "C" fn media_get_state(data: *mut c_void) -> obs_media_state {
    let state: &mut State = &mut *data.cast();
    let phase = state.global_timer.timer.read().unwrap().current_phase();
    match phase {
        TimerPhase::NotRunning => OBS_MEDIA_STATE_STOPPED,
        TimerPhase::Running => OBS_MEDIA_STATE_PLAYING,
        TimerPhase::Ended => OBS_MEDIA_STATE_ENDED,
        TimerPhase::Paused => OBS_MEDIA_STATE_PAUSED,
    }
}

unsafe extern "C" fn media_play_pause(data: *mut c_void, pause: bool) {
    let state: &mut State = &mut *data.cast();
    let mut timer = state.global_timer.timer.write().unwrap();
    match timer.current_phase() {
        TimerPhase::NotRunning => {
            if !pause {
                timer.start()
            }
        }
        TimerPhase::Running => {
            if pause {
                timer.pause()
            }
        }
        TimerPhase::Ended => {}
        TimerPhase::Paused => {
            if !pause {
                timer.resume()
            }
        }
    }
}

unsafe extern "C" fn media_restart(data: *mut c_void) {
    let state: &mut State = &mut *data.cast();
    if state.auto_save {
        save_splits_file(state);
    }
    let mut timer = state.global_timer.timer.write().unwrap();
    timer.reset(true);
    timer.start();
}

unsafe extern "C" fn media_stop(data: *mut c_void) {
    let state: &mut State = &mut *data.cast();
    state.global_timer.timer.write().unwrap().reset(true);
    if state.auto_save {
        save_splits_file(state);
    }
}

unsafe extern "C" fn media_next(data: *mut c_void) {
    let state: &mut State = &mut *data.cast();
    state.global_timer.timer.write().unwrap().split();
}

unsafe extern "C" fn media_previous(data: *mut c_void) {
    let state: &mut State = &mut *data.cast();
    state.global_timer.timer.write().unwrap().undo_split();
}

unsafe extern "C" fn media_get_time(data: *mut c_void) -> i64 {
    let state: &mut State = &mut *data.cast();
    let timer = state.global_timer.timer.read().unwrap();
    let time = timer.snapshot().current_time()[timer.current_timing_method()].unwrap_or_default();
    let (secs, nanos) = time.to_seconds_and_subsec_nanoseconds();
    secs * 1000 + (nanos / 1_000_000) as i64
}

unsafe extern "C" fn media_get_duration(data: *mut c_void) -> i64 {
    let state: &mut State = &mut *data.cast();
    let timer = state.global_timer.timer.read().unwrap();
    let time = timer
        .run()
        .segments()
        .last()
        .unwrap()
        .personal_best_split_time()[timer.current_timing_method()]
    .unwrap_or_default();
    let (secs, nanos) = time.to_seconds_and_subsec_nanoseconds();
    secs * 1000 + (nanos / 1_000_000) as i64
}

const SETTINGS_WIDTH: *const c_char = cstr!("width");
const SETTINGS_HEIGHT: *const c_char = cstr!("height");
const SETTINGS_USE_GAME_ARGUMENTS: *const c_char = cstr!("game_use_arguments");
const SETTINGS_GAME_PATH: *const c_char = cstr!("game_path");
const SETTINGS_GAME_ARGUMENTS: *const c_char = cstr!("game_arguments");
const SETTINGS_GAME_WORKING_DIRECTORY: *const c_char = cstr!("game_working_directory");
const SETTINGS_GAME_ENVIRONMENT_LIST: *const c_char = cstr!("game_environment_list");
const SETTINGS_START_GAME: *const c_char = cstr!("start_game");
const SETTINGS_SPLITS_PATH: *const c_char = cstr!("splits_path");
const SETTINGS_AUTO_SAVE: *const c_char = cstr!("auto_save");
#[cfg(feature = "auto-splitting")]
const SETTINGS_LOCAL_AUTO_SPLITTER: *const c_char = cstr!("local_auto_splitter");
#[cfg(feature = "auto-splitting")]
const SETTINGS_LOCAL_AUTO_SPLITTER_PATH: *const c_char = cstr!("local_auto_splitter_path");
#[cfg(feature = "auto-splitting")]
const SETTINGS_AUTO_SPLITTER_INFO: *const c_char = cstr!("auto_splitter_info");
#[cfg(feature = "auto-splitting")]
const SETTINGS_AUTO_SPLITTER_ACTIVATE: *const c_char = cstr!("auto_splitter_activate");
#[cfg(feature = "auto-splitting")]
const SETTINGS_AUTO_SPLITTER_WEBSITE: *const c_char = cstr!("auto_splitter_website");
#[cfg(feature = "auto-splitting")]
const SETTINGS_AUTO_SPLITTER_SETTINGS_LIST: *const c_char = cstr!("auto_splitter_settings_list");
#[cfg(feature = "auto-splitting")]
const SETTINGS_AUTO_SPLITTER_SETTINGS_TOOLTIP: *const c_char =
    cstr!("auto_splitter_settings_tooltip");
#[cfg(feature = "auto-splitting")]
const SETTINGS_AUTO_SPLITTER_SETTINGS_ENABLE: *const c_char =
    cstr!("auto_splitter_settings_enable");
#[cfg(feature = "auto-splitting")]
const SETTINGS_AUTO_SPLITTER_SETTINGS_FILE_SELECT: *const c_char =
    cstr!("auto_splitter_settings_file_select");
const SETTINGS_LAYOUT_PATH: *const c_char = cstr!("layout_path");
const SETTINGS_SAVE_SPLITS: *const c_char = cstr!("save_splits");

unsafe extern "C" fn get_properties(data: *mut c_void) -> *mut obs_properties_t {
    debug!("We are getting the properties.");

    let props = obs_properties_create();
    obs_properties_add_int(props, SETTINGS_WIDTH, cstr!("Width"), 10, 8200, 10);
    obs_properties_add_int(props, SETTINGS_HEIGHT, cstr!("Height"), 10, 8200, 10);

    let splits_path = obs_properties_add_path(
        props,
        SETTINGS_SPLITS_PATH,
        cstr!("Splits"),
        OBS_PATH_FILE,
        cstr!("LiveSplit Splits (*.lss)"),
        ptr::null(),
    );
    obs_properties_add_bool(
        props,
        SETTINGS_AUTO_SAVE,
        cstr!("Automatically save splits file on reset"),
    );
    obs_properties_add_button(
        props,
        SETTINGS_SAVE_SPLITS,
        cstr!("Save Splits"),
        Some(save_splits),
    );

    obs_properties_add_path(
        props,
        SETTINGS_LAYOUT_PATH,
        cstr!("Layout"),
        OBS_PATH_FILE,
        cstr!("LiveSplit Layouts (*.lsl *.ls1l)"),
        ptr::null(),
    );

    let use_game_arguments = obs_properties_add_bool(
        props,
        SETTINGS_USE_GAME_ARGUMENTS,
        cstr!("Advanced start game options"),
    );

    obs_property_set_modified_callback2(
        use_game_arguments,
        Some(use_game_arguments_modified),
        data,
    );

    obs_properties_add_path(
        props,
        SETTINGS_GAME_PATH,
        cstr!("Game path"),
        OBS_PATH_FILE,
        cstr!("Executable files (*)"),
        ptr::null(),
    );
    let game_arguments = obs_properties_add_text(
        props,
        SETTINGS_GAME_ARGUMENTS,
        cstr!("Game arguments"),
        OBS_TEXT_DEFAULT,
    );
    let game_working_directory = obs_properties_add_path(
        props,
        SETTINGS_GAME_WORKING_DIRECTORY,
        cstr!("Working directory"),
        OBS_PATH_DIRECTORY,
        cstr!("Directories"),
        ptr::null(),
    );
    let game_env_list = obs_properties_add_editable_list(
        props,
        SETTINGS_GAME_ENVIRONMENT_LIST,
        cstr!("Game environment variables (KEY=VALUE)"),
        OBS_EDITABLE_LIST_TYPE_STRINGS,
        ptr::null(),
        ptr::null(),
    );

    let state: &mut State = &mut *data.cast();

    let uses_game_arguments = state.use_game_arguments;
    obs_property_set_visible(game_arguments, uses_game_arguments);
    obs_property_set_visible(game_working_directory, uses_game_arguments);
    obs_property_set_visible(game_env_list, uses_game_arguments);

    obs_properties_add_button(
        props,
        SETTINGS_START_GAME,
        cstr!("Start game"),
        Some(start_game_clicked),
    );

    obs_property_set_modified_callback2(splits_path, Some(splits_path_modified), data);

    #[cfg(feature = "auto-splitting")]
    {
        let use_local_auto_splitter = obs_properties_add_bool(
            props,
            SETTINGS_LOCAL_AUTO_SPLITTER,
            cstr!("Use local auto splitter"),
        );

        obs_property_set_modified_callback2(
            use_local_auto_splitter,
            Some(use_local_auto_splitter_modified),
            data,
        );

        let local_auto_splitter_path = obs_properties_add_path(
            props,
            SETTINGS_LOCAL_AUTO_SPLITTER_PATH,
            cstr!("Local Auto Splitter file"),
            OBS_PATH_FILE,
            cstr!("LiveSplit One Auto Splitter (*.wasm)"),
            ptr::null(),
        );

        let info_text = obs_properties_add_text(
            props,
            SETTINGS_AUTO_SPLITTER_INFO,
            cstr!("No splits loaded"),
            OBS_TEXT_INFO,
        );

        let activate_button_text = match state
            .global_timer
            .auto_splitter_is_enabled
            .load(atomic::Ordering::Relaxed)
        {
            true => cstr!("Deactivate"),
            false => cstr!("Activate"),
        };

        let activate_button = obs_properties_add_button(
            props,
            SETTINGS_AUTO_SPLITTER_ACTIVATE,
            activate_button_text,
            Some(auto_splitter_activate_clicked),
        );

        let website_button = obs_properties_add_button(
            props,
            SETTINGS_AUTO_SPLITTER_WEBSITE,
            cstr!("Website"),
            Some(auto_splitter_open_website),
        );

        update_auto_splitter_ui(
            info_text,
            website_button,
            activate_button,
            state.global_timer.timer.read().unwrap().run().game_name(),
        );

        let uses_local_auto_splitter = state.local_auto_splitter.is_some();
        obs_property_set_visible(info_text, !uses_local_auto_splitter);
        obs_property_set_visible(activate_button, !uses_local_auto_splitter);
        obs_property_set_visible(website_button, !uses_local_auto_splitter);

        obs_property_set_visible(local_auto_splitter_path, uses_local_auto_splitter);

        if !state
            .global_timer
            .auto_splitter_is_enabled
            .load(atomic::Ordering::Relaxed)
        {
            return props;
        }

        let settings_list = obs_properties_add_list(
            props,
            SETTINGS_AUTO_SPLITTER_SETTINGS_LIST,
            cstr!("Custom auto splitter settings"),
            OBS_COMBO_TYPE_LIST,
            OBS_COMBO_FORMAT_STRING,
        );

        obs_properties_add_text(
            props,
            SETTINGS_AUTO_SPLITTER_SETTINGS_TOOLTIP,
            DEFAULT_AUTO_SPLITTER_SETTING_TOOLTIP,
            OBS_TEXT_INFO,
        );

        let settings_enable = obs_properties_add_bool(
            props,
            SETTINGS_AUTO_SPLITTER_SETTINGS_ENABLE,
            cstr!("Enable"),
        );

        let settings_file_select = obs_properties_add_path(
            props,
            SETTINGS_AUTO_SPLITTER_SETTINGS_FILE_SELECT,
            cstr!("Select a file"),
            OBS_PATH_FILE,
            cstr!("All files (*.*)"),
            ptr::null(),
        );

        obs_property_list_add_string(
            settings_list,
            cstr!("Select a setting to change"),
            DEFAULT_AUTO_SPLITTER_LIST_SETTING,
        );

        obs_data_set_string(
            state.obs_settings,
            SETTINGS_AUTO_SPLITTER_SETTINGS_LIST,
            DEFAULT_AUTO_SPLITTER_LIST_SETTING,
        );
        obs_data_set_bool(
            state.obs_settings,
            SETTINGS_AUTO_SPLITTER_SETTINGS_ENABLE,
            false,
        );

        obs_property_set_modified_callback2(settings_list, Some(settings_list_modified), data);
        obs_property_set_modified_callback2(settings_enable, Some(settings_enable_modified), data);
        obs_property_set_modified_callback2(settings_file_select, Some(settings_file_select_modified), data);

        let auto_splitter_settings = state.global_timer.auto_splitter.settings_widgets();

        if let Some(auto_splitter_settings) = auto_splitter_settings {
            state.auto_splitter_settings = auto_splitter_settings;

            for setting in state.auto_splitter_settings.iter() {
                let setting_description = CString::new(setting.description.as_ref());

                let setting_key = CString::new(setting.key.as_ref());

                if let (Ok(setting_key), Ok(setting_description)) =
                    (setting_key, setting_description)
                {
                    obs_property_list_add_string(
                        settings_list,
                        setting_description.as_ptr(),
                        setting_key.as_ptr(),
                    );
                } else {
                    warn!(
                        "Couldn't add invalid setting to the settings list ({}: {})",
                        setting.key, setting.description
                    );
                }
            }
        }
    }

    props
}

unsafe extern "C" fn get_defaults(settings: *mut obs_data_t) {
    obs_data_set_default_int(settings, SETTINGS_WIDTH, 300);
    obs_data_set_default_int(settings, SETTINGS_HEIGHT, 500);
    obs_data_set_default_bool(settings, SETTINGS_AUTO_SAVE, false);
}

unsafe extern "C" fn activate(data: *mut c_void) {
    let state: &mut State = &mut *data.cast();
    state.activated = true;
}

unsafe extern "C" fn deactivate(data: *mut c_void) {
    let state: &mut State = &mut *data.cast();
    state.activated = false;
}

fn default_run() -> (Run, bool) {
    let mut run = Run::new();
    run.push_segment(Segment::new("Time"));
    (run, false)
}

unsafe extern "C" fn update(data: *mut c_void, settings: *mut obs_data_t) {
    debug!("Reloading settings.");

    let state: &mut State = &mut *data.cast();
    let settings = parse_settings(settings);

    handle_splits_path_change(state, settings.splits_path);

    state.use_game_arguments = settings.use_game_arguments;

    state.game_arguments = settings.game_arguments;
    state.game_working_directory = settings.game_working_directory;
    state.game_environment_vars = settings.game_environment_vars;

    state.game_path = settings.game_path;

    state.auto_save = settings.auto_save;
    state.layout = settings.layout;

    #[cfg(feature = "auto-splitting")]
    {
        if state.local_auto_splitter != settings.local_auto_splitter {
            auto_splitter_unload(&state.global_timer);

            state.local_auto_splitter = settings.local_auto_splitter;

            if let Some(local_auto_splitter) = &state.local_auto_splitter {
                auto_splitter_load(&state.global_timer, local_auto_splitter.clone());
            }
        }
    }

    if state.width != settings.width || state.height != settings.height {
        state.width = settings.width;
        state.height = settings.height;

        obs_enter_graphics();
        let mut texture = gs_texture_create(
            state.width,
            state.height,
            GS_RGBA,
            1,
            ptr::null_mut(),
            GS_DYNAMIC,
        );
        mem::swap(&mut state.texture, &mut texture);
        gs_texture_destroy(texture);
        obs_leave_graphics();
    }
}

fn handle_splits_path_change(state: &mut State, splits_path: PathBuf) {
    state.global_timer = get_global_timer(splits_path);
}

fn get_global_timer(splits_path: PathBuf) -> Arc<GlobalTimer> {
    let mut timers = TIMERS.lock().unwrap();
    timers.retain(|timer| timer.strong_count() > 0);
    if let Some(timer) = timers.iter().find_map(|timer| {
        let timer = timer.upgrade()?;
        if timer.path == splits_path {
            Some(timer)
        } else {
            None
        }
    }) {
        debug!("Found timer to reuse.");
        timer
    } else {
        debug!("Storing timer for reuse.");
        let (run, can_save_splits) = parse_run(&splits_path).unwrap_or_else(default_run);
        let timer = Timer::new(run).unwrap().into_shared();
        #[cfg(feature = "auto-splitting")]
        let auto_splitter = auto_splitting::Runtime::new();
        let global_timer = Arc::new(GlobalTimer {
            path: splits_path,
            can_save_splits,
            timer,
            #[cfg(feature = "auto-splitting")]
            auto_splitter,
            #[cfg(feature = "auto-splitting")]
            auto_splitter_is_enabled: AtomicBool::new(false),
        });
        timers.push(Arc::downgrade(&global_timer));
        global_timer
    }
}

struct ObsLog;

impl Log for ObsLog {
    fn enabled(&self, _metadata: &Metadata) -> bool {
        true
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            log(record.level(), record.target(), record.args());
        }
    }

    fn flush(&self) {}
}

#[no_mangle]
pub extern "C" fn obs_module_load() -> bool {
    static SOURCE_INFO: UnsafeMultiThread<obs_source_info> = UnsafeMultiThread(obs_source_info {
        id: cstr!("livesplit-one"),
        type_: OBS_SOURCE_TYPE_INPUT,
        output_flags: OBS_SOURCE_VIDEO
            | OBS_SOURCE_CUSTOM_DRAW
            | OBS_SOURCE_INTERACTION
            | OBS_SOURCE_CONTROLLABLE_MEDIA,
        get_name: Some(get_name),
        create: Some(create),
        destroy: Some(destroy),
        get_width: Some(get_width),
        get_height: Some(get_height),
        video_render: Some(video_render),
        mouse_wheel: Some(mouse_wheel),
        get_properties: Some(get_properties),
        get_defaults: Some(get_defaults),
        update: Some(update),
        icon_type: OBS_ICON_TYPE_GAME_CAPTURE,
        activate: Some(activate),
        deactivate: Some(deactivate),
        show: None,
        hide: None,
        video_tick: None,
        filter_video: None,
        filter_audio: None,
        enum_active_sources: None,
        save: None,
        load: None,
        mouse_click: None,
        mouse_move: None,
        focus: None,
        key_click: None,
        filter_remove: None,
        type_data: ptr::null_mut(),
        free_type_data: None,
        audio_render: None,
        enum_all_sources: None,
        transition_start: None,
        transition_stop: None,
        get_defaults2: None,
        get_properties2: None,
        audio_mix: None,
        media_play_pause: Some(media_play_pause),
        media_restart: Some(media_restart),
        media_stop: Some(media_stop),
        media_next: Some(media_next),
        media_previous: Some(media_previous),
        media_get_duration: Some(media_get_duration),
        media_get_time: Some(media_get_time),
        media_set_time: None,
        media_get_state: Some(media_get_state),
        version: 0,
        unversioned_id: ptr::null(),
    });

    let _ = log::set_logger(&ObsLog);
    log::set_max_level(LevelFilter::Debug);

    let source_info: &obs_source_info = &SOURCE_INFO.0;

    unsafe {
        obs_register_source_s(source_info, mem::size_of_val(source_info) as _);
    }

    #[cfg(feature = "auto-splitting")]
    auto_splitters::set_up();

    true
}
