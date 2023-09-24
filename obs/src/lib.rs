use std::{
    ffi::c_void,
    os::raw::{c_char, c_int, c_longlong},
};

#[path = "../../src/ffi_types.rs"]
mod ffi_types;

use ffi_types::*;

#[no_mangle]
pub extern "C" fn obs_register_source_s(_info: *const obs_source_info, _size: size_t) {
    panic!()
}

#[no_mangle]
pub extern "C" fn gs_texture_create(
    _width: u32,
    _height: u32,
    _color_format: gs_color_format,
    _levels: u32,
    _data: *mut *const u8,
    _flags: u32,
) -> *mut gs_texture_t {
    panic!()
}

#[no_mangle]
pub extern "C" fn obs_enter_graphics() {
    panic!()
}

#[no_mangle]
pub extern "C" fn obs_leave_graphics() {
    panic!()
}

#[no_mangle]
pub extern "C" fn gs_texture_set_image(
    _tex: *mut gs_texture_t,
    _data: *const u8,
    _linesize: u32,
    _invert: bool,
) {
    panic!()
}

#[no_mangle]
pub extern "C" fn obs_hotkey_register_source(
    _source: *mut obs_source_t,
    _name: *const c_char,
    _description: *const c_char,
    _func: obs_hotkey_func,
    _data: *mut c_void,
) -> obs_hotkey_id {
    panic!()
}

#[no_mangle]
pub extern "C" fn obs_properties_create() -> *mut obs_properties_t {
    panic!()
}

#[no_mangle]
pub extern "C" fn obs_properties_add_path(
    _props: *mut obs_properties_t,
    _name: *const c_char,
    _description: *const c_char,
    _type_: obs_path_type,
    _filter: *const c_char,
    _default_path: *const c_char,
) -> *mut obs_property_t {
    panic!()
}

#[no_mangle]
pub extern "C" fn obs_data_get_string(
    _data: *mut obs_data_t,
    _name: *const c_char,
) -> *const c_char {
    panic!()
}

// TODO: This technically should take a varargs ... argument, but that's not stable.
#[no_mangle]
pub extern "C" fn blog(_log_level: c_int, _format: *const c_char) {
    panic!()
}

#[no_mangle]
pub extern "C" fn obs_properties_add_bool(
    _props: *mut obs_properties_t,
    _name: *const c_char,
    _description: *const c_char,
) -> *mut obs_property_t {
    panic!()
}

#[no_mangle]
pub extern "C" fn obs_data_get_bool(_data: *mut obs_data_t, _name: *const c_char) -> bool {
    panic!()
}

#[no_mangle]
pub extern "C" fn obs_properties_add_int(
    _props: *mut obs_properties_t,
    _name: *const c_char,
    _description: *const c_char,
    _min: c_int,
    _max: c_int,
    _step: c_int,
) -> *mut obs_property_t {
    panic!()
}

#[no_mangle]
pub extern "C" fn obs_data_get_int(_data: *mut obs_data_t, _name: *const c_char) -> c_longlong {
    panic!()
}

#[no_mangle]
pub extern "C" fn gs_texture_destroy(_tex: *mut gs_texture_t) {
    panic!()
}

#[no_mangle]
pub extern "C" fn gs_draw_sprite(_tex: *mut gs_texture_t, _flip: u32, _width: u32, _height: u32) {
    panic!()
}

#[no_mangle]
pub extern "C" fn gs_effect_get_param_by_name(
    _effect: *const gs_effect_t,
    _name: *const c_char,
) -> *mut gs_eparam_t {
    panic!()
}

#[no_mangle]
pub extern "C" fn gs_effect_get_technique(
    _effect: *const gs_effect_t,
    _name: *const c_char,
) -> *mut gs_technique_t {
    panic!()
}

#[no_mangle]
pub extern "C" fn gs_effect_set_texture(_param: *mut gs_eparam_t, _val: *mut gs_texture_t) {
    panic!()
}

#[no_mangle]
pub extern "C" fn gs_technique_begin(_technique: *mut gs_technique_t) -> size_t {
    panic!()
}

#[no_mangle]
pub extern "C" fn gs_technique_begin_pass(_technique: *mut gs_technique_t, _pass: size_t) -> bool {
    panic!()
}

#[no_mangle]
pub extern "C" fn gs_technique_end(_technique: *mut gs_technique_t) {
    panic!()
}

