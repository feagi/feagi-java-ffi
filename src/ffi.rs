// Copyright 2026 Neuraville Inc.
// SPDX-License-Identifier: Apache-2.0
//
// @cursor:ffi-safe
// C ABI surface for FEAGI Java SDK (JNI will call into these functions).

use std::cell::RefCell;
use std::ffi::{c_char, c_uchar, CStr, CString};
use std::ptr;

use feagi_agent::core::{AgentClient, AgentConfig, AgentType, RegistrationResponse};
use feagi_serialization::FeagiByteContainer;

/// ABI version for `feagi-java-ffi`.
///
/// This must be bumped ONLY when the C ABI changes in a breaking way
/// (signature changes, struct layout changes, removed symbols, semantic contract breaks).
pub const FEAGI_JAVA_FFI_ABI_VERSION: u32 = 1;

/// Status code for all C ABI functions.
#[repr(C)]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum FeagiStatus {
    /// Success.
    Ok = 0,
    /// A required pointer argument was null.
    NullPointer = 1,
    /// An argument was invalid (e.g. empty string, out-of-range value).
    InvalidArgument = 2,
    /// UTF-8 decoding failed for a string argument.
    InvalidUtf8 = 3,
    /// JSON parsing/serialization failed.
    JsonError = 4,
    /// Underlying FEAGI SDK returned an error.
    SdkError = 5,
    /// Allocation failed.
    AllocationFailed = 6,
}

/// Agent type mapping for the C ABI.
#[repr(C)]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum FeagiAgentType {
    Sensory = 0,
    Motor = 1,
    Both = 2,
    Visualization = 3,
    Infrastructure = 4,
}

impl From<FeagiAgentType> for AgentType {
    fn from(value: FeagiAgentType) -> Self {
        match value {
            FeagiAgentType::Sensory => AgentType::Sensory,
            FeagiAgentType::Motor => AgentType::Motor,
            FeagiAgentType::Both => AgentType::Both,
            FeagiAgentType::Visualization => AgentType::Visualization,
            FeagiAgentType::Infrastructure => AgentType::Infrastructure,
        }
    }
}

/// Sensory unit mapping for the C ABI.
#[repr(C)]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum FeagiSensoryUnit {
    Infrared = 0,
    Proximity = 1,
    Shock = 2,
    Battery = 3,
    Servo = 4,
    AnalogGpio = 5,
    DigitalGpio = 6,
    MiscData = 7,
    TextEnglishInput = 8,
    CountInput = 9,
    Vision = 10,
    SegmentedVision = 11,
    Accelerometer = 12,
    Gyroscope = 13,
}

impl From<FeagiSensoryUnit> for feagi_io::SensoryUnit {
    fn from(value: FeagiSensoryUnit) -> Self {
        match value {
            FeagiSensoryUnit::Infrared => feagi_io::SensoryUnit::Infrared,
            FeagiSensoryUnit::Proximity => feagi_io::SensoryUnit::Proximity,
            FeagiSensoryUnit::Shock => feagi_io::SensoryUnit::Shock,
            FeagiSensoryUnit::Battery => feagi_io::SensoryUnit::Battery,
            FeagiSensoryUnit::Servo => feagi_io::SensoryUnit::Servo,
            FeagiSensoryUnit::AnalogGpio => feagi_io::SensoryUnit::AnalogGpio,
            FeagiSensoryUnit::DigitalGpio => feagi_io::SensoryUnit::DigitalGpio,
            FeagiSensoryUnit::MiscData => feagi_io::SensoryUnit::MiscData,
            FeagiSensoryUnit::TextEnglishInput => feagi_io::SensoryUnit::TextEnglishInput,
            FeagiSensoryUnit::CountInput => feagi_io::SensoryUnit::CountInput,
            FeagiSensoryUnit::Vision => feagi_io::SensoryUnit::Vision,
            FeagiSensoryUnit::SegmentedVision => feagi_io::SensoryUnit::SegmentedVision,
            FeagiSensoryUnit::Accelerometer => feagi_io::SensoryUnit::Accelerometer,
            FeagiSensoryUnit::Gyroscope => feagi_io::SensoryUnit::Gyroscope,
        }
    }
}

/// Motor unit mapping for the C ABI.
#[repr(C)]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum FeagiMotorUnit {
    RotaryMotor = 0,
    PositionalServo = 1,
    Gaze = 2,
    MiscData = 3,
    TextEnglishOutput = 4,
    CountOutput = 5,
    ObjectSegmentation = 6,
    SimpleVisionOutput = 7,
}

impl From<FeagiMotorUnit> for feagi_io::MotorUnit {
    fn from(value: FeagiMotorUnit) -> Self {
        match value {
            FeagiMotorUnit::RotaryMotor => feagi_io::MotorUnit::RotaryMotor,
            FeagiMotorUnit::PositionalServo => feagi_io::MotorUnit::PositionalServo,
            FeagiMotorUnit::Gaze => feagi_io::MotorUnit::Gaze,
            FeagiMotorUnit::MiscData => feagi_io::MotorUnit::MiscData,
            FeagiMotorUnit::TextEnglishOutput => feagi_io::MotorUnit::TextEnglishOutput,
            FeagiMotorUnit::CountOutput => feagi_io::MotorUnit::CountOutput,
            FeagiMotorUnit::ObjectSegmentation => feagi_io::MotorUnit::ObjectSegmentation,
            FeagiMotorUnit::SimpleVisionOutput => feagi_io::MotorUnit::SimpleVisionOutput,
        }
    }
}

