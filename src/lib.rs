use std::{
    cmp::Ordering,
    ffi::{c_void, CStr},
    fs::File,
    io::{BufReader, BufWriter, Seek, SeekFrom},
    mem,
    os::raw::{c_char, c_int},
    path::PathBuf,
    ptr,
};

mod ffi;
mod ffi_types;

use ffi::{
    gs_draw_sprite, gs_effect_get_param_by_name, gs_effect_get_technique, gs_effect_set_texture,
    gs_effect_t, gs_technique_begin, gs_technique_begin_pass, gs_technique_end,
    gs_technique_end_pass, gs_texture_create, gs_texture_destroy, gs_texture_set_image,
    gs_texture_t, obs_data_get_int, obs_data_get_string, obs_data_set_default_int, obs_data_t,
    obs_enter_graphics, obs_get_base_effect, obs_hotkey_id, obs_hotkey_register_source,
    obs_hotkey_t, obs_leave_graphics, obs_module_t, obs_mouse_event, obs_properties_add_button,
    obs_properties_add_int, obs_properties_add_path, obs_properties_create, obs_properties_t,
    obs_property_t, obs_register_source_s, obs_source_info, obs_source_t, GS_DYNAMIC, GS_RGBA,
    OBS_EFFECT_PREMULTIPLIED_ALPHA, OBS_ICON_TYPE_GAME_CAPTURE, OBS_PATH_FILE,
    OBS_SOURCE_CUSTOM_DRAW, OBS_SOURCE_INTERACTION, OBS_SOURCE_TYPE_INPUT, OBS_SOURCE_VIDEO,
};
use livesplit_core::{
    layout::{self, LayoutSettings, LayoutState},
    rendering::software::SoftwareRenderer,
    run::{parser::composite, saver::livesplit::save_timer},
    Layout, Run, Segment, Timer,
};
use once_cell::sync::Lazy;

macro_rules! cstr {
    ($f:literal) => {
        concat!($f, '\0').as_ptr().cast()
    };
}

static mut OBS_MODULE_POINTER: *mut obs_module_t = ptr::null_mut();

#[no_mangle]
pub extern "C" fn obs_module_set_pointer(module: *mut obs_module_t) {
    unsafe {
        OBS_MODULE_POINTER = module;
    }
}

#[no_mangle]
pub extern "C" fn obs_module_ver() -> u32 {
    (26 << 24) | (1 << 16) | 1
}

struct UnsafeMultiThread<T>(T);

unsafe impl<T> Sync for UnsafeMultiThread<T> {}
unsafe impl<T> Send for UnsafeMultiThread<T> {}

struct State {
    timer: Timer,
    layout: Layout,
    state: LayoutState,
    renderer: SoftwareRenderer,
    texture: *mut gs_texture_t,
    width: u32,
    height: u32,
}

struct Settings {
    run: Run,
    layout: Layout,
    width: u32,
    height: u32,
}

fn parse_run(path: &CStr) -> Option<Run> {
    let path = path.to_str().ok()?;
    if path.is_empty() {
        return None;
    }
    let reader = BufReader::new(File::open(path).ok()?);
    let run = composite::parse(reader, Some(PathBuf::from(path)), true).ok()?;
    if run.run.is_empty() {
        return None;
    }
    Some(run.run)
}

// fn log(x: fmt::Arguments<'_>) {
//     let str = format!("{}\0", x);
//     unsafe {
//         blog(LOG_WARNING as _, str.as_ptr().cast());
//     }
// }

fn parse_layout(path: &CStr) -> Option<Layout> {
    let path = path.to_str().ok()?;
    if path.is_empty() {
        return None;
    }
    let mut reader = BufReader::new(File::open(path).ok()?);

    if let Ok(settings) = LayoutSettings::from_json(&mut reader) {
        return Some(Layout::from_settings(settings));
    }

    reader.seek(SeekFrom::Start(0)).ok()?;
    layout::parser::parse(&mut reader).ok()
}

unsafe fn parse_settings(settings: *mut obs_data_t) -> Settings {
    let splits_path = CStr::from_ptr(obs_data_get_string(settings, SETTINGS_SPLITS_PATH).cast());
    let run = parse_run(splits_path).unwrap_or_else(default_run);

    let layout_path = CStr::from_ptr(obs_data_get_string(settings, SETTINGS_LAYOUT_PATH).cast());
    let layout = parse_layout(layout_path).unwrap_or_else(Layout::default_layout);

    let width = obs_data_get_int(settings, SETTINGS_WIDTH) as u32;
    let height = obs_data_get_int(settings, SETTINGS_HEIGHT) as u32;

    Settings {
        run,
        layout,
        width,
        height,
    }
}

impl State {
    unsafe fn new(
        Settings {
            run,
            layout,
            width,
            height,
        }: Settings,
    ) -> Self {
        let timer = Timer::new(run).unwrap();
        let state = LayoutState::default();
        let renderer = SoftwareRenderer::new();

        obs_enter_graphics();
        let texture = gs_texture_create(width, height, GS_RGBA, 1, ptr::null_mut(), GS_DYNAMIC);
        obs_leave_graphics();

        Self {
            timer,
            layout,
            state,
            renderer,
            texture,
            width,
            height,
        }
    }

