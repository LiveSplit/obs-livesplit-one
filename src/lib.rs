use core::fmt;
use std::{
    borrow::Cow,
    cmp::Ordering,
    ffi::{c_void, CStr},
    fs::{self, File},
    future::Future,
    io::{BufWriter, Cursor},
    mem,
    os::raw::{c_char, c_int},
    path::{Path, PathBuf},
    process::Command,
    ptr,
    sync::{
        atomic::{self, AtomicBool, AtomicPtr},
        Arc, Mutex, RwLock, RwLockReadGuard, Weak,
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
    obs_data_get_json, obs_data_get_string, obs_data_release, obs_data_set_bool,
    obs_data_set_default_bool, obs_data_set_default_int, obs_data_t, obs_enter_graphics,
    obs_get_base_effect, obs_hotkey_id, obs_hotkey_register_source, obs_hotkey_t,
    obs_leave_graphics, obs_mouse_event, obs_properties_add_bool, obs_properties_add_button,
    obs_properties_add_editable_list, obs_properties_add_int, obs_properties_add_path,
    obs_properties_add_text, obs_properties_create, obs_properties_get,
    obs_property_set_modified_callback2, obs_property_set_visible, obs_property_t,
    obs_register_source_s, obs_source_info, obs_source_t, GS_DYNAMIC, GS_RGBA, LOG_WARNING,
    OBS_EDITABLE_LIST_TYPE_STRINGS, OBS_EFFECT_PREMULTIPLIED_ALPHA, OBS_ICON_TYPE_GAME_CAPTURE,
    OBS_PATH_FILE, OBS_SOURCE_CONTROLLABLE_MEDIA, OBS_SOURCE_CUSTOM_DRAW, OBS_SOURCE_INTERACTION,
    OBS_SOURCE_TYPE_INPUT, OBS_SOURCE_VIDEO,
};
use ffi_types::{
    obs_media_state, obs_module_t, obs_properties_t, LOG_DEBUG, LOG_ERROR, LOG_INFO,
    OBS_MEDIA_STATE_ENDED, OBS_MEDIA_STATE_PAUSED, OBS_MEDIA_STATE_PLAYING,
    OBS_MEDIA_STATE_STOPPED, OBS_PATH_DIRECTORY, OBS_TEXT_DEFAULT,
};

use livesplit_core::{
    event::{CommandSink, Event, Result, TimerQuery},
    layout::{self, LayoutSettings, LayoutState},
    rendering::software::Renderer,
    run::{
        parser::{composite, TimerKind},
        saver::livesplit::{save_timer, IoWrite},
    },
    settings::ImageCache,
    Layout, Run, Segment, TimeSpan, Timer, TimerPhase, TimingMethod,
};
use log::{debug, error, info, warn, Level, LevelFilter, Log, Metadata, Record};
use serde_derive::Deserialize;
use serde_json::from_str;

#[cfg(feature = "auto-splitting")]
use {
    self::ffi::{
        obs_data_erase, obs_data_set_default_string, obs_data_set_string, obs_properties_add_group,
        obs_properties_add_list, obs_property_list_add_string, obs_property_set_description,
        obs_property_set_enabled, obs_property_set_long_description, obs_source_update_properties,
        OBS_COMBO_FORMAT_STRING, OBS_COMBO_TYPE_LIST, OBS_GROUP_NORMAL, OBS_TEXT_INFO,
    },
    livesplit_core::auto_splitting::{
        self,
        settings::{self, FileFilter, Value, Widget, WidgetKind},
        wasi_path,
    },
    std::ffi::CString,
};

macro_rules! cstr {
    ($f:literal) => {
        std::ffi::CStr::as_ptr($f)
    };
}

#[cfg(feature = "auto-splitting")]
mod auto_splitters;

static OBS_MODULE_POINTER: AtomicPtr<obs_module_t> = AtomicPtr::new(ptr::null_mut());

// This function is required for the OBS module registration to happen
// It is essentially like calling OBS_DECLARE_MODULE() in C
#[unsafe(no_mangle)]
pub extern "C" fn obs_module_set_pointer(module: *mut obs_module_t) {
    OBS_MODULE_POINTER.store(module, atomic::Ordering::Relaxed);
}

#[unsafe(no_mangle)]
pub extern "C" fn obs_module_ver() -> u32 {
    (26 << 24) | (1 << 16) | 1
}

struct UnsafeMultiThread<T>(T);

unsafe impl<T> Sync for UnsafeMultiThread<T> {}
unsafe impl<T> Send for UnsafeMultiThread<T> {}

struct GlobalTimer {
    timer: Arc<InnerTimer>,
    #[cfg(feature = "auto-splitting")]
    auto_splitter: auto_splitting::Runtime<Arc<InnerTimer>>,
    #[cfg(feature = "auto-splitting")]
    auto_splitter_is_enabled: AtomicBool,
}

struct InnerTimer {
    path: PathBuf,
    can_save_splits: bool,
    timer: RwLock<Timer>,
    auto_save: AtomicBool,
}

impl InnerTimer {
    fn save(&self) {
        if self.can_save_splits {
            if let Ok(file) = File::create(&self.path) {
                let _ = save_timer(&self.get_timer(), IoWrite(BufWriter::new(file)));
                info!("Saved splits to `{}`.", self.path.display());
            }
        }
    }
}

impl CommandSink for InnerTimer {
    fn start(&self) -> impl Future<Output = Result> + 'static {
        let result = self.timer.write().unwrap().start();
        async move { result }
    }

    fn split(&self) -> impl Future<Output = Result> + 'static {
        let result = self.timer.write().unwrap().split();
        async move { result }
    }

    fn split_or_start(&self) -> impl Future<Output = Result> + 'static {
        let result = self.timer.write().unwrap().split_or_start();
        async move { result }
    }

    fn reset(&self, save_attempt: Option<bool>) -> impl Future<Output = Result> + 'static {
        let result = self
            .timer
            .write()
            .unwrap()
            .reset(save_attempt.unwrap_or(true));

        if result.is_ok() && self.auto_save.load(atomic::Ordering::Relaxed) {
            self.save();
        }

        async move { result }
    }

    fn undo_split(&self) -> impl Future<Output = Result> + 'static {
        let result = self.timer.write().unwrap().undo_split();
        async move { result }
    }

    fn skip_split(&self) -> impl Future<Output = Result> + 'static {
        let result = self.timer.write().unwrap().skip_split();
        async move { result }
    }

    fn toggle_pause_or_start(&self) -> impl Future<Output = Result> + 'static {
        let result = self.timer.write().unwrap().toggle_pause_or_start();
        async move { result }
    }

    fn pause(&self) -> impl Future<Output = Result> + 'static {
        let result = self.timer.write().unwrap().pause();
        async move { result }
    }

    fn resume(&self) -> impl Future<Output = Result> + 'static {
        let result = self.timer.write().unwrap().resume();
        async move { result }
    }

    fn undo_all_pauses(&self) -> impl Future<Output = Result> + 'static {
        let result = self.timer.write().unwrap().undo_all_pauses();
        async move { result }
    }

    fn switch_to_previous_comparison(&self) -> impl Future<Output = Result> + 'static {
        self.timer.write().unwrap().switch_to_previous_comparison();
        async { Ok(Event::ComparisonChanged) }
    }

    fn switch_to_next_comparison(&self) -> impl Future<Output = Result> + 'static {
        self.timer.write().unwrap().switch_to_next_comparison();
        async { Ok(Event::ComparisonChanged) }
    }

    fn toggle_timing_method(&self) -> impl Future<Output = Result> + 'static {
        self.timer.write().unwrap().toggle_timing_method();
        async move { Ok(Event::TimingMethodChanged) }
    }

    fn set_game_time(&self, time: TimeSpan) -> impl Future<Output = Result> + 'static {
        let result = self.timer.write().unwrap().set_game_time(time);
        async move { result }
    }

    fn pause_game_time(&self) -> impl Future<Output = Result> + 'static {
        let result = self.timer.write().unwrap().pause_game_time();
        async move { result }
    }

    fn resume_game_time(&self) -> impl Future<Output = Result> + 'static {
        let result = self.timer.write().unwrap().resume_game_time();
        async move { result }
    }

    fn set_custom_variable(
        &self,
        name: Cow<'_, str>,
        value: Cow<'_, str>
    ) -> impl Future<Output = Result> + 'static {
        self.timer.write().unwrap().set_custom_variable(name, value);
        async { Ok(Event::CustomVariableSet) }
    }

    fn set_current_comparison(&self, comparison: Cow<'_, str>) -> impl Future<Output = Result> + 'static {
        let result = self
            .timer
            .write()
            .unwrap()
            .set_current_comparison(comparison);
        async move { result }
    }

    fn set_current_timing_method(
        &self,
        method: TimingMethod,
    ) -> impl Future<Output = Result> + 'static {
        self.timer
            .write()
            .unwrap()
            .set_current_timing_method(method);
        async { Ok(Event::TimingMethodChanged) }
    }

    fn initialize_game_time(&self) -> impl Future<Output = Result> + 'static {
        let result = self.timer.write().unwrap().initialize_game_time();
        async move { result }
    }

    fn set_loading_times(&self, time: TimeSpan) -> impl Future<Output = Result> + 'static {
        let result = self.timer.write().unwrap().set_loading_times(time);
        async move { result }
    }
}