/// Opaque config handle (caller owns it).
pub struct FeagiAgentConfigHandle {
    config: AgentConfig,
}

/// Opaque client handle (caller owns it).
pub struct FeagiAgentClientHandle {
    client: AgentClient,
}

/// Opaque byte buffer handle allocated by this library.
///
/// JNI callers should:
/// - call `feagi_buffer_ptr` + `feagi_buffer_len`
/// - copy into a Java `byte[]`
/// - call `feagi_buffer_free`
pub struct FeagiByteBufferHandle {
    data: Box<[u8]>,
}

thread_local! {
    static LAST_ERROR: RefCell<Option<CString>> = const { RefCell::new(None) };
}

fn set_last_error(message: impl AsRef<str>) {
    let msg = message.as_ref();
    // If message contains interior NUL, replace it deterministically.
    let sanitized = msg.replace('\0', "\\0");
    let cstr = CString::new(sanitized).unwrap_or_else(|_| CString::new("ffi error").unwrap());
    LAST_ERROR.with(|cell| {
        *cell.borrow_mut() = Some(cstr);
    });
}

fn clear_last_error() {
    LAST_ERROR.with(|cell| {
        *cell.borrow_mut() = None;
    });
}

fn cstr_to_string(ptr: *const c_char, arg_name: &str) -> Result<String, FeagiStatus> {
    if ptr.is_null() {
        set_last_error(format!("{arg_name} must not be null"));
        return Err(FeagiStatus::NullPointer);
    }
    let s = unsafe { CStr::from_ptr(ptr) };
    match s.to_str() {
        Ok(v) => Ok(v.to_string()),
        Err(_) => {
            set_last_error(format!("{arg_name} must be valid UTF-8"));
            Err(FeagiStatus::InvalidUtf8)
        }
    }
}

/// Allocate a C string containing the last error message for the calling thread.
///
/// The returned pointer must be freed with `feagi_string_free`.
#[no_mangle]
pub extern "C" fn feagi_last_error_message_alloc() -> *mut c_char {
    let maybe = LAST_ERROR.with(|cell| cell.borrow().clone());
    match maybe {
        Some(s) => s.into_raw(),
        None => ptr::null_mut(),
    }
}

/// Get the ABI version for this native library.
///
/// Java should call this immediately after loading the native library and refuse to run
/// if the value doesn't match what the Java bindings were compiled against.
#[no_mangle]
pub extern "C" fn feagi_abi_version() -> u32 {
    FEAGI_JAVA_FFI_ABI_VERSION
}

/// Allocate a string containing the crate version (Cargo package version).
///
/// The returned pointer must be freed with `feagi_string_free`.
#[no_mangle]
pub extern "C" fn feagi_library_version_alloc() -> *mut c_char {
    clear_last_error();
    let v = env!("CARGO_PKG_VERSION");
    match CString::new(v) {
        Ok(s) => s.into_raw(),
        Err(e) => {
            set_last_error(format!("Failed to allocate version string: {e}"));
            ptr::null_mut()
        }
    }
}

/// Free a string previously returned by `feagi_last_error_message_alloc` (or any other
/// `*_alloc` string function in this library).
#[no_mangle]
pub extern "C" fn feagi_string_free(s: *mut c_char) {
    if s.is_null() {
        return;
    }
    unsafe {
        drop(CString::from_raw(s));
    }
}

#[no_mangle]
pub extern "C" fn feagi_buffer_ptr(buf: *const FeagiByteBufferHandle) -> *const c_uchar {
    if buf.is_null() {
        return ptr::null();
    }
    unsafe { (*buf).data.as_ptr() }
}

#[no_mangle]
pub extern "C" fn feagi_buffer_len(buf: *const FeagiByteBufferHandle) -> usize {
    if buf.is_null() {
        return 0;
    }
    // Be explicit: avoid implicit autoref on raw pointer deref.
    let data: &[u8] = unsafe { &(*buf).data };
    data.len()
}

#[no_mangle]
pub extern "C" fn feagi_buffer_free(buf: *mut FeagiByteBufferHandle) {
    if buf.is_null() {
        return;
    }
    unsafe {
        drop(Box::from_raw(buf));
    }
}

/// Create a new config handle.
///
/// Note: endpoints and capabilities must be provided explicitly before `feagi_client_new`.
#[no_mangle]
pub extern "C" fn feagi_config_new(
    agent_id: *const c_char,
    agent_type: FeagiAgentType,
) -> *mut FeagiAgentConfigHandle {
    clear_last_error();
    let Ok(agent_id) = cstr_to_string(agent_id, "agent_id") else {
        return ptr::null_mut();
    };
    if agent_id.is_empty() {
        set_last_error("agent_id must not be empty");
        return ptr::null_mut();
    }
    let config = AgentConfig::new(agent_id, AgentType::from(agent_type));
    Box::into_raw(Box::new(FeagiAgentConfigHandle { config }))
}

#[no_mangle]
pub extern "C" fn feagi_config_free(cfg: *mut FeagiAgentConfigHandle) {
    if cfg.is_null() {
        return;
    }
    unsafe {
        drop(Box::from_raw(cfg));
    }
}