    unsafe fn update(&mut self) {
        self.layout
            .update_state(&mut self.state, &self.timer.snapshot());

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
    if pressed {
        let state: &mut State = &mut *data.cast();
        state.timer.split_or_start();
    }
}

unsafe extern "C" fn reset(
    data: *mut c_void,
    _: obs_hotkey_id,
    _: *mut obs_hotkey_t,
    pressed: bool,
) {
    if pressed {
        let state: &mut State = &mut *data.cast();
        state.timer.reset(true);
    }
}

unsafe extern "C" fn undo(
    data: *mut c_void,
    _: obs_hotkey_id,
    _: *mut obs_hotkey_t,
    pressed: bool,
) {
    if pressed {
        let state: &mut State = &mut *data.cast();
        state.timer.undo_split();
    }
}

unsafe extern "C" fn skip(
    data: *mut c_void,
    _: obs_hotkey_id,
    _: *mut obs_hotkey_t,
    pressed: bool,
) {
    if pressed {
        let state: &mut State = &mut *data.cast();
        state.timer.skip_split();
    }
}

unsafe extern "C" fn pause(
    data: *mut c_void,
    _: obs_hotkey_id,
    _: *mut obs_hotkey_t,
    pressed: bool,
) {
    if pressed {
        let state: &mut State = &mut *data.cast();
        state.timer.toggle_pause_or_start();
    }
}

unsafe extern "C" fn undo_all_pauses(
    data: *mut c_void,
    _: obs_hotkey_id,
    _: *mut obs_hotkey_t,
    pressed: bool,
) {
    if pressed {
        let state: &mut State = &mut *data.cast();
        state.timer.undo_all_pauses();
    }
}

unsafe extern "C" fn previous_comparison(
    data: *mut c_void,
    _: obs_hotkey_id,
    _: *mut obs_hotkey_t,
    pressed: bool,
) {
    if pressed {
        let state: &mut State = &mut *data.cast();
        state.timer.switch_to_previous_comparison();
    }
}

unsafe extern "C" fn next_comparison(
    data: *mut c_void,
    _: obs_hotkey_id,
    _: *mut obs_hotkey_t,
    pressed: bool,
) {
    if pressed {
        let state: &mut State = &mut *data.cast();
        state.timer.switch_to_next_comparison();
    }
}

unsafe extern "C" fn toggle_timing_method(
    data: *mut c_void,
    _: obs_hotkey_id,
    _: *mut obs_hotkey_t,
    pressed: bool,
) {
    if pressed {
        let state: &mut State = &mut *data.cast();
        state.timer.toggle_timing_method();
    }
}

unsafe extern "C" fn create(settings: *mut obs_data_t, source: *mut obs_source_t) -> *mut c_void {
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
    state.update();

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
    if let Some(path) = state.timer.run().path() {
        if let Ok(file) = File::create(path) {
            let _ = save_timer(&state.timer, BufWriter::new(file));
        }
    }
    false
}

const SETTINGS_WIDTH: *const c_char = cstr!("width");
const SETTINGS_HEIGHT: *const c_char = cstr!("height");
const SETTINGS_SPLITS_PATH: *const c_char = cstr!("splits_path");
const SETTINGS_LAYOUT_PATH: *const c_char = cstr!("layout_path");
const SETTINGS_SAVE_SPLITS: *const c_char = cstr!("save_splits");

unsafe extern "C" fn get_properties(_: *mut c_void) -> *mut obs_properties_t {
    let props = obs_properties_create();
    obs_properties_add_int(props, SETTINGS_WIDTH, cstr!("Width"), 10, 8200, 10);
    obs_properties_add_int(props, SETTINGS_HEIGHT, cstr!("Height"), 10, 8200, 10);
    obs_properties_add_path(
        props,
        SETTINGS_SPLITS_PATH,
        cstr!("Splits"),
        OBS_PATH_FILE,
        cstr!("LiveSplit Splits (*.lss)"),
        ptr::null(),
    );
    obs_properties_add_path(
        props,
        SETTINGS_LAYOUT_PATH,
        cstr!("Layout"),
        OBS_PATH_FILE,
        cstr!("LiveSplit Layouts (*.lsl *.ls1l)"),
        ptr::null(),
    );
    obs_properties_add_button(
        props,
        SETTINGS_SAVE_SPLITS,
        cstr!("Save Splits"),
        Some(save_splits),
    );
    props
}

unsafe extern "C" fn get_defaults(settings: *mut obs_data_t) {
    obs_data_set_default_int(settings, SETTINGS_WIDTH, 300);
    obs_data_set_default_int(settings, SETTINGS_HEIGHT, 500);
}

fn default_run() -> Run {
    let mut run = Run::new();
    run.push_segment(Segment::new("Time"));
    run
}

unsafe extern "C" fn update(data: *mut c_void, settings: *mut obs_data_t) {
    let state: &mut State = &mut *data.cast();
    let settings = parse_settings(settings);
    state.timer.set_run(settings.run).unwrap();
    state.layout = settings.layout;
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

#[no_mangle]
pub extern "C" fn obs_module_load() -> bool {
    static SOURCE_INFO: Lazy<UnsafeMultiThread<obs_source_info>> = Lazy::new(|| {
        UnsafeMultiThread(unsafe {
            obs_source_info {
                id: cstr!("livesplit-one"),
                type_: OBS_SOURCE_TYPE_INPUT,
                output_flags: OBS_SOURCE_VIDEO | OBS_SOURCE_CUSTOM_DRAW | OBS_SOURCE_INTERACTION,
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
                ..mem::zeroed()
            }
        })
    });

    let source_info: &obs_source_info = &SOURCE_INFO.0;

    unsafe {
        obs_register_source_s(source_info, mem::size_of_val(source_info) as _);
    }
    true
}
