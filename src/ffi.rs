// Based on https://docs.rs/crate/obs-sys/0.1.3/source/generated/bindings.rs
// There's a few changes:
// - enums used to be prefixed by the type. But they already are anyway.
//   So obs_icon_type_OBS_ICON_TYPE_GAME_CAPTURE is just OBS_ICON_TYPE_GAME_CAPTURE
// - The functions properly link against libobs.
// - No dependency on bindgen, which just causes trouble.
// - size_t is usize, not ulong, which would be u32 on 64-bit Windows.

#![allow(non_camel_case_types)]

use std::{
    ffi::c_void,
    os::raw::{c_char, c_int, c_long, c_longlong},
};

pub type gs_color_format = u32;
pub const GS_RGBA: gs_color_format = 3;
pub type gs_effect_t = gs_effect;

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct gs_effect {
    _unused: [u8; 0],
}

pub type gs_eparam_t = gs_effect_param;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct gs_effect_param {
    _unused: [u8; 0],
}

pub type gs_technique_t = gs_effect_technique;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct gs_effect_technique {
    _unused: [u8; 0],
}

pub type gs_texture_t = gs_texture;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct gs_texture {
    _unused: [u8; 0],
}

pub type obs_base_effect = u32;
pub const OBS_EFFECT_PREMULTIPLIED_ALPHA: obs_base_effect = 7;

pub type obs_data_t = obs_data;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct obs_data {
    _unused: [u8; 0],
}

pub type obs_hotkey_func = Option<
    unsafe extern "C" fn(
        data: *mut c_void,
        id: obs_hotkey_id,
        hotkey: *mut obs_hotkey_t,
        pressed: bool,
    ),
>;

pub type size_t = usize;
pub type obs_hotkey_id = size_t;

pub type obs_hotkey_t = obs_hotkey;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct obs_hotkey {
    _unused: [u8; 0],
}

pub const OBS_ICON_TYPE_GAME_CAPTURE: obs_icon_type = 8;
pub type obs_icon_type = u32;

pub type obs_module_t = obs_module;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct obs_module {
    _unused: [u8; 0],
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone)]
pub struct obs_mouse_event {
    pub modifiers: u32,
    pub x: i32,
    pub y: i32,
}

pub type obs_path_type = u32;
pub const OBS_PATH_FILE: obs_path_type = 0;

pub type obs_properties_t = obs_properties;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct obs_properties {
    _unused: [u8; 0],
}

pub type obs_property_clicked_t = Option<
    unsafe extern "C" fn(
        props: *mut obs_properties_t,
        property: *mut obs_property_t,
        data: *mut c_void,
    ) -> bool,
>;