#[no_mangle]
pub extern "C" fn feagi_config_set_registration_endpoint(
    cfg: *mut FeagiAgentConfigHandle,
    endpoint: *const c_char,
) -> FeagiStatus {
    clear_last_error();
    if cfg.is_null() {
        set_last_error("cfg must not be null");
        return FeagiStatus::NullPointer;
    }
    let Ok(endpoint) = cstr_to_string(endpoint, "registration_endpoint") else {
        return FeagiStatus::InvalidUtf8;
    };
    unsafe {
        (*cfg).config = (*cfg)
            .config
            .clone()
            .with_registration_endpoint(endpoint);
    }
    FeagiStatus::Ok
}

#[no_mangle]
pub extern "C" fn feagi_config_set_sensory_endpoint(
    cfg: *mut FeagiAgentConfigHandle,
    endpoint: *const c_char,
) -> FeagiStatus {
    clear_last_error();
    if cfg.is_null() {
        set_last_error("cfg must not be null");
        return FeagiStatus::NullPointer;
    }
    let Ok(endpoint) = cstr_to_string(endpoint, "sensory_endpoint") else {
        return FeagiStatus::InvalidUtf8;
    };
    unsafe {
        (*cfg).config = (*cfg).config.clone().with_sensory_endpoint(endpoint);
    }
    FeagiStatus::Ok
}

#[no_mangle]
pub extern "C" fn feagi_config_set_motor_endpoint(
    cfg: *mut FeagiAgentConfigHandle,
    endpoint: *const c_char,
) -> FeagiStatus {
    clear_last_error();
    if cfg.is_null() {
        set_last_error("cfg must not be null");
        return FeagiStatus::NullPointer;
    }
    let Ok(endpoint) = cstr_to_string(endpoint, "motor_endpoint") else {
        return FeagiStatus::InvalidUtf8;
    };
    unsafe {
        (*cfg).config = (*cfg).config.clone().with_motor_endpoint(endpoint);
    }
    FeagiStatus::Ok
}

#[no_mangle]
pub extern "C" fn feagi_config_set_visualization_endpoint(
    cfg: *mut FeagiAgentConfigHandle,
    endpoint: *const c_char,
) -> FeagiStatus {
    clear_last_error();
    if cfg.is_null() {
        set_last_error("cfg must not be null");
        return FeagiStatus::NullPointer;
    }
    let Ok(endpoint) = cstr_to_string(endpoint, "visualization_endpoint") else {
        return FeagiStatus::InvalidUtf8;
    };
    unsafe {
        (*cfg).config = (*cfg)
            .config
            .clone()
            .with_visualization_endpoint(endpoint);
    }
    FeagiStatus::Ok
}

#[no_mangle]
pub extern "C" fn feagi_config_set_control_endpoint(
    cfg: *mut FeagiAgentConfigHandle,
    endpoint: *const c_char,
) -> FeagiStatus {
    clear_last_error();
    if cfg.is_null() {
        set_last_error("cfg must not be null");
        return FeagiStatus::NullPointer;
    }
    let Ok(endpoint) = cstr_to_string(endpoint, "control_endpoint") else {
        return FeagiStatus::InvalidUtf8;
    };
    unsafe {
        (*cfg).config = (*cfg).config.clone().with_control_endpoint(endpoint);
    }
    FeagiStatus::Ok
}

/// Convenience helper to set all FEAGI endpoints from a host + explicit ports.
///
/// This has **no default ports**: every port must be provided explicitly.
#[no_mangle]
pub extern "C" fn feagi_config_set_feagi_endpoints(
    cfg: *mut FeagiAgentConfigHandle,
    host: *const c_char,
    registration_port: u16,
    sensory_port: u16,
    motor_port: u16,
    visualization_port: u16,
    control_port: u16,
) -> FeagiStatus {
    clear_last_error();
    if cfg.is_null() {
        set_last_error("cfg must not be null");
        return FeagiStatus::NullPointer;
    }
    let Ok(host) = cstr_to_string(host, "host") else {
        return FeagiStatus::InvalidUtf8;
    };
    if host.is_empty() {
        set_last_error("host must not be empty");
        return FeagiStatus::InvalidArgument;
    }
    if registration_port == 0
        || sensory_port == 0
        || motor_port == 0
        || visualization_port == 0
        || control_port == 0
    {
        set_last_error("all ports must be > 0");
        return FeagiStatus::InvalidArgument;
    }

    unsafe {
        (*cfg).config = (*cfg).config.clone().with_feagi_endpoints(
            host,
            registration_port,
            sensory_port,
            motor_port,
            visualization_port,
            control_port,
        );
    }
    FeagiStatus::Ok
}

#[no_mangle]
pub extern "C" fn feagi_config_set_heartbeat_interval_s(
    cfg: *mut FeagiAgentConfigHandle,
    heartbeat_interval_s: f64,
) -> FeagiStatus {
    clear_last_error();
    if cfg.is_null() {
        set_last_error("cfg must not be null");
        return FeagiStatus::NullPointer;
    }
    if heartbeat_interval_s < 0.0 {
        set_last_error("heartbeat_interval_s must be >= 0");
        return FeagiStatus::InvalidArgument;
    }
    unsafe {
        (*cfg).config = (*cfg)
            .config
            .clone()
            .with_heartbeat_interval(heartbeat_interval_s);
    }
    FeagiStatus::Ok
}

