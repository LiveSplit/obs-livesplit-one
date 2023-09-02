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
    os::raw::{c_char, c_int, c_longlong},
};

pub use crate::ffi_types::*;

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
    pub fn obs_properties_add_bool(
        props: *mut obs_properties_t,
        name: *const c_char,
        description: *const c_char,
    ) -> *mut obs_property_t;
    pub fn obs_data_get_bool(data: *mut obs_data_t, name: *const c_char) -> bool;
    #[cfg(feature = "auto-splitting")]
    pub fn obs_properties_add_text(
        props: *mut obs_properties_t,
        name: *const c_char,
        description: *const c_char,
        text_type: obs_text_type,
    ) -> *mut obs_property_t;
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
    pub fn obs_data_set_default_bool(data: *mut obs_data_t, name: *const c_char, val: bool);
    pub fn obs_data_set_default_int(data: *mut obs_data_t, name: *const c_char, val: c_longlong);
    pub fn obs_properties_add_button(
        props: *mut obs_properties_t,
        name: *const c_char,
        text: *const c_char,
        callback: obs_property_clicked_t,
    ) -> *mut obs_property_t;
    pub fn obs_property_set_modified_callback2(
        prop: *mut obs_property_t,
        modified2_callback: obs_property_modified2_t,
        private: *mut c_void,
    );
    #[cfg(feature = "auto-splitting")]
    pub fn obs_property_set_description(prop: *mut obs_property_t, description: *const c_char);
    #[cfg(feature = "auto-splitting")]
    pub fn obs_property_set_enabled(prop: *mut obs_property_t, enabled: bool);
    #[cfg(feature = "auto-splitting")]
    pub fn obs_property_set_visible(prop: *mut obs_property_t, visible: bool);
    #[cfg(feature = "auto-splitting")]
    pub fn obs_properties_get(
        props: *mut obs_properties_t,
        prop: *const c_char,
    ) -> *mut obs_property_t;
    pub fn obs_module_get_config_path(
        module: *mut obs_module_t,
        file: *const c_char,
    ) -> *const c_char;
}