impl TimerQuery for InnerTimer {
    type Guard<'a> = RwLockReadGuard<'a, Timer>;

    fn get_timer(&self) -> Self::Guard<'_> {
        self.timer.read().unwrap()
    }
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
    layout: Layout,
    state: LayoutState,
    image_cache: ImageCache,
    renderer: Renderer,
    texture: *mut gs_texture_t,
    width: u32,
    height: u32,
    activated: bool,
    obs_settings: *mut obs_data_t,
    #[cfg(feature = "auto-splitting")]
    auto_splitter_widgets: Arc<Vec<Widget>>,
    #[cfg(feature = "auto-splitting")]
    auto_splitter_map: settings::Map,
    #[cfg(feature = "auto-splitting")]
    source: *mut obs_source_t,
}

impl Drop for State {
    fn drop(&mut self) {
        unsafe {
            obs_enter_graphics();
            gs_texture_destroy(self.texture);
            obs_leave_graphics();
        }
    }
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
        blog(level as _, cstr!(c"%s"), str.as_ptr());
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

unsafe fn get_game_environment_vars(settings: *mut obs_data_t) -> Vec<(String, String)> { unsafe {
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
}}

unsafe fn parse_settings(settings: *mut obs_data_t) -> Settings { unsafe {
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
}}

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
        _source: *mut obs_source_t,
        obs_settings: *mut obs_data_t,
    ) -> Self { unsafe {
        debug!("Loading settings.");

        let global_timer = get_global_timer(splits_path);
        global_timer
            .timer
            .auto_save
            .store(auto_save, atomic::Ordering::Relaxed);

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
            layout,
            state,
            image_cache: ImageCache::new(),
            renderer,
            texture,
            width,
            height,
            activated: false,
            obs_settings,
            #[cfg(feature = "auto-splitting")]
            auto_splitter_widgets: Arc::default(),
            #[cfg(feature = "auto-splitting")]
            auto_splitter_map: settings::Map::new(),
            #[cfg(feature = "auto-splitting")]
            source: _source,
        }
    }}

    unsafe fn render(&mut self) { unsafe {
        self.layout.update_state(
            &mut self.state,
            &mut self.image_cache,
            &self.global_timer.timer.get_timer().snapshot(),
        );

        self.renderer
            .render(&self.state, &self.image_cache, [self.width, self.height]);

        gs_texture_set_image(
            self.texture,
            self.renderer.image_data().as_ptr(),
            self.width * 4,
            false,
        );

        self.image_cache.collect();

        #[cfg(feature = "auto-splitting")]
        {
            let mut needs_properties_update = false;

            if let Some(auto_splitter_widgets) = self.global_timer.auto_splitter.settings_widgets()
            {
                if !Arc::ptr_eq(&self.auto_splitter_widgets, &auto_splitter_widgets) {
                    self.auto_splitter_widgets = auto_splitter_widgets;
                    needs_properties_update = true;
                }
            }

            if let Some(auto_splitter_map) = self.global_timer.auto_splitter.settings_map() {
                if !self.auto_splitter_map.is_unchanged(&auto_splitter_map) {
                    self.auto_splitter_map = auto_splitter_map;
                    needs_properties_update = true;
                }
            }

            if needs_properties_update {
                obs_source_update_properties(self.source);
            }
        }
    }}
}

unsafe extern "C" fn get_name(_: *mut c_void) -> *const c_char {
    cstr!(c"LiveSplit One")
}

unsafe extern "C" fn split(
    data: *mut c_void,
    _: obs_hotkey_id,
    _: *mut obs_hotkey_t,
    pressed: bool,
) { unsafe {
    if !pressed {
        return;
    }

    let state: &mut State = &mut (*data.cast::<Mutex<State>>()).lock().unwrap();
    if !state.activated {
        return;
    }

    drop(state.global_timer.timer.split_or_start());
}}