#[no_mangle]
pub extern "C" fn feagi_config_set_connection_timeout_ms(
    cfg: *mut FeagiAgentConfigHandle,
    connection_timeout_ms: u64,
) -> FeagiStatus {
    clear_last_error();
    if cfg.is_null() {
        set_last_error("cfg must not be null");
        return FeagiStatus::NullPointer;
    }
    if connection_timeout_ms == 0 {
        set_last_error("connection_timeout_ms must be > 0");
        return FeagiStatus::InvalidArgument;
    }
    unsafe {
        (*cfg).config = (*cfg)
            .config
            .clone()
            .with_connection_timeout_ms(connection_timeout_ms);
    }
    FeagiStatus::Ok
}

#[no_mangle]
pub extern "C" fn feagi_config_set_registration_retries(
    cfg: *mut FeagiAgentConfigHandle,
    registration_retries: u32,
) -> FeagiStatus {
    clear_last_error();
    if cfg.is_null() {
        set_last_error("cfg must not be null");
        return FeagiStatus::NullPointer;
    }
    unsafe {
        (*cfg).config = (*cfg)
            .config
            .clone()
            .with_registration_retries(registration_retries);
    }
    FeagiStatus::Ok
}

#[no_mangle]
pub extern "C" fn feagi_config_set_retry_backoff_ms(
    cfg: *mut FeagiAgentConfigHandle,
    retry_backoff_ms: u64,
) -> FeagiStatus {
    clear_last_error();
    if cfg.is_null() {
        set_last_error("cfg must not be null");
        return FeagiStatus::NullPointer;
    }
    if retry_backoff_ms == 0 {
        set_last_error("retry_backoff_ms must be > 0");
        return FeagiStatus::InvalidArgument;
    }
    unsafe {
        (*cfg).config = (*cfg).config.clone().with_retry_backoff_ms(retry_backoff_ms);
    }
    FeagiStatus::Ok
}

#[no_mangle]
pub extern "C" fn feagi_config_set_sensory_socket_config(
    cfg: *mut FeagiAgentConfigHandle,
    send_hwm: i32,
    linger_ms: i32,
    immediate: bool,
) -> FeagiStatus {
    clear_last_error();
    if cfg.is_null() {
        set_last_error("cfg must not be null");
        return FeagiStatus::NullPointer;
    }
    unsafe {
        (*cfg).config = (*cfg)
            .config
            .clone()
            .with_sensory_socket_config(send_hwm, linger_ms, immediate);
    }
    FeagiStatus::Ok
}

/// Add a generic sensory capability (non-vision) to satisfy config validation.
#[no_mangle]
pub extern "C" fn feagi_config_set_sensory_capability(
    cfg: *mut FeagiAgentConfigHandle,
    rate_hz: f64,
    shm_path: *const c_char,
) -> FeagiStatus {
    clear_last_error();
    if cfg.is_null() {
        set_last_error("cfg must not be null");
        return FeagiStatus::NullPointer;
    }
    if rate_hz <= 0.0 {
        set_last_error("rate_hz must be > 0");
        return FeagiStatus::InvalidArgument;
    }
    let shm_path_opt = if shm_path.is_null() {
        None
    } else {
        let Ok(p) = cstr_to_string(shm_path, "shm_path") else {
            return FeagiStatus::InvalidUtf8;
        };
        if p.is_empty() {
            None
        } else {
            Some(p)
        }
    };
    unsafe {
        (*cfg).config = (*cfg)
            .config
            .clone()
            .with_sensory_capability(rate_hz, shm_path_opt);
    }
    FeagiStatus::Ok
}

/// Add a vision capability.
#[no_mangle]
pub extern "C" fn feagi_config_set_vision_capability(
    cfg: *mut FeagiAgentConfigHandle,
    modality: *const c_char,
    width: usize,
    height: usize,
    channels: usize,
    target_cortical_area: *const c_char,
) -> FeagiStatus {
    clear_last_error();
    if cfg.is_null() {
        set_last_error("cfg must not be null");
        return FeagiStatus::NullPointer;
    }
    if width == 0 || height == 0 || channels == 0 {
        set_last_error("width/height/channels must be > 0");
        return FeagiStatus::InvalidArgument;
    }
    let Ok(modality) = cstr_to_string(modality, "modality") else {
        return FeagiStatus::InvalidUtf8;
    };
    let Ok(target) = cstr_to_string(target_cortical_area, "target_cortical_area") else {
        return FeagiStatus::InvalidUtf8;
    };
    unsafe {
        (*cfg).config = (*cfg)
            .config
            .clone()
            .with_vision_capability(modality, (width, height), channels, target);
    }
    FeagiStatus::Ok
}

/// Add a vision capability using semantic unit + group (Option B contract).
#[no_mangle]
pub extern "C" fn feagi_config_set_vision_unit(
    cfg: *mut FeagiAgentConfigHandle,
    modality: *const c_char,
    width: usize,
    height: usize,
    channels: usize,
    unit: FeagiSensoryUnit,
    group: u8,
) -> FeagiStatus {
    clear_last_error();
    if cfg.is_null() {
        set_last_error("cfg must not be null");
        return FeagiStatus::NullPointer;
    }
    if width == 0 || height == 0 || channels == 0 {
        set_last_error("width/height/channels must be > 0");
        return FeagiStatus::InvalidArgument;
    }
    let Ok(modality) = cstr_to_string(modality, "modality") else {
        return FeagiStatus::InvalidUtf8;
    };
    unsafe {
        (*cfg).config = (*cfg)
            .config
            .clone()
            .with_vision_unit(modality, (width, height), channels, unit.into(), group);
    }
    FeagiStatus::Ok
}