pub type obs_property_t = obs_property;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct obs_property {
    _unused: [u8; 0],
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct obs_source_info {
    pub id: *const c_char,
    pub type_: obs_source_type,
    pub output_flags: u32,
    pub get_name: Option<unsafe extern "C" fn(type_data: *mut c_void) -> *const c_char>,
    pub create: Option<
        unsafe extern "C" fn(settings: *mut obs_data_t, source: *mut obs_source_t) -> *mut c_void,
    >,
    pub destroy: Option<unsafe extern "C" fn(data: *mut c_void)>,
    pub get_width: Option<unsafe extern "C" fn(data: *mut c_void) -> u32>,
    pub get_height: Option<unsafe extern "C" fn(data: *mut c_void) -> u32>,
    pub get_defaults: Option<unsafe extern "C" fn(settings: *mut obs_data_t)>,
    pub get_properties: Option<unsafe extern "C" fn(data: *mut c_void) -> *mut obs_properties_t>,
    pub update: Option<unsafe extern "C" fn(data: *mut c_void, settings: *mut obs_data_t)>,
    pub activate: Option<unsafe extern "C" fn(data: *mut c_void)>,
    pub deactivate: Option<unsafe extern "C" fn(data: *mut c_void)>,
    pub show: Option<unsafe extern "C" fn(data: *mut c_void)>,
    pub hide: Option<unsafe extern "C" fn(data: *mut c_void)>,
    pub video_tick: Option<unsafe extern "C" fn(data: *mut c_void, seconds: f32)>,
    pub video_render: Option<unsafe extern "C" fn(data: *mut c_void, effect: *mut gs_effect_t)>,
    pub filter_video: Option<
        unsafe extern "C" fn(
            data: *mut c_void,
            frame: *mut obs_source_frame,
        ) -> *mut obs_source_frame,
    >,
    pub filter_audio: Option<
        unsafe extern "C" fn(data: *mut c_void, audio: *mut obs_audio_data) -> *mut obs_audio_data,
    >,
    pub enum_active_sources: Option<
        unsafe extern "C" fn(
            data: *mut c_void,
            enum_callback: obs_source_enum_proc_t,
            param: *mut c_void,
        ),
    >,
    pub save: Option<unsafe extern "C" fn(data: *mut c_void, settings: *mut obs_data_t)>,
    pub load: Option<unsafe extern "C" fn(data: *mut c_void, settings: *mut obs_data_t)>,
    pub mouse_click: Option<
        unsafe extern "C" fn(
            data: *mut c_void,
            event: *const obs_mouse_event,
            type_: i32,
            mouse_up: bool,
            click_count: u32,
        ),
    >,
    pub mouse_move: Option<
        unsafe extern "C" fn(data: *mut c_void, event: *const obs_mouse_event, mouse_leave: bool),
    >,
    pub mouse_wheel: Option<
        unsafe extern "C" fn(
            data: *mut c_void,
            event: *const obs_mouse_event,
            x_delta: c_int,
            y_delta: c_int,
        ),
    >,
    pub focus: Option<unsafe extern "C" fn(data: *mut c_void, focus: bool)>,
    pub key_click:
        Option<unsafe extern "C" fn(data: *mut c_void, event: *const obs_key_event, key_up: bool)>,
    pub filter_remove: Option<unsafe extern "C" fn(data: *mut c_void, source: *mut obs_source_t)>,
    pub type_data: *mut c_void,
    pub free_type_data: Option<unsafe extern "C" fn(type_data: *mut c_void)>,
    pub audio_render: Option<
        unsafe extern "C" fn(
            data: *mut c_void,
            ts_out: *mut u64,
            audio_output: *mut obs_source_audio_mix,
            mixers: u32,
            channels: size_t,
            sample_rate: size_t,
        ) -> bool,
    >,
    pub enum_all_sources: Option<
        unsafe extern "C" fn(
            data: *mut c_void,
            enum_callback: obs_source_enum_proc_t,
            param: *mut c_void,
        ),
    >,
    pub transition_start: Option<unsafe extern "C" fn(data: *mut c_void)>,
    pub transition_stop: Option<unsafe extern "C" fn(data: *mut c_void)>,
    pub get_defaults2:
        Option<unsafe extern "C" fn(type_data: *mut c_void, settings: *mut obs_data_t)>,
    pub get_properties2: Option<
        unsafe extern "C" fn(data: *mut c_void, type_data: *mut c_void) -> *mut obs_properties_t,
    >,
    pub audio_mix: Option<
        unsafe extern "C" fn(
            data: *mut c_void,
            ts_out: *mut u64,
            audio_output: *mut audio_output_data,
            channels: size_t,
            sample_rate: size_t,
        ) -> bool,
    >,
    pub icon_type: obs_icon_type,
    pub media_play_pause: Option<unsafe extern "C" fn(data: *mut c_void, pause: bool)>,
    pub media_restart: Option<unsafe extern "C" fn(data: *mut c_void)>,
    pub media_stop: Option<unsafe extern "C" fn(data: *mut c_void)>,
    pub media_next: Option<unsafe extern "C" fn(data: *mut c_void)>,
    pub media_previous: Option<unsafe extern "C" fn(data: *mut c_void)>,
    pub media_get_duration: Option<unsafe extern "C" fn(data: *mut c_void) -> i64>,
    pub media_get_time: Option<unsafe extern "C" fn(data: *mut c_void) -> i64>,
    pub media_set_time: Option<unsafe extern "C" fn(data: *mut c_void, miliseconds: i64)>,
    pub media_get_state: Option<unsafe extern "C" fn(data: *mut c_void) -> obs_media_state>,
    pub version: u32,
    pub unversioned_id: *const c_char,
}

pub type obs_source_type = u32;
pub const OBS_SOURCE_TYPE_INPUT: obs_source_type = 0;

pub type obs_source_t = obs_source;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct obs_source {
    _unused: [u8; 0],
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct obs_source_frame {
    pub data: [*mut u8; 8usize],
    pub linesize: [u32; 8usize],
    pub width: u32,
    pub height: u32,
    pub timestamp: u64,
    pub format: video_format,
    pub color_matrix: [f32; 16usize],
    pub full_range: bool,
    pub color_range_min: [f32; 3usize],
    pub color_range_max: [f32; 3usize],
    pub flip: bool,
    pub refs: c_long,
    pub prev_frame: bool,
}

pub type video_format = u32;

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct obs_audio_data {
    pub data: [*mut u8; 8usize],
    pub frames: u32,
    pub timestamp: u64,
}

pub type obs_source_enum_proc_t = Option<
    unsafe extern "C" fn(parent: *mut obs_source_t, child: *mut obs_source_t, param: *mut c_void),
>;

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct obs_key_event {
    pub modifiers: u32,
    pub text: *mut c_char,
    pub native_modifiers: u32,
    pub native_scancode: u32,
    pub native_vkey: u32,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct obs_source_audio_mix {
    pub output: [audio_output_data; 6usize],
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct audio_output_data {
    pub data: [*mut f32; 8usize],
}

pub type obs_media_state = u32;

pub const GS_DYNAMIC: u32 = 2;

pub type _bindgen_ty_1 = u32;
pub const LOG_WARNING: _bindgen_ty_1 = 200;

pub const OBS_SOURCE_CUSTOM_DRAW: u32 = 8;
pub const OBS_SOURCE_INTERACTION: u32 = 32;
pub const OBS_SOURCE_VIDEO: u32 = 1;

#[link(name = "obs", kind = "dylib")]
extern "C" {
    pub fn obs_register_source_s(info: *const obs_source_info, size: size_t);
    pub fn gs_texture_create(
        width: u32,
        height: u32,
        color_format: gs_color_format,
        levels: u32,
        data: *mut *const u8,
        flags: u32,
    ) -> *mut gs_texture_t;
    pub fn obs_enter_graphics();
    pub fn obs_leave_graphics();
    pub fn gs_texture_set_image(
        tex: *mut gs_texture_t,
        data: *const u8,
        linesize: u32,
        invert: bool,
    );
    pub fn obs_hotkey_register_source(
        source: *mut obs_source_t,
        name: *const c_char,
        description: *const c_char,
        func: obs_hotkey_func,
        data: *mut c_void,
    ) -> obs_hotkey_id;
    pub fn obs_properties_create() -> *mut obs_properties_t;
    pub fn obs_properties_add_path(
        props: *mut obs_properties_t,
        name: *const c_char,
        description: *const c_char,
        type_: obs_path_type,
        filter: *const c_char,
        default_path: *const c_char,
    ) -> *mut obs_property_t;
    pub fn obs_data_get_string(data: *mut obs_data_t, name: *const c_char) -> *const c_char;
    pub fn blog(log_level: c_int, format: *const c_char, ...);
    pub fn obs_properties_add_int(
        props: *mut obs_properties_t,
        name: *const c_char,
        description: *const c_char,
        min: c_int,
        max: c_int,
        step: c_int,
    ) -> *mut obs_property_t;
    pub fn obs_data_get_int(data: *mut obs_data_t, name: *const c_char) -> c_longlong;
    pub fn gs_texture_destroy(tex: *mut gs_texture_t);
    pub fn gs_draw_sprite(tex: *mut gs_texture_t, flip: u32, width: u32, height: u32);
    pub fn gs_effect_get_param_by_name(
        effect: *const gs_effect_t,
        name: *const c_char,
    ) -> *mut gs_eparam_t;
    pub fn gs_effect_get_technique(
        effect: *const gs_effect_t,
        name: *const c_char,
    ) -> *mut gs_technique_t;
    pub fn gs_effect_set_texture(param: *mut gs_eparam_t, val: *mut gs_texture_t);
    pub fn gs_technique_begin(technique: *mut gs_technique_t) -> size_t;
    pub fn gs_technique_begin_pass(technique: *mut gs_technique_t, pass: size_t) -> bool;
    pub fn gs_technique_end(technique: *mut gs_technique_t);
    pub fn gs_technique_end_pass(technique: *mut gs_technique_t);
    pub fn obs_get_base_effect(effect: obs_base_effect) -> *mut gs_effect_t;
    pub fn obs_data_set_default_int(data: *mut obs_data_t, name: *const c_char, val: c_longlong);
    pub fn obs_properties_add_button(
        props: *mut obs_properties_t,
        name: *const c_char,
        text: *const c_char,
        callback: obs_property_clicked_t,
    ) -> *mut obs_property_t;
}