unsafe extern "C" fn reset(
    data: *mut c_void,
    _: obs_hotkey_id,
    _: *mut obs_hotkey_t,
    pressed: bool,
) { unsafe {
    if !pressed {
        return;
    }

    let state: &mut State = &mut (*data.cast::<Mutex<State>>()).lock().unwrap();
    if !state.activated {
        return;
    }

    drop(state.global_timer.timer.reset(None));
}}

unsafe extern "C" fn undo(
    data: *mut c_void,
    _: obs_hotkey_id,
    _: *mut obs_hotkey_t,
    pressed: bool,
) { unsafe {
    if !pressed {
        return;
    }

    let state: &mut State = &mut (*data.cast::<Mutex<State>>()).lock().unwrap();
    if !state.activated {
        return;
    }

    drop(state.global_timer.timer.undo_split());
}}

unsafe extern "C" fn skip(
    data: *mut c_void,
    _: obs_hotkey_id,
    _: *mut obs_hotkey_t,
    pressed: bool,
) { unsafe {
    if !pressed {
        return;
    }

    let state: &mut State = &mut (*data.cast::<Mutex<State>>()).lock().unwrap();
    if !state.activated {
        return;
    }

    drop(state.global_timer.timer.skip_split());
}}

unsafe extern "C" fn pause(
    data: *mut c_void,
    _: obs_hotkey_id,
    _: *mut obs_hotkey_t,
    pressed: bool,
) { unsafe {
    if !pressed {
        return;
    }

    let state: &mut State = &mut (*data.cast::<Mutex<State>>()).lock().unwrap();
    if !state.activated {
        return;
    }

    drop(state.global_timer.timer.toggle_pause_or_start());
}}

unsafe extern "C" fn undo_all_pauses(
    data: *mut c_void,
    _: obs_hotkey_id,
    _: *mut obs_hotkey_t,
    pressed: bool,
) { unsafe {
    if !pressed {
        return;
    }

    let state: &mut State = &mut (*data.cast::<Mutex<State>>()).lock().unwrap();
    if !state.activated {
        return;
    }

    drop(state.global_timer.timer.undo_all_pauses());
}}

unsafe extern "C" fn previous_comparison(
    data: *mut c_void,
    _: obs_hotkey_id,
    _: *mut obs_hotkey_t,
    pressed: bool,
) { unsafe {
    if !pressed {
        return;
    }

    let state: &mut State = &mut (*data.cast::<Mutex<State>>()).lock().unwrap();
    if !state.activated {
        return;
    }

    drop(state.global_timer.timer.switch_to_previous_comparison());
}}

unsafe extern "C" fn next_comparison(
    data: *mut c_void,
    _: obs_hotkey_id,
    _: *mut obs_hotkey_t,
    pressed: bool,
) { unsafe {
    if !pressed {
        return;
    }

    let state: &mut State = &mut (*data.cast::<Mutex<State>>()).lock().unwrap();
    if !state.activated {
        return;
    }

    drop(state.global_timer.timer.switch_to_next_comparison());
}}

unsafe extern "C" fn toggle_timing_method(
    data: *mut c_void,
    _: obs_hotkey_id,
    _: *mut obs_hotkey_t,
    pressed: bool,
) { unsafe {
    if !pressed {
        return;
    }

    let state: &mut State = &mut (*data.cast::<Mutex<State>>()).lock().unwrap();
    if !state.activated {
        return;
    }

    drop(state.global_timer.timer.toggle_timing_method());
}}

unsafe extern "C" fn create(settings: *mut obs_data_t, source: *mut obs_source_t) -> *mut c_void { unsafe {
    let data = Box::into_raw(Box::new(Mutex::new(State::new(
        parse_settings(settings),
        source,
        settings,
    ))))
    .cast();

    obs_hotkey_register_source(
        source,
        cstr!(c"hotkey_split"),
        cstr!(c"Split"),
        Some(split),
        data,
    );

    obs_hotkey_register_source(
        source,
        cstr!(c"hotkey_reset"),
        cstr!(c"Reset"),
        Some(reset),
        data,
    );

    obs_hotkey_register_source(
        source,
        cstr!(c"hotkey_undo"),
        cstr!(c"Undo Split"),
        Some(undo),
        data,
    );

    obs_hotkey_register_source(
        source,
        cstr!(c"hotkey_skip"),
        cstr!(c"Skip Split"),
        Some(skip),
        data,
    );

    obs_hotkey_register_source(
        source,
        cstr!(c"hotkey_pause"),
        cstr!(c"Pause"),
        Some(pause),
        data,
    );

    obs_hotkey_register_source(
        source,
        cstr!(c"hotkey_undo_all_pauses"),
        cstr!(c"Undo All Pauses"),
        Some(undo_all_pauses),
        data,
    );

    obs_hotkey_register_source(
        source,
        cstr!(c"hotkey_previous_comparison"),
        cstr!(c"Previous Comparison"),
        Some(previous_comparison),
        data,
    );

    obs_hotkey_register_source(
        source,
        cstr!(c"hotkey_next_comparison"),
        cstr!(c"Next Comparison"),
        Some(next_comparison),
        data,
    );

    obs_hotkey_register_source(
        source,
        cstr!(c"hotkey_toggle_timing_method"),
        cstr!(c"Toggle Timing Method"),
        Some(toggle_timing_method),
        data,
    );

    data
}}

unsafe extern "C" fn destroy(data: *mut c_void) { unsafe {
    drop(Box::<Mutex<State>>::from_raw(data.cast()));
}}

unsafe extern "C" fn get_width(data: *mut c_void) -> u32 { unsafe {
    let state: &mut State = &mut (*data.cast::<Mutex<State>>()).lock().unwrap();
    state.width
}}

unsafe extern "C" fn get_height(data: *mut c_void) -> u32 { unsafe {
    let state: &mut State = &mut (*data.cast::<Mutex<State>>()).lock().unwrap();
    state.height
}}

unsafe extern "C" fn video_render(data: *mut c_void, _: *mut gs_effect_t) { unsafe {
    let state: &mut State = &mut (*data.cast::<Mutex<State>>()).lock().unwrap();
    state.render();

    let effect = obs_get_base_effect(OBS_EFFECT_PREMULTIPLIED_ALPHA);
    let tech = gs_effect_get_technique(effect, cstr!(c"Draw"));

    gs_technique_begin(tech);
    gs_technique_begin_pass(tech, 0);

    gs_effect_set_texture(
        gs_effect_get_param_by_name(effect, cstr!(c"image")),
        state.texture,
    );
    gs_draw_sprite(state.texture, 0, 0, 0);

    gs_technique_end_pass(tech);
    gs_technique_end(tech);
}}