/// Add a motor capability.
#[no_mangle]
pub extern "C" fn feagi_config_set_motor_capability(
    cfg: *mut FeagiAgentConfigHandle,
    modality: *const c_char,
    output_count: usize,
    source_cortical_areas_json: *const c_char,
) -> FeagiStatus {
    clear_last_error();
    if cfg.is_null() {
        set_last_error("cfg must not be null");
        return FeagiStatus::NullPointer;
    }
    if output_count == 0 {
        set_last_error("output_count must be > 0");
        return FeagiStatus::InvalidArgument;
    }
    let Ok(modality) = cstr_to_string(modality, "modality") else {
        return FeagiStatus::InvalidUtf8;
    };
    let Ok(json_str) = cstr_to_string(source_cortical_areas_json, "source_cortical_areas_json")
    else {
        return FeagiStatus::InvalidUtf8;
    };

    let value: serde_json::Value = match serde_json::from_str(&json_str) {
        Ok(v) => v,
        Err(e) => {
            set_last_error(format!("source_cortical_areas_json parse failed: {e}"));
            return FeagiStatus::JsonError;
        }
    };
    let arr = match value.as_array() {
        Some(a) => a,
        None => {
            set_last_error("source_cortical_areas_json must be a JSON array of strings");
            return FeagiStatus::InvalidArgument;
        }
    };
    let mut cortical_areas = Vec::with_capacity(arr.len());
    for v in arr {
        let Some(s) = v.as_str() else {
            set_last_error("source_cortical_areas_json must be a JSON array of strings");
            return FeagiStatus::InvalidArgument;
        };
        cortical_areas.push(s.to_string());
    }

    unsafe {
        (*cfg).config = (*cfg)
            .config
            .clone()
            .with_motor_capability(modality, output_count, cortical_areas);
    }
    FeagiStatus::Ok
}

/// Add a motor capability using semantic unit + group (Option B contract).
#[no_mangle]
pub extern "C" fn feagi_config_set_motor_unit(
    cfg: *mut FeagiAgentConfigHandle,
    modality: *const c_char,
    output_count: usize,
    unit: FeagiMotorUnit,
    group: u8,
) -> FeagiStatus {
    clear_last_error();
    if cfg.is_null() {
        set_last_error("cfg must not be null");
        return FeagiStatus::NullPointer;
    }
    if output_count == 0 {
        set_last_error("output_count must be > 0");
        return FeagiStatus::InvalidArgument;
    }
    let Ok(modality) = cstr_to_string(modality, "modality") else {
        return FeagiStatus::InvalidUtf8;
    };
    unsafe {
        (*cfg).config = (*cfg)
            .config
            .clone()
            .with_motor_unit(modality, output_count, unit.into(), group);
    }
    FeagiStatus::Ok
}

/// Add multiple motor unit sources from JSON.
///
/// Expects a JSON array of objects: `[{"unit":"rotary_motor","group":0}, ...]`.
#[no_mangle]
pub extern "C" fn feagi_config_set_motor_units_json(
    cfg: *mut FeagiAgentConfigHandle,
    modality: *const c_char,
    output_count: usize,
    motor_units_json: *const c_char,
) -> FeagiStatus {
    clear_last_error();
    if cfg.is_null() {
        set_last_error("cfg must not be null");
        return FeagiStatus::NullPointer;
    }
    if output_count == 0 {
        set_last_error("output_count must be > 0");
        return FeagiStatus::InvalidArgument;
    }
    let Ok(modality) = cstr_to_string(modality, "modality") else {
        return FeagiStatus::InvalidUtf8;
    };
    let Ok(json_str) = cstr_to_string(motor_units_json, "motor_units_json") else {
        return FeagiStatus::InvalidUtf8;
    };
    let units: Vec<feagi_io::MotorUnitSpec> = match serde_json::from_str(&json_str) {
        Ok(v) => v,
        Err(e) => {
            set_last_error(format!("motor_units_json parse failed: {e}"));
            return FeagiStatus::JsonError;
        }
    };
    if units.is_empty() {
        set_last_error("motor_units_json must include at least one unit");
        return FeagiStatus::InvalidArgument;
    }
    unsafe {
        (*cfg).config = (*cfg)
            .config
            .clone()
            .with_motor_units(modality, output_count, units);
    }
    FeagiStatus::Ok
}