#[no_mangle]
pub extern "C" fn gs_technique_end_pass(_technique: *mut gs_technique_t) {
    panic!()
}

#[no_mangle]
pub extern "C" fn obs_get_base_effect(_effect: obs_base_effect) -> *mut gs_effect_t {
    panic!()
}

#[no_mangle]
pub extern "C" fn obs_data_set_default_bool(
    _data: *mut obs_data_t,
    _name: *const c_char,
    _val: bool,
) {
    panic!()
}

#[no_mangle]
pub extern "C" fn obs_data_set_default_int(
    _data: *mut obs_data_t,
    _name: *const c_char,
    _val: c_longlong,
) {
    panic!()
}

#[no_mangle]
pub extern "C" fn obs_properties_add_button(
    _props: *mut obs_properties_t,
    _name: *const c_char,
    _text: *const c_char,
    _callback: obs_property_clicked_t,
) -> *mut obs_property_t {
    panic!()
}

#[no_mangle]
pub extern "C" fn obs_properties_add_text(
    _props: *mut obs_properties_t,
    _name: *const c_char,
    _description: *const c_char,
    _text_type: obs_text_type,
) -> *mut obs_property_t {
    panic!()
}
#[no_mangle]
pub extern "C" fn obs_property_set_modified_callback2(
    _prop: *mut obs_property_t,
    _modified2_callback: obs_property_modified2_t,
    _private: *mut c_void,
) {
    panic!()
}
#[no_mangle]
pub extern "C" fn obs_property_set_description(
    _prop: *mut obs_property_t,
    _description: *const c_char,
) {
    panic!()
}
#[no_mangle]
pub extern "C" fn obs_property_set_enabled(_prop: *mut obs_property_t, _enabled: bool) {
    panic!()
}
#[no_mangle]
pub extern "C" fn obs_property_set_visible(_prop: *mut obs_property_t, _visible: bool) {
    panic!()
}
#[no_mangle]
pub extern "C" fn obs_properties_get(
    _props: *mut obs_properties_t,
    _prop: *const c_char,
) -> *mut obs_property_t {
    panic!()
}
#[no_mangle]
pub extern "C" fn obs_module_get_config_path(
    _module: *mut obs_module_t,
    _file: *const c_char,
) -> *const c_char {
    panic!()
}
#[no_mangle]
pub extern "C" fn obs_properties_add_list(
    _props: *mut obs_properties_t,
    _name: *const c_char,
    _description: *const c_char,
    _combo_type: obs_combo_type,
    _combo_format: obs_combo_format,
) -> *mut obs_property_t {
    panic!()
}
#[no_mangle]
pub extern "C" fn obs_properties_add_editable_list(
    _props: *mut obs_properties_t,
    _name: *const c_char,
    _description: *const c_char,
    _list_type: obs_editable_list_type,
    _filter: *const c_char,
    _default_path: *const c_char,
) -> *mut obs_property_t {
    panic!()
}
#[no_mangle]
pub extern "C" fn obs_property_list_add_string(
    _prop: *mut obs_property_t,
    _name: *const c_char,
    _val: *const c_char,
) -> size_t {
    panic!()
}
#[no_mangle]
pub extern "C" fn obs_data_set_bool(_data: *mut obs_data_t, _name: *const c_char, _val: bool) {
    panic!()
}
#[no_mangle]
pub extern "C" fn obs_data_set_string(
    _data: *mut obs_data_t,
    _name: *const c_char,
    _val: *const c_char,
) {
    panic!()
}
#[no_mangle]
pub extern "C" fn obs_data_get_array(_data: *mut obs_data_t, _name: *const c_char) -> *mut c_void {
    panic!()
}
#[no_mangle]
pub extern "C" fn obs_data_array_count(_array: *mut c_void) -> size_t {
    panic!()
}
#[no_mangle]
pub extern "C" fn obs_data_array_item(_array: *mut c_void, _idx: size_t) -> *mut obs_data_t {
    panic!()
}
#[no_mangle]
pub extern "C" fn obs_data_array_release(_array: *mut c_void) {
    panic!()
}
#[no_mangle]
pub extern "C" fn obs_data_release(_data: *mut obs_data_t) {
    panic!()
}
#[no_mangle]
pub extern "C" fn obs_data_get_json(_data: *mut obs_data_t) -> *const c_char {
    panic!()
}