unsafe extern "C" fn mouse_wheel(
    data: *mut c_void,
    _: *const obs_mouse_event,
    _: c_int,
    y_delta: c_int,
) { unsafe {
    let state: &mut State = &mut (*data.cast::<Mutex<State>>()).lock().unwrap();
    match y_delta.cmp(&0) {
        Ordering::Less => state.layout.scroll_down(),
        Ordering::Equal => {}
        Ordering::Greater => state.layout.scroll_up(),
    }
}}

unsafe extern "C" fn save_splits(
    _: *mut obs_properties_t,
    _: *mut obs_property_t,
    data: *mut c_void,
) -> bool { unsafe {
    let state: &mut State = &mut (*data.cast::<Mutex<State>>()).lock().unwrap();
    state.global_timer.timer.save();
    false
}}

unsafe extern "C" fn use_game_arguments_modified(
    data: *mut c_void,
    props: *mut obs_properties_t,
    _prop: *mut obs_property_t,
    settings: *mut obs_data_t,
) -> bool { unsafe {
    let state: &mut State = &mut (*data.cast::<Mutex<State>>()).lock().unwrap();

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
}}

unsafe extern "C" fn start_game_clicked(
    _props: *mut obs_properties_t,
    _prop: *mut obs_property_t,
    data: *mut c_void,
) -> bool { unsafe {
    let state: &mut State = &mut (*data.cast::<Mutex<State>>()).lock().unwrap();

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
}}

#[cfg(feature = "auto-splitting")]
unsafe extern "C" fn use_local_auto_splitter_modified(
    data: *mut c_void,
    props: *mut obs_properties_t,
    _prop: *mut obs_property_t,
    settings: *mut obs_data_t,
) -> bool { unsafe {
    let state: &mut State = &mut (*data.cast::<Mutex<State>>()).lock().unwrap();

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

    obs_property_set_description(auto_splitter_activate, cstr!(c"Activate"));

    update_auto_splitter_ui(
        auto_splitter_info,
        auto_splitter_website,
        auto_splitter_activate,
        state.global_timer.timer.get_timer().run().game_name(),
    );

    true
}}

unsafe extern "C" fn splits_path_modified(
    data: *mut c_void,
    _props: *mut obs_properties_t,
    _prop: *mut obs_property_t,
    settings: *mut obs_data_t,
) -> bool { unsafe {
    let splits_path = CStr::from_ptr(obs_data_get_string(settings, SETTINGS_SPLITS_PATH).cast());
    let splits_path = PathBuf::from(splits_path.to_string_lossy().into_owned());

    let state: &mut State = &mut (*data.cast::<Mutex<State>>()).lock().unwrap();

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
            state.global_timer.timer.get_timer().run().game_name(),
        );
        auto_splitter_update_activation_label(activate_button, state);
    }

    true
}}