/// Add a visualization capability.
#[no_mangle]
pub extern "C" fn feagi_config_set_visualization_capability(
    cfg: *mut FeagiAgentConfigHandle,
    visualization_type: *const c_char,
    has_resolution: bool,
    resolution_width: usize,
    resolution_height: usize,
    has_refresh_rate: bool,
    refresh_rate_hz: f64,
    bridge_proxy: bool,
) -> FeagiStatus {
    clear_last_error();
    if cfg.is_null() {
        set_last_error("cfg must not be null");
        return FeagiStatus::NullPointer;
    }
    let Ok(visualization_type) = cstr_to_string(visualization_type, "visualization_type") else {
        return FeagiStatus::InvalidUtf8;
    };
    let resolution = if has_resolution {
        if resolution_width == 0 || resolution_height == 0 {
            set_last_error("resolution_width/height must be > 0 when has_resolution is true");
            return FeagiStatus::InvalidArgument;
        }
        Some((resolution_width, resolution_height))
    } else {
        None
    };
    let refresh_rate = if has_refresh_rate {
        if refresh_rate_hz <= 0.0 {
            set_last_error("refresh_rate_hz must be > 0 when has_refresh_rate is true");
            return FeagiStatus::InvalidArgument;
        }
        Some(refresh_rate_hz)
    } else {
        None
    };
    unsafe {
        (*cfg).config = (*cfg).config.clone().with_visualization_capability(
            visualization_type,
            resolution,
            refresh_rate,
            bridge_proxy,
        );
    }
    FeagiStatus::Ok
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;

    #[test]
    fn test_visualization_config_roundtrip() {
        let agent_id = CString::new("viz_agent").unwrap();
        let registration = CString::new("tcp://feagi.invalid:30001").unwrap();
        let visualization = CString::new("tcp://feagi.invalid:5562").unwrap();
        let viz_type = CString::new("3d_brain").unwrap();

        let cfg = feagi_config_new(agent_id.as_ptr(), FeagiAgentType::Visualization);
        assert!(!cfg.is_null());

        assert_eq!(
            feagi_config_set_registration_endpoint(cfg, registration.as_ptr()),
            FeagiStatus::Ok
        );
        assert_eq!(
            feagi_config_set_visualization_endpoint(cfg, visualization.as_ptr()),
            FeagiStatus::Ok
        );
        assert_eq!(
            feagi_config_set_visualization_capability(
                cfg,
                viz_type.as_ptr(),
                false,
                0,
                0,
                false,
                0.0,
                false
            ),
            FeagiStatus::Ok
        );
        assert_eq!(feagi_config_validate(cfg), FeagiStatus::Ok);

        feagi_config_free(cfg);
    }
}

/// Add a custom capability from JSON.
#[no_mangle]
pub extern "C" fn feagi_config_set_custom_capability_json(
    cfg: *mut FeagiAgentConfigHandle,
    key: *const c_char,
    json_value: *const c_char,
) -> FeagiStatus {
    clear_last_error();
    if cfg.is_null() {
        set_last_error("cfg must not be null");
        return FeagiStatus::NullPointer;
    }
    let Ok(key) = cstr_to_string(key, "key") else {
        return FeagiStatus::InvalidUtf8;
    };
    if key.is_empty() {
        set_last_error("key must not be empty");
        return FeagiStatus::InvalidArgument;
    }
    let Ok(json_str) = cstr_to_string(json_value, "json_value") else {
        return FeagiStatus::InvalidUtf8;
    };
    let value: serde_json::Value = match serde_json::from_str(&json_str) {
        Ok(v) => v,
        Err(e) => {
            set_last_error(format!("json_value parse failed: {e}"));
            return FeagiStatus::JsonError;
        }
    };
    unsafe {
        (*cfg).config = (*cfg).config.clone().with_custom_capability(key, value);
    }
    FeagiStatus::Ok
}

/// Validate the config (same rules as Rust SDK uses internally).
#[no_mangle]
pub extern "C" fn feagi_config_validate(cfg: *const FeagiAgentConfigHandle) -> FeagiStatus {
    clear_last_error();
    if cfg.is_null() {
        set_last_error("cfg must not be null");
        return FeagiStatus::NullPointer;
    }
    let res = unsafe { (*cfg).config.validate() };
    match res {
        Ok(()) => FeagiStatus::Ok,
        Err(e) => {
            set_last_error(e.to_string());
            FeagiStatus::SdkError
        }
    }
}

/// Create a new client from the config.
///
/// On success, `out_client` will be set and the caller owns the resulting handle.
#[no_mangle]
pub extern "C" fn feagi_client_new(
    cfg: *const FeagiAgentConfigHandle,
    out_client: *mut *mut FeagiAgentClientHandle,
) -> FeagiStatus {
    clear_last_error();
    if cfg.is_null() {
        set_last_error("cfg must not be null");
        return FeagiStatus::NullPointer;
    }
    if out_client.is_null() {
        set_last_error("out_client must not be null");
        return FeagiStatus::NullPointer;
    }
    let config = unsafe { (*cfg).config.clone() };
    match AgentClient::new(config) {
        Ok(client) => {
            let boxed = Box::new(FeagiAgentClientHandle { client });
            unsafe {
                *out_client = Box::into_raw(boxed);
            }
            FeagiStatus::Ok
        }
        Err(e) => {
            set_last_error(e.to_string());
            FeagiStatus::SdkError
        }
    }
}

#[no_mangle]
pub extern "C" fn feagi_client_free(client: *mut FeagiAgentClientHandle) {
    if client.is_null() {
        return;
    }
    unsafe {
        drop(Box::from_raw(client));
    }
}