#[cfg(feature = "auto-splitting")]
unsafe fn update_auto_splitter_ui(
    info_text: *mut obs_property_t,
    website_button: *mut obs_property_t,
    activate_button: *mut obs_property_t,
    game_name: &str,
) { unsafe {
    if let Some(auto_splitter) = auto_splitters::get_list().get_for_game(game_name) {
        obs_property_set_enabled(website_button, auto_splitter.website.is_some());

        if !auto_splitter.is_using_auto_splitting_runtime() {
            obs_property_set_enabled(activate_button, false);
            obs_property_set_description(
                info_text,
                cstr!(c"This game's auto splitter is incompatible with LiveSplit One."),
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
            cstr!(c"No auto splitter available for this game."),
        );
    }
}}

#[cfg(feature = "auto-splitting")]
fn auto_splitter_unload(global_timer: &GlobalTimer) {
    global_timer.auto_splitter.unload().ok();

    global_timer
        .auto_splitter_is_enabled
        .store(false, atomic::Ordering::Relaxed);
}

#[cfg(feature = "auto-splitting")]
fn auto_splitter_load(global_timer: &GlobalTimer, path: PathBuf) {
    let enabled = match global_timer
        .auto_splitter
        .load(path, global_timer.timer.clone())
    { Err(e) => {
        warn!("Auto Splitter could not be loaded: {e}");
        false
    } _ => {
        true
    }};

    global_timer
        .auto_splitter_is_enabled
        .store(enabled, atomic::Ordering::Relaxed);
}

#[cfg(feature = "auto-splitting")]
unsafe extern "C" fn auto_splitter_activate_clicked(
    _props: *mut obs_properties_t,
    prop: *mut obs_property_t,
    data: *mut c_void,
) -> bool { unsafe {
    let state: &mut State = &mut (*data.cast::<Mutex<State>>()).lock().unwrap();

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
            state.global_timer.timer.get_timer().run().game_name(),
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
}}

#[cfg(feature = "auto-splitting")]
unsafe fn auto_splitter_update_activation_label(
    activate_button_prop: *mut obs_property_t,
    state: &mut State,
) { unsafe {
    let is_active = state
        .global_timer
        .auto_splitter_is_enabled
        .load(atomic::Ordering::Relaxed);

    obs_property_set_description(
        activate_button_prop,
        if !is_active {
            cstr!(c"Activate")
        } else {
            cstr!(c"Deactivate")
        },
    );
}}

#[cfg(feature = "auto-splitting")]
unsafe extern "C" fn auto_splitter_open_website(
    _props: *mut obs_properties_t,
    _prop: *mut obs_property_t,
    data: *mut c_void,
) -> bool { unsafe {
    let state: &mut State = &mut (*data.cast::<Mutex<State>>()).lock().unwrap();

    let website = auto_splitters::get_list()
        .get_website_for_game(state.global_timer.timer.get_timer().run().game_name());

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
}}

unsafe extern "C" fn media_get_state(data: *mut c_void) -> obs_media_state { unsafe {
    let state: &mut State = &mut (*data.cast::<Mutex<State>>()).lock().unwrap();
    let phase = state.global_timer.timer.get_timer().current_phase();
    match phase {
        TimerPhase::NotRunning => OBS_MEDIA_STATE_STOPPED,
        TimerPhase::Running => OBS_MEDIA_STATE_PLAYING,
        TimerPhase::Ended => OBS_MEDIA_STATE_ENDED,
        TimerPhase::Paused => OBS_MEDIA_STATE_PAUSED,
    }
}}

unsafe extern "C" fn media_play_pause(data: *mut c_void, pause: bool) { unsafe {
    let state: &mut State = &mut (*data.cast::<Mutex<State>>()).lock().unwrap();
    let phase = state.global_timer.timer.get_timer().current_phase();
    match phase {
        TimerPhase::NotRunning => {
            if !pause {
                drop(state.global_timer.timer.start());
            }
        }
        TimerPhase::Running => {
            if pause {
                drop(state.global_timer.timer.pause());
            }
        }
        TimerPhase::Ended => {}
        TimerPhase::Paused => {
            if !pause {
                drop(state.global_timer.timer.resume());
            }
        }
    }
}}

unsafe extern "C" fn media_restart(data: *mut c_void) { unsafe {
    let state: &mut State = &mut (*data.cast::<Mutex<State>>()).lock().unwrap();
    drop(state.global_timer.timer.reset(None));
    drop(state.global_timer.timer.start());
}}

unsafe extern "C" fn media_stop(data: *mut c_void) { unsafe {
    let state: &mut State = &mut (*data.cast::<Mutex<State>>()).lock().unwrap();
    drop(state.global_timer.timer.reset(None));
}}

unsafe extern "C" fn media_next(data: *mut c_void) { unsafe {
    let state: &mut State = &mut (*data.cast::<Mutex<State>>()).lock().unwrap();
    drop(state.global_timer.timer.split());
}}

unsafe extern "C" fn media_previous(data: *mut c_void) { unsafe {
    let state: &mut State = &mut (*data.cast::<Mutex<State>>()).lock().unwrap();
    drop(state.global_timer.timer.undo_split());
}}

unsafe extern "C" fn media_get_time(data: *mut c_void) -> i64 { unsafe {
    let state: &mut State = &mut (*data.cast::<Mutex<State>>()).lock().unwrap();
    let timer = state.global_timer.timer.get_timer();
    let time = timer.snapshot().current_time()[timer.current_timing_method()].unwrap_or_default();
    let (secs, nanos) = time.to_seconds_and_subsec_nanoseconds();
    secs * 1000 + (nanos / 1_000_000) as i64
}}

unsafe extern "C" fn media_get_duration(data: *mut c_void) -> i64 { unsafe {
    let state: &mut State = &mut (*data.cast::<Mutex<State>>()).lock().unwrap();
    let timer = state.global_timer.timer.get_timer();
    let time = timer
        .run()
        .segments()
        .last()
        .unwrap()
        .personal_best_split_time()[timer.current_timing_method()]
    .unwrap_or_default();
    let (secs, nanos) = time.to_seconds_and_subsec_nanoseconds();
    secs * 1000 + (nanos / 1_000_000) as i64
}}

const SETTINGS_WIDTH: *const c_char = cstr!(c"width");
const SETTINGS_HEIGHT: *const c_char = cstr!(c"height");
const SETTINGS_USE_GAME_ARGUMENTS: *const c_char = cstr!(c"game_use_arguments");
const SETTINGS_GAME_PATH: *const c_char = cstr!(c"game_path");
const SETTINGS_GAME_ARGUMENTS: *const c_char = cstr!(c"game_arguments");
const SETTINGS_GAME_WORKING_DIRECTORY: *const c_char = cstr!(c"game_working_directory");
const SETTINGS_GAME_ENVIRONMENT_LIST: *const c_char = cstr!(c"game_environment_list");
const SETTINGS_START_GAME: *const c_char = cstr!(c"start_game");
const SETTINGS_SPLITS_PATH: *const c_char = cstr!(c"splits_path");
const SETTINGS_AUTO_SAVE: *const c_char = cstr!(c"auto_save");
#[cfg(feature = "auto-splitting")]
const SETTINGS_LOCAL_AUTO_SPLITTER: *const c_char = cstr!(c"local_auto_splitter");
#[cfg(feature = "auto-splitting")]
const SETTINGS_LOCAL_AUTO_SPLITTER_PATH: *const c_char = cstr!(c"local_auto_splitter_path");
#[cfg(feature = "auto-splitting")]
const SETTINGS_AUTO_SPLITTER_INFO: *const c_char = cstr!(c"auto_splitter_info");
#[cfg(feature = "auto-splitting")]
const SETTINGS_AUTO_SPLITTER_ACTIVATE: *const c_char = cstr!(c"auto_splitter_activate");
#[cfg(feature = "auto-splitting")]
const SETTINGS_AUTO_SPLITTER_WEBSITE: *const c_char = cstr!(c"auto_splitter_website");
const SETTINGS_LAYOUT_PATH: *const c_char = cstr!(c"layout_path");
const SETTINGS_SAVE_SPLITS: *const c_char = cstr!(c"save_splits");

unsafe extern "C" fn get_properties(data: *mut c_void) -> *mut obs_properties_t { unsafe {
    let state: &mut State = &mut (*data.cast::<Mutex<State>>()).lock().unwrap();

    let props = obs_properties_create();
    obs_properties_add_int(props, SETTINGS_WIDTH, cstr!(c"Width"), 10, 8200, 10);
    obs_properties_add_int(props, SETTINGS_HEIGHT, cstr!(c"Height"), 10, 8200, 10);

    let splits_path = obs_properties_add_path(
        props,
        SETTINGS_SPLITS_PATH,
        cstr!(c"Splits"),
        OBS_PATH_FILE,
        cstr!(c"LiveSplit Splits (*.lss)"),
        ptr::null(),
    );
    obs_data_set_bool(
        state.obs_settings,
        SETTINGS_AUTO_SAVE,
        state
            .global_timer
            .timer
            .auto_save
            .load(atomic::Ordering::Relaxed),
    );
    obs_properties_add_bool(
        props,
        SETTINGS_AUTO_SAVE,
        cstr!(c"Automatically save splits file on reset"),
    );
    obs_properties_add_button(
        props,
        SETTINGS_SAVE_SPLITS,
        cstr!(c"Save Splits"),
        Some(save_splits),
    );

    obs_properties_add_path(
        props,
        SETTINGS_LAYOUT_PATH,
        cstr!(c"Layout"),
        OBS_PATH_FILE,
        cstr!(c"LiveSplit Layouts (*.lsl *.ls1l)"),
        ptr::null(),
    );

    let use_game_arguments = obs_properties_add_bool(
        props,
        SETTINGS_USE_GAME_ARGUMENTS,
        cstr!(c"Advanced start game options"),
    );

    obs_property_set_modified_callback2(
        use_game_arguments,
        Some(use_game_arguments_modified),
        data,
    );

    obs_properties_add_path(
        props,
        SETTINGS_GAME_PATH,
        cstr!(c"Game Path"),
        OBS_PATH_FILE,
        cstr!(c"Executable files (*)"),
        ptr::null(),
    );
    let game_arguments = obs_properties_add_text(
        props,
        SETTINGS_GAME_ARGUMENTS,
        cstr!(c"Game Arguments"),
        OBS_TEXT_DEFAULT,
    );
    let game_working_directory = obs_properties_add_path(
        props,
        SETTINGS_GAME_WORKING_DIRECTORY,
        cstr!(c"Working Directory"),
        OBS_PATH_DIRECTORY,
        cstr!(c"Directories"),
        ptr::null(),
    );
    let game_env_list = obs_properties_add_editable_list(
        props,
        SETTINGS_GAME_ENVIRONMENT_LIST,
        cstr!(c"Game Environment Variables (KEY=VALUE)"),
        OBS_EDITABLE_LIST_TYPE_STRINGS,
        ptr::null(),
        ptr::null(),
    );

    let uses_game_arguments = state.use_game_arguments;
    obs_property_set_visible(game_arguments, uses_game_arguments);
    obs_property_set_visible(game_working_directory, uses_game_arguments);
    obs_property_set_visible(game_env_list, uses_game_arguments);

    obs_properties_add_button(
        props,
        SETTINGS_START_GAME,
        cstr!(c"Start Game"),
        Some(start_game_clicked),
    );

    obs_property_set_modified_callback2(splits_path, Some(splits_path_modified), data);

    #[cfg(feature = "auto-splitting")]
    {
        let use_local_auto_splitter = obs_properties_add_bool(
            props,
            SETTINGS_LOCAL_AUTO_SPLITTER,
            cstr!(c"Use local auto splitter"),
        );

        obs_property_set_modified_callback2(
            use_local_auto_splitter,
            Some(use_local_auto_splitter_modified),
            data,
        );

        let local_auto_splitter_path = obs_properties_add_path(
            props,
            SETTINGS_LOCAL_AUTO_SPLITTER_PATH,
            cstr!(c"Local Auto Splitter File"),
            OBS_PATH_FILE,
            cstr!(c"LiveSplit One Auto Splitter (*.wasm)"),
            ptr::null(),
        );

        let info_text = obs_properties_add_text(
            props,
            SETTINGS_AUTO_SPLITTER_INFO,
            cstr!(c"No splits loaded"),
            OBS_TEXT_INFO,
        );

        let activate_button_text = match state
            .global_timer
            .auto_splitter_is_enabled
            .load(atomic::Ordering::Relaxed)
        {
            true => cstr!(c"Deactivate"),
            false => cstr!(c"Activate"),
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
            cstr!(c"Website"),
            Some(auto_splitter_open_website),
        );

        update_auto_splitter_ui(
            info_text,
            website_button,
            activate_button,
            state.global_timer.timer.get_timer().run().game_name(),
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

        let auto_splitter_properties = obs_properties_create();

        let mut parents = vec![auto_splitter_properties];

        for widget in state.auto_splitter_widgets.iter() {
            let widget_description = CString::new(widget.description.as_ref());

            let setting_key = CString::from_vec_with_nul(
                format!("auto_splitter_setting_{}\0", widget.key).into(),
            );

            if let (Ok(setting_key), Ok(widget_description)) = (setting_key, widget_description) {
                match &widget.kind {
                    WidgetKind::Bool { default_value } => {
                        let property = obs_properties_add_bool(
                            *parents.last().unwrap(),
                            setting_key.as_ptr(),
                            widget_description.as_ptr(),
                        );

                        if let Some(tooltip) = widget
                            .tooltip
                            .as_ref()
                            .and_then(|t| CString::new(t.as_bytes()).ok())
                        {
                            obs_property_set_long_description(property, tooltip.as_ptr());
                        }

                        if let Some(value) = state
                            .auto_splitter_map
                            .get(&widget.key)
                            .and_then(|v| v.to_bool())
                        {
                            obs_data_set_bool(state.obs_settings, setting_key.as_ptr(), value);
                        } else {
                            obs_data_erase(state.obs_settings, setting_key.as_ptr());
                        }

                        obs_data_set_default_bool(
                            state.obs_settings,
                            setting_key.as_ptr(),
                            *default_value,
                        );
                    }
                    WidgetKind::Title { heading_level } => {
                        parents.truncate(*heading_level as usize + 1);

                        let auto_splitter_properties = obs_properties_create();
                        let property = obs_properties_add_group(
                            *parents.last().unwrap(),
                            setting_key.as_ptr(),
                            widget_description.as_ptr(),
                            OBS_GROUP_NORMAL,
                            auto_splitter_properties,
                        );

                        if let Some(tooltip) = widget
                            .tooltip
                            .as_ref()
                            .and_then(|t| CString::new(t.as_bytes()).ok())
                        {
                            obs_property_set_long_description(property, tooltip.as_ptr());
                        }

                        parents.push(auto_splitter_properties);
                    }
                    WidgetKind::Choice {
                        default_option_key,
                        options,
                    } => {
                        let property = obs_properties_add_list(
                            *parents.last().unwrap(),
                            setting_key.as_ptr(),
                            widget_description.as_ptr(),
                            OBS_COMBO_TYPE_LIST,
                            OBS_COMBO_FORMAT_STRING,
                        );

                        if let Some(tooltip) = widget
                            .tooltip
                            .as_ref()
                            .and_then(|t| CString::new(t.as_bytes()).ok())
                        {
                            obs_property_set_long_description(property, tooltip.as_ptr());
                        }

                        for option in &**options {
                            let option_key =
                                CString::from_vec_with_nul(format!("{}\0", option.key).into());
                            let option_description = CString::from_vec_with_nul(
                                format!("{}\0", option.description).into(),
                            );

                            if let (Ok(option_key), Ok(option_description)) =
                                (option_key, option_description)
                            {
                                obs_property_list_add_string(
                                    property,
                                    option_description.as_ptr(),
                                    option_key.as_ptr(),
                                );
                            }
                        }

                        if let Some(value) = state
                            .auto_splitter_map
                            .get(&widget.key)
                            .and_then(|v| v.as_string())
                        {
                            if let Ok(value) =
                                CString::from_vec_with_nul(format!("{}\0", value).into())
                            {
                                obs_data_set_string(
                                    state.obs_settings,
                                    setting_key.as_ptr(),
                                    value.as_ptr(),
                                );
                            }
                        } else {
                            obs_data_erase(state.obs_settings, setting_key.as_ptr());
                        }

                        if let Ok(default_option_key) =
                            CString::from_vec_with_nul(format!("{}\0", default_option_key).into())
                        {
                            obs_data_set_default_string(
                                state.obs_settings,
                                setting_key.as_ptr(),
                                default_option_key.as_ptr(),
                            );
                        }
                    }
                    WidgetKind::FileSelect { filters } => {
                        let mut filter_buf = Vec::new();
                        build_filter(filters, &mut filter_buf);
                        filter_buf.push(b'\0');

                        let property = obs_properties_add_path(
                            *parents.last().unwrap(),
                            setting_key.as_ptr(),
                            widget_description.as_ptr(),
                            OBS_PATH_FILE,
                            filter_buf.as_ptr().cast(),
                            ptr::null(),
                        );

                        if let Some(tooltip) = widget
                            .tooltip
                            .as_ref()
                            .and_then(|t| CString::new(t.as_bytes()).ok())
                        {
                            obs_property_set_long_description(property, tooltip.as_ptr());
                        }

                        match state
                            .auto_splitter_map
                            .get(&widget.key)
                            .and_then(|v| v.as_string())
                            .and_then(|s| wasi_path::to_native(s, true))
                            .filter(|p| p.exists())
                            .and_then(|p| CString::new(p.as_os_str().as_encoded_bytes()).ok())
                        { Some(value) => {
                            obs_data_set_string(
                                state.obs_settings,
                                setting_key.as_ptr(),
                                value.as_ptr(),
                            );
                        } _ => {
                            obs_data_erase(state.obs_settings, setting_key.as_ptr());
                        }}
                    }
                }
            }
        }

        obs_properties_add_group(
            props,
            cstr!(c"auto_splitter_settings_group"),
            cstr!(c"Auto Splitter Settings"),
            OBS_GROUP_NORMAL,
            auto_splitter_properties,
        );
    }

    props
}}

unsafe extern "C" fn get_defaults(settings: *mut obs_data_t) { unsafe {
    obs_data_set_default_int(settings, SETTINGS_WIDTH, 300);
    obs_data_set_default_int(settings, SETTINGS_HEIGHT, 500);
    obs_data_set_default_bool(settings, SETTINGS_AUTO_SAVE, false);
}}

unsafe extern "C" fn activate(data: *mut c_void) { unsafe {
    let state: &mut State = &mut (*data.cast::<Mutex<State>>()).lock().unwrap();
    state.activated = true;
}}

unsafe extern "C" fn deactivate(data: *mut c_void) { unsafe {
    let state: &mut State = &mut (*data.cast::<Mutex<State>>()).lock().unwrap();
    state.activated = false;
}}

fn default_run() -> (Run, bool) {
    let mut run = Run::new();
    run.push_segment(Segment::new("Time"));
    (run, false)
}

unsafe extern "C" fn update(data: *mut c_void, settings_obj: *mut obs_data_t) { unsafe {
    let state: &mut State = &mut (*data.cast::<Mutex<State>>()).lock().unwrap();

    let settings = parse_settings(settings_obj);

    handle_splits_path_change(state, settings.splits_path);

    state.use_game_arguments = settings.use_game_arguments;

    state.game_arguments = settings.game_arguments;
    state.game_working_directory = settings.game_working_directory;
    state.game_environment_vars = settings.game_environment_vars;

    state.game_path = settings.game_path;

    state
        .global_timer
        .timer
        .auto_save
        .store(settings.auto_save, atomic::Ordering::Relaxed);
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

        loop {
            let Some(original) = state.global_timer.auto_splitter.settings_map() else {
                break;
            };
            let mut map = original.clone();

            for widget in state.auto_splitter_widgets.iter() {
                let key = &widget.key;
                let Ok(data_key) = CString::new(format!("auto_splitter_setting_{}", key)) else {
                    continue;
                };

                match &widget.kind {
                    WidgetKind::Title { .. } => {}
                    WidgetKind::Bool { default_value } => {
                        let value = obs_data_get_bool(settings_obj, data_key.as_ptr());
                        if value != *default_value {
                            map.insert(key.clone(), Value::Bool(value));
                        } else {
                            map.remove(key);
                        }
                    }
                    WidgetKind::Choice {
                        default_option_key, ..
                    } => {
                        if let Some(value) =
                            CStr::from_ptr(obs_data_get_string(settings_obj, data_key.as_ptr()))
                                .to_str()
                                .ok()
                                .filter(|v| *v != &**default_option_key)
                        {
                            map.insert(key.clone(), Value::String(Arc::from(value)));
                        } else {
                            map.remove(key);
                        }
                    }
                    WidgetKind::FileSelect { .. } => {
                        if let Some(value) =
                            CStr::from_ptr(obs_data_get_string(settings_obj, data_key.as_ptr()))
                                .to_str()
                                .ok()
                                .filter(|v| !v.is_empty())
                                .and_then(|s| wasi_path::from_native(Path::new(s)))
                        {
                            map.insert(key.clone(), Value::String(Arc::from(value)));
                        } else {
                            map.remove(key);
                        }
                    }
                }
            }

            if state
                .global_timer
                .auto_splitter
                .set_settings_map_if_unchanged(&original, map.clone())
                != Some(false)
            {
                state.auto_splitter_map = map;
                break;
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
}}

fn handle_splits_path_change(state: &mut State, splits_path: PathBuf) {
    state.global_timer = get_global_timer(splits_path);
}

fn get_global_timer(splits_path: PathBuf) -> Arc<GlobalTimer> {
    let mut timers = TIMERS.lock().unwrap();
    timers.retain(|timer| timer.strong_count() > 0);
    match timers.iter().find_map(|timer| {
        let timer = timer.upgrade()?;
        if timer.timer.path == splits_path {
            Some(timer)
        } else {
            None
        }
    }) { Some(timer) => {
        debug!("Found timer to reuse.");
        timer
    } _ => {
        debug!("Storing timer for reuse.");
        let (run, can_save_splits) = parse_run(&splits_path).unwrap_or_else(default_run);
        let timer = Timer::new(run).unwrap();
        #[cfg(feature = "auto-splitting")]
        let auto_splitter = auto_splitting::Runtime::new();
        let global_timer = Arc::new(GlobalTimer {
            timer: Arc::new(InnerTimer {
                timer: RwLock::new(timer),
                auto_save: AtomicBool::new(false),
                path: splits_path,
                can_save_splits,
            }),
            #[cfg(feature = "auto-splitting")]
            auto_splitter,
            #[cfg(feature = "auto-splitting")]
            auto_splitter_is_enabled: AtomicBool::new(false),
        });
        timers.push(Arc::downgrade(&global_timer));
        global_timer
    }}
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

#[unsafe(no_mangle)]
pub extern "C" fn obs_module_load() -> bool {
    static SOURCE_INFO: UnsafeMultiThread<obs_source_info> = UnsafeMultiThread(obs_source_info {
        id: cstr!(c"livesplit-one"),
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

    std::panic::set_hook(Box::new(|info| {
        error!("obs-livesplit-one crashed, please report this:\n{info}");
    }));

    let source_info: &obs_source_info = &SOURCE_INFO.0;

    unsafe {
        obs_register_source_s(source_info, mem::size_of_val(source_info) as _);
    }

    #[cfg(feature = "auto-splitting")]
    auto_splitters::set_up();

    true
}

#[cfg(feature = "auto-splitting")]
fn build_filter(filters: &[FileFilter], output: &mut Vec<u8>) {
    for filter in filters.iter() {
        match filter {
            FileFilter::Name {
                description,
                pattern,
            } => {
                if pattern.contains(";;") {
                    continue;
                }
                if !output.is_empty() {
                    output.extend_from_slice(b";;");
                }
                match &description {
                    Some(description) => {
                        output.extend(
                            description
                                .trim()
                                .split(";;")
                                .flat_map(|s| s.bytes())
                                .filter(|b| *b != b'(' && *b != b')'),
                        );
                        output.extend_from_slice(b" (");
                    }
                    None => {
                        let mime = pattern.split(' ').find_map(|pat| {
                            let (name, ext) = pat.rsplit_once('.')?;
                            if name != "*" {
                                return None;
                            }
                            if ext.contains('*') {
                                return None;
                            }
                            mime_guess::from_ext(ext).first()
                        });
                        if let Some(mime) = mime {
                            append_mime_desc(
                                mime.type_().as_str(),
                                mime.subtype().as_str(),
                                output,
                            );
                        } else {
                            let mut ext_count = 0;

                            let only_contains_extensions = pattern.split(' ').all(|pat| {
                                ext_count += 1;
                                let Some((name, ext)) = pat.rsplit_once('.') else {
                                    return false;
                                };
                                name == "*" && !ext.contains('*')
                            });

                            if only_contains_extensions {
                                let mut char_buf = [0; 4];

                                for (i, ext) in pattern
                                    .split(' ')
                                    .filter_map(|pat| {
                                        let (_, ext) = pat.rsplit_once('.')?;
                                        Some(ext)
                                    })
                                    .enumerate()
                                {
                                    if i != 0 {
                                        output.extend_from_slice(if i + 1 != ext_count {
                                            b", "
                                        } else {
                                            b" or "
                                        });
                                    }

                                    for c in ext
                                        .chars()
                                        .flat_map(|c| c.to_uppercase())
                                        .filter(|c| *c != '(' && *c != ')')
                                    {
                                        output.extend_from_slice(
                                            c.encode_utf8(&mut char_buf).as_bytes(),
                                        );
                                    }
                                }

                                output.extend_from_slice(b" files (");
                            } else {
                                output.extend(
                                    pattern.trim().bytes().filter(|&c| c != b'(' && c != b')'),
                                );
                                output.extend_from_slice(b" (");
                            }
                        }
                    }
                }

                for (i, pattern) in pattern.split(' ').enumerate() {
                    if i != 0 {
                        output.push(b' ');
                    }
                    output.extend_from_slice(pattern.as_bytes());
                }
            }
            FileFilter::MimeType(mime_type) => {
                let Some((top, sub)) = mime_type.split_once('/') else {
                    continue;
                };
                if top == "*" {
                    continue;
                }
                let Some(extensions) = mime_guess::get_extensions(top, sub) else {
                    continue;
                };

                if !output.is_empty() {
                    output.extend_from_slice(b";;");
                }

                append_mime_desc(top, sub, output);

                for (i, extension) in extensions.iter().enumerate() {
                    if i != 0 {
                        output.push(b' ');
                    }
                    output.extend_from_slice(b"*.");
                    output.extend_from_slice(extension.as_bytes());
                }
            }
        }

        output.push(b')');
    }

    if !output.is_empty() {
        output.extend_from_slice(b";;");
    }
    output.extend_from_slice(b"All files (*.*)");
}

#[cfg(feature = "auto-splitting")]
fn append_mime_desc(top: &str, sub: &str, output: &mut Vec<u8>) {
    let mut char_buf = [0; 4];

    if sub != "*" {
        // Strip vendor and x- prefixes

        let sub = sub
            .strip_prefix("vnd.")
            .unwrap_or(sub)
            .strip_prefix("x-")
            .unwrap_or(sub);

        // Capitalize the first letter

        let mut chars = sub.chars();
        if let Some(c) = chars
            .by_ref()
            .map(|c| match c {
                '-' | '.' | '+' | '|' | '(' | ')' => ' ',
                _ => c,
            })
            .next()
        {
            for c in c.to_uppercase() {
                output.extend_from_slice(c.encode_utf8(&mut char_buf).as_bytes());
            }
        }

        // Only capitalize chunks of the rest that are 4 characters or less as a
        // heuristic to detect acronyms

        let rem = chars.as_str();
        for (i, piece) in rem.split(&['-', '.', '+', '|', ' ', '(', ')']).enumerate() {
            if i != 0 {
                output.push(b' ');
            }
            if piece.len() <= 4 - (i == 0) as usize {
                for c in piece.chars() {
                    for c in c.to_uppercase() {
                        output.extend_from_slice(c.encode_utf8(&mut char_buf).as_bytes());
                    }
                }
            } else {
                output.extend_from_slice(piece.as_bytes());
            }
        }

        output.push(b' ');
    }

    let mut chars = top.chars();
    if sub == "*" {
        if let Some(c) = chars.by_ref().find(|c| *c != '(' && *c != ')') {
            for c in c.to_uppercase() {
                output.extend_from_slice(c.encode_utf8(&mut char_buf).as_bytes());
            }
        }
    }
    output.extend(chars.as_str().bytes().filter(|b| *b != b'(' && *b != b')'));
    output.extend_from_slice(if top == "image" { b"s (" } else { b" files (" });
}