#[no_mangle]
pub extern "C" fn feagi_client_connect(client: *mut FeagiAgentClientHandle) -> FeagiStatus {
    clear_last_error();
    if client.is_null() {
        set_last_error("client must not be null");
        return FeagiStatus::NullPointer;
    }
    let res = unsafe { (*client).client.connect() };
    match res {
        Ok(()) => FeagiStatus::Ok,
        Err(e) => {
            set_last_error(e.to_string());
            FeagiStatus::SdkError
        }
    }
}

/// Allocate the last successful FEAGI registration response body as JSON.
///
/// This is only available after `feagi_client_connect(...)` succeeds.
///
/// The returned pointer must be freed with `feagi_string_free`.
#[no_mangle]
pub extern "C" fn feagi_client_registration_response_json_alloc(
    client: *const FeagiAgentClientHandle,
) -> *mut c_char {
    clear_last_error();
    if client.is_null() {
        set_last_error("client must not be null");
        return ptr::null_mut();
    }
    let body_opt = unsafe { (*client).client.registration_body_json() };
    let Some(body) = body_opt else {
        set_last_error(
            "Registration response not available (call feagi_client_connect() successfully first)",
        );
        return ptr::null_mut();
    };
    let json = match serde_json::to_string(body) {
        Ok(s) => s,
        Err(e) => {
            set_last_error(format!("Failed to serialize registration response: {e}"));
            return ptr::null_mut();
        }
    };
    match CString::new(json) {
        Ok(s) => s.into_raw(),
        Err(e) => {
            set_last_error(format!("Failed to allocate registration response string: {e}"));
            ptr::null_mut()
        }
    }
}

/// Allocate the ZMQ port map from the last successful registration response as JSON.
///
/// Returns a JSON object like: `{"sensory":5558,"motor":5564,...}`.
/// The returned pointer must be freed with `feagi_string_free`.
#[no_mangle]
pub extern "C" fn feagi_client_registration_zmq_ports_json_alloc(
    client: *const FeagiAgentClientHandle,
) -> *mut c_char {
    clear_last_error();
    if client.is_null() {
        set_last_error("client must not be null");
        return ptr::null_mut();
    }

    let body_opt = unsafe { (*client).client.registration_body_json() };
    let Some(body) = body_opt else {
        set_last_error(
            "Registration response not available (call feagi_client_connect() successfully first)",
        );
        return ptr::null_mut();
    };

    let parsed = match RegistrationResponse::from_json(body) {
        Ok(v) => v,
        Err(e) => {
            set_last_error(format!("Failed to parse registration response: {e}"));
            return ptr::null_mut();
        }
    };

    let transport = match parsed.get_transport("zmq") {
        Some(t) => t,
        None => {
            set_last_error("Registration response did not include an enabled ZMQ transport");
            return ptr::null_mut();
        }
    };

    let ports = transport.ports.clone();

    let json = match serde_json::to_string(&ports) {
        Ok(s) => s,
        Err(e) => {
            set_last_error(format!("Failed to serialize ZMQ ports: {e}"));
            return ptr::null_mut();
        }
    };

    match CString::new(json) {
        Ok(s) => s.into_raw(),
        Err(e) => {
            set_last_error(format!("Failed to allocate zmq_ports string: {e}"));
            ptr::null_mut()
        }
    }
}

/// Allocate the transport configuration chosen from the last successful registration response as JSON.
///
/// - If `preference` is non-null/non-empty, it is attempted first.
/// - Otherwise, FEAGI's `recommended_transport` is used.
///
/// The returned pointer must be freed with `feagi_string_free`.
#[no_mangle]
pub extern "C" fn feagi_client_registration_chosen_transport_json_alloc(
    client: *const FeagiAgentClientHandle,
    preference: *const c_char,
) -> *mut c_char {
    clear_last_error();
    if client.is_null() {
        set_last_error("client must not be null");
        return ptr::null_mut();
    }

    let body_opt = unsafe { (*client).client.registration_body_json() };
    let Some(body) = body_opt else {
        set_last_error(
            "Registration response not available (call feagi_client_connect() successfully first)",
        );
        return ptr::null_mut();
    };

    let parsed = match RegistrationResponse::from_json(body) {
        Ok(v) => v,
        Err(e) => {
            set_last_error(format!("Failed to parse registration response: {e}"));
            return ptr::null_mut();
        }
    };

    let pref = if preference.is_null() {
        None
    } else {
        match cstr_to_string(preference, "preference") {
            Ok(s) if !s.is_empty() => Some(s),
            Ok(_) => None,
            Err(_) => return ptr::null_mut(),
        }
    };

    let chosen = parsed.choose_transport(pref.as_deref());
    let Some(chosen) = chosen else {
        set_last_error("No enabled transports available in registration response");
        return ptr::null_mut();
    };

    let json = match serde_json::to_string(chosen) {
        Ok(s) => s,
        Err(e) => {
            set_last_error(format!("Failed to serialize chosen transport: {e}"));
            return ptr::null_mut();
        }
    };

    match CString::new(json) {
        Ok(s) => s.into_raw(),
        Err(e) => {
            set_last_error(format!(
                "Failed to allocate chosen transport string: {e}"
            ));
            ptr::null_mut()
        }
    }
}

/// Allocate FEAGI's `recommended_transport` string (if provided).
///
/// The returned pointer must be freed with `feagi_string_free`.
#[no_mangle]
pub extern "C" fn feagi_client_registration_recommended_transport_alloc(
    client: *const FeagiAgentClientHandle,
) -> *mut c_char {
    clear_last_error();
    if client.is_null() {
        set_last_error("client must not be null");
        return ptr::null_mut();
    }

    let body_opt = unsafe { (*client).client.registration_body_json() };
    let Some(body) = body_opt else {
        set_last_error(
            "Registration response not available (call feagi_client_connect() successfully first)",
        );
        return ptr::null_mut();
    };

    let parsed = match RegistrationResponse::from_json(body) {
        Ok(v) => v,
        Err(e) => {
            set_last_error(format!("Failed to parse registration response: {e}"));
            return ptr::null_mut();
        }
    };

    let Some(recommended) = parsed.recommended_transport else {
        set_last_error("Registration response did not include recommended_transport");
        return ptr::null_mut();
    };

    match CString::new(recommended) {
        Ok(s) => s.into_raw(),
        Err(e) => {
            set_last_error(format!(
                "Failed to allocate recommended_transport string: {e}"
            ));
            ptr::null_mut()
        }
    }
}

/// Send bytes (non-blocking semantics, drops on backpressure).
#[no_mangle]
pub extern "C" fn feagi_client_send_sensory_bytes(
    client: *mut FeagiAgentClientHandle,
    bytes: *const c_uchar,
    len: usize,
) -> FeagiStatus {
    clear_last_error();
    if client.is_null() {
        set_last_error("client must not be null");
        return FeagiStatus::NullPointer;
    }
    if bytes.is_null() {
        set_last_error("bytes must not be null");
        return FeagiStatus::NullPointer;
    }
    if len == 0 {
        set_last_error("len must be > 0");
        return FeagiStatus::InvalidArgument;
    }
    let slice = unsafe { std::slice::from_raw_parts(bytes, len) };
    let res = unsafe { (*client).client.send_sensory_bytes(slice.to_vec()) };
    match res {
        Ok(()) => FeagiStatus::Ok,
        Err(e) => {
            set_last_error(e.to_string());
            FeagiStatus::SdkError
        }
    }
}

/// Try send bytes (non-blocking), returning whether the message was actually sent.
#[no_mangle]
pub extern "C" fn feagi_client_try_send_sensory_bytes(
    client: *mut FeagiAgentClientHandle,
    bytes: *const c_uchar,
    len: usize,
    out_sent: *mut bool,
) -> FeagiStatus {
    clear_last_error();
    if client.is_null() {
        set_last_error("client must not be null");
        return FeagiStatus::NullPointer;
    }
    if bytes.is_null() {
        set_last_error("bytes must not be null");
        return FeagiStatus::NullPointer;
    }
    if out_sent.is_null() {
        set_last_error("out_sent must not be null");
        return FeagiStatus::NullPointer;
    }
    if len == 0 {
        set_last_error("len must be > 0");
        return FeagiStatus::InvalidArgument;
    }
    let slice = unsafe { std::slice::from_raw_parts(bytes, len) };
    let res = unsafe { (*client).client.try_send_sensory_bytes(slice) };
    match res {
        Ok(sent) => {
            unsafe { *out_sent = sent };
            FeagiStatus::Ok
        }
        Err(e) => {
            set_last_error(e.to_string());
            FeagiStatus::SdkError
        }
    }
}

/// Receive motor data (non-blocking). If no data is available, `out_has_data` is set to false.
///
/// If data is available, it is returned as FEAGI byte-container bytes (FBC) via an opaque
/// buffer handle allocated by this library. The caller must free it with `feagi_buffer_free`.
#[no_mangle]
pub extern "C" fn feagi_client_receive_motor_buffer(
    client: *mut FeagiAgentClientHandle,
    out_buf: *mut *mut FeagiByteBufferHandle,
    out_has_data: *mut bool,
) -> FeagiStatus {
    clear_last_error();
    if client.is_null() {
        set_last_error("client must not be null");
        return FeagiStatus::NullPointer;
    }
    if out_buf.is_null() || out_has_data.is_null() {
        set_last_error("out_buf/out_has_data must not be null");
        return FeagiStatus::NullPointer;
    }

    let res = unsafe { (*client).client.receive_motor_data() };
    let maybe = match res {
        Ok(v) => v,
        Err(e) => {
            set_last_error(e.to_string());
            return FeagiStatus::SdkError;
        }
    };

    let Some(motor_data) = maybe else {
        unsafe {
            *out_has_data = false;
            *out_buf = ptr::null_mut();
        }
        return FeagiStatus::Ok;
    };

    let mut container = FeagiByteContainer::new_empty();
    if let Err(e) = container.overwrite_byte_data_with_single_struct_data(&motor_data, 0) {
        set_last_error(format!("Failed to serialize motor data to container: {e:?}"));
        return FeagiStatus::SdkError;
    }
    let buffer = container.get_byte_ref().to_vec();

    if buffer.is_empty() {
        unsafe {
            *out_has_data = false;
            *out_buf = ptr::null_mut();
        }
        return FeagiStatus::Ok;
    }

    let buf_handle = FeagiByteBufferHandle {
        data: buffer.into_boxed_slice(),
    };
    let raw = Box::into_raw(Box::new(buf_handle));

    unsafe {
        *out_has_data = true;
        *out_buf = raw;
    }
    FeagiStatus::Ok
}

