// Copyright 2026 Neuraville Inc.
// SPDX-License-Identifier: Apache-2.0
//
// @cursor:ffi-safe
// C ABI surface for FEAGI Java SDK (JNI will call into these functions).

use std::cell::RefCell;
use std::collections::HashMap;
use std::ffi::{c_char, c_uchar, CStr, CString};
use std::ptr;
use std::time::{Duration, Instant};

use feagi_agent::clients::{AgentRegistrationStatus, CommandControlAgent};
use feagi_agent::{AgentCapabilities, AgentDescriptor, AuthToken};
use feagi_io::protocol_implementations::websocket::WebSocketUrl;
use feagi_io::protocol_implementations::zmq::ZmqUrl;
use feagi_io::traits_and_enums::client::{FeagiClientPusher, FeagiClientSubscriber};
use feagi_io::traits_and_enums::shared::{FeagiEndpointState, TransportProtocolEndpoint};
use feagi_io::AgentID;
use serde::{Deserialize, Serialize};

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

impl FeagiSensoryUnit {
    fn as_contract_str(self) -> &'static str {
        match self {
            FeagiSensoryUnit::Infrared => "infrared",
            FeagiSensoryUnit::Proximity => "proximity",
            FeagiSensoryUnit::Shock => "shock",
            FeagiSensoryUnit::Battery => "battery",
            FeagiSensoryUnit::Servo => "servo",
            FeagiSensoryUnit::AnalogGpio => "analog_gpio",
            FeagiSensoryUnit::DigitalGpio => "digital_gpio",
            FeagiSensoryUnit::MiscData => "misc_data",
            FeagiSensoryUnit::TextEnglishInput => "text_english_input",
            FeagiSensoryUnit::CountInput => "count_input",
            FeagiSensoryUnit::Vision => "vision",
            FeagiSensoryUnit::SegmentedVision => "segmented_vision",
            FeagiSensoryUnit::Accelerometer => "accelerometer",
            FeagiSensoryUnit::Gyroscope => "gyroscope",
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

impl FeagiMotorUnit {
    fn as_contract_str(self) -> &'static str {
        match self {
            FeagiMotorUnit::RotaryMotor => "rotary_motor",
            FeagiMotorUnit::PositionalServo => "positional_servo",
            FeagiMotorUnit::Gaze => "gaze",
            FeagiMotorUnit::MiscData => "misc_data",
            FeagiMotorUnit::TextEnglishOutput => "text_english_output",
            FeagiMotorUnit::CountOutput => "count_output",
            FeagiMotorUnit::ObjectSegmentation => "object_segmentation",
            FeagiMotorUnit::SimpleVisionOutput => "simple_vision_output",
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
enum AgentType {
    Sensory,
    Motor,
    Both,
    Visualization,
    Infrastructure,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct MotorUnitCompat {
    unit: String,
    group: u8,
}

#[derive(Clone, Debug)]
struct AgentConfig {
    agent_id: String,
    agent_type: AgentType,
    registration_endpoint: String,
    sensory_endpoint: String,
    motor_endpoint: String,
    visualization_endpoint: String,
    control_endpoint: String,
    heartbeat_interval_s: Option<f64>,
    connection_timeout_ms: Option<u64>,
    registration_retries: Option<u32>,
    retry_backoff_ms: Option<u64>,
    sensory_send_hwm: i32,
    sensory_linger_ms: i32,
    sensory_immediate: bool,
    descriptor: Option<AgentDescriptor>,
    auth_token: Option<AuthToken>,
    sensory_capability: Option<serde_json::Value>,
    vision_capability: Option<serde_json::Value>,
    motor_capability: Option<serde_json::Value>,
    visualization_capability: Option<serde_json::Value>,
    custom_capabilities: Vec<(String, serde_json::Value)>,
}

impl AgentConfig {
    fn new(agent_id: String, agent_type: AgentType) -> Self {
        Self {
            agent_id,
            agent_type,
            registration_endpoint: String::new(),
            sensory_endpoint: String::new(),
            motor_endpoint: String::new(),
            visualization_endpoint: String::new(),
            control_endpoint: String::new(),
            heartbeat_interval_s: None,
            connection_timeout_ms: None,
            registration_retries: None,
            retry_backoff_ms: None,
            sensory_send_hwm: 1,
            sensory_linger_ms: 0,
            sensory_immediate: true,
            descriptor: None,
            auth_token: None,
            sensory_capability: None,
            vision_capability: None,
            motor_capability: None,
            visualization_capability: None,
            custom_capabilities: Vec::new(),
        }
    }

    fn with_registration_endpoint(mut self, endpoint: String) -> Self {
        self.registration_endpoint = endpoint;
        self
    }

    fn with_sensory_endpoint(mut self, endpoint: String) -> Self {
        self.sensory_endpoint = endpoint;
        self
    }

    fn with_motor_endpoint(mut self, endpoint: String) -> Self {
        self.motor_endpoint = endpoint;
        self
    }

    fn with_visualization_endpoint(mut self, endpoint: String) -> Self {
        self.visualization_endpoint = endpoint;
        self
    }

    fn with_control_endpoint(mut self, endpoint: String) -> Self {
        self.control_endpoint = endpoint;
        self
    }

    fn with_feagi_endpoints(
        mut self,
        host: String,
        registration_port: u16,
        sensory_port: u16,
        motor_port: u16,
        visualization_port: u16,
        control_port: u16,
    ) -> Self {
        self.registration_endpoint = format!("tcp://{}:{}", host, registration_port);
        self.sensory_endpoint = format!("tcp://{}:{}", host, sensory_port);
        self.motor_endpoint = format!("tcp://{}:{}", host, motor_port);
        self.visualization_endpoint = format!("tcp://{}:{}", host, visualization_port);
        self.control_endpoint = format!("tcp://{}:{}", host, control_port);
        self
    }

    fn with_heartbeat_interval(mut self, heartbeat_interval_s: f64) -> Self {
        self.heartbeat_interval_s = Some(heartbeat_interval_s);
        self
    }

    fn with_connection_timeout_ms(mut self, connection_timeout_ms: u64) -> Self {
        self.connection_timeout_ms = Some(connection_timeout_ms);
        self
    }

    fn with_registration_retries(mut self, registration_retries: u32) -> Self {
        self.registration_retries = Some(registration_retries);
        self
    }

    fn with_retry_backoff_ms(mut self, retry_backoff_ms: u64) -> Self {
        self.retry_backoff_ms = Some(retry_backoff_ms);
        self
    }

    fn with_sensory_socket_config(mut self, send_hwm: i32, linger_ms: i32, immediate: bool) -> Self {
        self.sensory_send_hwm = send_hwm;
        self.sensory_linger_ms = linger_ms;
        self.sensory_immediate = immediate;
        self
    }

    fn with_sensory_capability(mut self, rate_hz: f64, shm_path: Option<String>) -> Self {
        self.sensory_capability = Some(serde_json::json!({
            "rate_hz": rate_hz,
            "shm_path": shm_path,
        }));
        self
    }

    fn with_vision_capability(
        mut self,
        modality: String,
        dimensions: (usize, usize),
        channels: usize,
        target_cortical_area: String,
    ) -> Self {
        self.vision_capability = Some(serde_json::json!({
            "modality": modality,
            "dimensions": [dimensions.0, dimensions.1],
            "channels": channels,
            "target_cortical_area": target_cortical_area,
        }));
        self
    }

    fn with_vision_unit(
        mut self,
        modality: String,
        dimensions: (usize, usize),
        channels: usize,
        unit: String,
        group: u8,
    ) -> Self {
        self.vision_capability = Some(serde_json::json!({
            "modality": modality,
            "dimensions": [dimensions.0, dimensions.1],
            "channels": channels,
            "unit": unit,
            "group": group,
        }));
        self
    }

    fn with_motor_capability(
        mut self,
        modality: String,
        output_count: usize,
        source_cortical_areas: Vec<String>,
    ) -> Self {
        self.motor_capability = Some(serde_json::json!({
            "modality": modality,
            "output_count": output_count,
            "source_cortical_areas": source_cortical_areas,
        }));
        self
    }

    fn with_motor_unit(
        mut self,
        modality: String,
        output_count: usize,
        unit: String,
        group: u8,
    ) -> Self {
        self.motor_capability = Some(serde_json::json!({
            "modality": modality,
            "output_count": output_count,
            "source_units": [{"unit": unit, "group": group}],
        }));
        self
    }

    fn with_motor_units(
        mut self,
        modality: String,
        output_count: usize,
        source_units: Vec<MotorUnitCompat>,
    ) -> Self {
        self.motor_capability = Some(serde_json::json!({
            "modality": modality,
            "output_count": output_count,
            "source_units": source_units,
        }));
        self
    }

    fn with_visualization_capability(
        mut self,
        visualization_type: String,
        resolution: Option<(usize, usize)>,
        refresh_rate_hz: Option<f64>,
        bridge_proxy: bool,
    ) -> Self {
        self.visualization_capability = Some(serde_json::json!({
            "visualization_type": visualization_type,
            "resolution": resolution.map(|(w, h)| vec![w, h]),
            "refresh_rate_hz": refresh_rate_hz,
            "bridge_proxy": bridge_proxy,
        }));
        self
    }

    fn with_custom_capability(mut self, key: String, value: serde_json::Value) -> Self {
        self.custom_capabilities.push((key, value));
        self
    }

    fn validate(&self) -> Result<(), String> {
        if self.agent_id.trim().is_empty() {
            return Err("agent_id must not be empty".to_string());
        }
        if self.registration_endpoint.trim().is_empty() {
            return Err("registration_endpoint must be set".to_string());
        }
        if matches!(self.agent_type, AgentType::Sensory | AgentType::Both)
            && self.sensory_endpoint.trim().is_empty()
        {
            return Err("sensory_endpoint must be set for sensory/both".to_string());
        }
        if matches!(self.agent_type, AgentType::Motor | AgentType::Both)
            && self.motor_endpoint.trim().is_empty()
        {
            return Err("motor_endpoint must be set for motor/both".to_string());
        }
        if self.descriptor.is_none() {
            return Err("agent descriptor must be set".to_string());
        }
        if self.auth_token.is_none() {
            return Err("auth token must be set".to_string());
        }
        if self.connection_timeout_ms.unwrap_or(0) == 0 {
            return Err("connection_timeout_ms must be set and > 0".to_string());
        }
        if self.registration_retries.unwrap_or(0) == 0 {
            return Err("registration_retries must be set and > 0".to_string());
        }
        Ok(())
    }
}

struct AgentClient {
    config: AgentConfig,
    command_control: Option<CommandControlAgent>,
    sensory_client: Option<Box<dyn FeagiClientPusher>>,
    motor_client: Option<Box<dyn FeagiClientSubscriber>>,
    registration_body_json: Option<serde_json::Value>,
}

impl AgentClient {
    fn new(config: AgentConfig) -> Result<Self, String> {
        config.validate()?;
        Ok(Self {
            config,
            command_control: None,
            sensory_client: None,
            motor_client: None,
            registration_body_json: None,
        })
    }

    fn parse_endpoint(endpoint: &str) -> Result<TransportProtocolEndpoint, String> {
        if endpoint.starts_with("tcp://") {
            let parsed = ZmqUrl::new(endpoint).map_err(|e| e.to_string())?;
            Ok(TransportProtocolEndpoint::Zmq(parsed))
        } else if endpoint.starts_with("ws://") || endpoint.starts_with("wss://") {
            let parsed = WebSocketUrl::new(endpoint).map_err(|e| e.to_string())?;
            Ok(TransportProtocolEndpoint::WebSocket(parsed))
        } else {
            Err(
                "Unsupported endpoint scheme; expected tcp://, ws://, or wss://".to_string(),
            )
        }
    }

    fn requested_capabilities(agent_type: AgentType) -> Vec<AgentCapabilities> {
        match agent_type {
            AgentType::Sensory => vec![AgentCapabilities::SendSensorData],
            AgentType::Motor => vec![AgentCapabilities::ReceiveMotorData],
            AgentType::Both => vec![
                AgentCapabilities::SendSensorData,
                AgentCapabilities::ReceiveMotorData,
            ],
            AgentType::Visualization => vec![AgentCapabilities::ReceiveNeuronVisualizations],
            AgentType::Infrastructure => vec![AgentCapabilities::ReceiveSystemMessages],
        }
    }

    fn connect(&mut self) -> Result<(), String> {
        let registration_endpoint = Self::parse_endpoint(&self.config.registration_endpoint)?;
        let requester_props = registration_endpoint
            .try_create_boxed_client_requester_properties()
            .map_err(|e| e.to_string())?;
        let mut control = CommandControlAgent::new(requester_props);
        control.request_connect().map_err(|e| e.to_string())?;

        let descriptor = self
            .config
            .descriptor
            .clone()
            .ok_or_else(|| "agent descriptor must be set".to_string())?;
        let auth_token = self
            .config
            .auth_token
            .clone()
            .ok_or_else(|| "auth token must be set".to_string())?;
        let timeout_ms = self
            .config
            .connection_timeout_ms
            .ok_or_else(|| "connection_timeout_ms must be set".to_string())?;
        let retries = self
            .config
            .registration_retries
            .ok_or_else(|| "registration_retries must be set".to_string())?;
        let deadline =
            Instant::now() + Duration::from_millis(timeout_ms.saturating_mul(retries as u64).max(1));
        let requested_capabilities = Self::requested_capabilities(self.config.agent_type);

        let mut sent_registration = false;
        let mut session_id: Option<AgentID> = None;
        let mut endpoint_map: Option<HashMap<AgentCapabilities, TransportProtocolEndpoint>> = None;

        while Instant::now() < deadline {
            let (state, _) = control.poll_for_messages().map_err(|e| e.to_string())?;
            if !sent_registration
                && matches!(
                    state,
                    FeagiEndpointState::ActiveWaiting | FeagiEndpointState::ActiveHasData
                )
            {
                control
                    .request_registration(
                        descriptor.clone(),
                        auth_token.clone(),
                        requested_capabilities.clone(),
                    )
                    .map_err(|e| e.to_string())?;
                sent_registration = true;
            }
            if let AgentRegistrationStatus::Registered(id, endpoints) = control.registration_status() {
                session_id = Some(*id);
                endpoint_map = Some(endpoints.clone());
                break;
            }
            std::thread::yield_now();
        }

        let _session_id = session_id.ok_or_else(|| "registration timed out".to_string())?;
        let endpoint_map = endpoint_map.ok_or_else(|| "missing endpoint map".to_string())?;

        if matches!(self.config.agent_type, AgentType::Sensory | AgentType::Both) {
            let sensory_endpoint = endpoint_map
                .get(&AgentCapabilities::SendSensorData)
                .ok_or_else(|| "missing sensory endpoint from registration".to_string())?;
            let props = sensory_endpoint
                .try_create_boxed_client_pusher_properties()
                .map_err(|e| e.to_string())?;
            let mut pusher = props.as_boxed_client_pusher();
            pusher.request_connect().map_err(|e| e.to_string())?;
            self.sensory_client = Some(pusher);
        }

        if matches!(self.config.agent_type, AgentType::Motor | AgentType::Both) {
            let motor_endpoint = endpoint_map
                .get(&AgentCapabilities::ReceiveMotorData)
                .ok_or_else(|| "missing motor endpoint from registration".to_string())?;
            let props = motor_endpoint
                .try_create_boxed_client_subscriber_properties()
                .map_err(|e| e.to_string())?;
            let mut subscriber = props.as_boxed_client_subscriber();
            subscriber.request_connect().map_err(|e| e.to_string())?;
            self.motor_client = Some(subscriber);
        }

        self.registration_body_json = Some(build_registration_json(&endpoint_map));
        self.command_control = Some(control);
        Ok(())
    }

    fn registration_body_json(&self) -> Option<&serde_json::Value> {
        self.registration_body_json.as_ref()
    }

    fn send_sensory_bytes(&mut self, bytes: Vec<u8>) -> Result<(), String> {
        let sensory_client = self
            .sensory_client
            .as_mut()
            .ok_or_else(|| "sensory channel is not available".to_string())?;
        match sensory_client.poll() {
            FeagiEndpointState::ActiveWaiting => {}
            FeagiEndpointState::ActiveHasData => {
                return Err("sensory endpoint is in ActiveHasData state".to_string());
            }
            FeagiEndpointState::Pending => return Err("sensory endpoint is pending".to_string()),
            FeagiEndpointState::Inactive => return Err("sensory endpoint is inactive".to_string()),
            FeagiEndpointState::Errored(err) => {
                return Err(format!("sensory endpoint errored: {}", err));
            }
        }
        sensory_client.publish_data(&bytes).map_err(|e| e.to_string())
    }

    fn try_send_sensory_bytes(&mut self, bytes: &[u8]) -> Result<bool, String> {
        let sensory_client = self
            .sensory_client
            .as_mut()
            .ok_or_else(|| "sensory channel is not available".to_string())?;
        match sensory_client.poll() {
            FeagiEndpointState::ActiveWaiting => {
                sensory_client.publish_data(bytes).map_err(|e| e.to_string())?;
                Ok(true)
            }
            FeagiEndpointState::Pending
            | FeagiEndpointState::Inactive
            | FeagiEndpointState::ActiveHasData => Ok(false),
            FeagiEndpointState::Errored(err) => Err(format!("sensory endpoint errored: {}", err)),
        }
    }

    fn receive_motor_data(&mut self) -> Result<Option<Vec<u8>>, String> {
        let motor_client = self
            .motor_client
            .as_mut()
            .ok_or_else(|| "motor channel is not available".to_string())?;
        match motor_client.poll() {
            FeagiEndpointState::ActiveHasData => motor_client
                .consume_retrieved_data()
                .map(|v| Some(v.to_vec()))
                .map_err(|e| e.to_string()),
            FeagiEndpointState::ActiveWaiting
            | FeagiEndpointState::Pending
            | FeagiEndpointState::Inactive => Ok(None),
            FeagiEndpointState::Errored(err) => Err(format!("motor endpoint errored: {}", err)),
        }
    }
}

#[derive(Clone, Debug)]
struct RegistrationResponse {
    recommended_transport: Option<String>,
    transports: HashMap<String, TransportConfig>,
}

#[derive(Clone, Debug, Serialize)]
struct TransportConfig {
    ports: HashMap<String, u16>,
}

impl RegistrationResponse {
    fn from_json(body: &serde_json::Value) -> Result<Self, String> {
        let recommended_transport = body
            .get("recommended_transport")
            .and_then(|v| v.as_str())
            .map(ToString::to_string);

        let transports_obj = body
            .get("transports")
            .and_then(|v| v.as_object())
            .ok_or_else(|| "registration body missing transports".to_string())?;
        let mut transports = HashMap::new();
        for (name, value) in transports_obj {
            let ports_obj = value
                .get("ports")
                .and_then(|v| v.as_object())
                .ok_or_else(|| format!("transport '{}' missing ports object", name))?;
            let mut ports = HashMap::new();
            for (port_name, port_value) in ports_obj {
                let Some(port_u64) = port_value.as_u64() else {
                    return Err(format!("transport '{}.{}' port is not an integer", name, port_name));
                };
                if port_u64 > u16::MAX as u64 {
                    return Err(format!("transport '{}.{}' port out of range", name, port_name));
                }
                ports.insert(port_name.clone(), port_u64 as u16);
            }
            transports.insert(name.clone(), TransportConfig { ports });
        }

        Ok(Self {
            recommended_transport,
            transports,
        })
    }

    fn get_transport(&self, name: &str) -> Option<&TransportConfig> {
        self.transports.get(name)
    }

    fn choose_transport(&self, preference: Option<&str>) -> Option<serde_json::Value> {
        if let Some(pref) = preference {
            if let Some(cfg) = self.transports.get(pref) {
                return Some(serde_json::json!({
                    "name": pref,
                    "ports": cfg.ports,
                }));
            }
        }
        let recommended = self.recommended_transport.as_deref()?;
        self.transports.get(recommended).map(|cfg| {
            serde_json::json!({
                "name": recommended,
                "ports": cfg.ports,
            })
        })
    }
}

fn parse_endpoint_port(endpoint: &TransportProtocolEndpoint) -> Option<u16> {
    let text = match endpoint {
        TransportProtocolEndpoint::Zmq(url) => url.as_str(),
        TransportProtocolEndpoint::WebSocket(url) => url.as_str(),
    };
    let port_str = text.rsplit(':').next()?;
    port_str.parse::<u16>().ok()
}

fn build_registration_json(
    endpoint_map: &HashMap<AgentCapabilities, TransportProtocolEndpoint>,
) -> serde_json::Value {
    let mut transports: serde_json::Map<String, serde_json::Value> = serde_json::Map::new();
    let mut zmq_ports: serde_json::Map<String, serde_json::Value> = serde_json::Map::new();
    let mut ws_ports: serde_json::Map<String, serde_json::Value> = serde_json::Map::new();

    for (capability, endpoint) in endpoint_map {
        let key = match capability {
            AgentCapabilities::SendSensorData => "sensory",
            AgentCapabilities::ReceiveMotorData => "motor",
            AgentCapabilities::ReceiveNeuronVisualizations => "visualization",
            AgentCapabilities::ReceiveSystemMessages => "control",
        };
        if let Some(port) = parse_endpoint_port(endpoint) {
            match endpoint {
                TransportProtocolEndpoint::Zmq(_) => {
                    zmq_ports.insert(key.to_string(), serde_json::json!(port));
                }
                TransportProtocolEndpoint::WebSocket(_) => {
                    ws_ports.insert(key.to_string(), serde_json::json!(port));
                }
            }
        }
    }

    if !zmq_ports.is_empty() {
        transports.insert(
            "zmq".to_string(),
            serde_json::json!({
                "ports": serde_json::Value::Object(zmq_ports),
            }),
        );
    }
    if !ws_ports.is_empty() {
        transports.insert(
            "websocket".to_string(),
            serde_json::json!({
                "ports": serde_json::Value::Object(ws_ports),
            }),
        );
    }

    let recommended_transport = if transports.contains_key("websocket") {
        "websocket"
    } else {
        "zmq"
    };

    serde_json::json!({
        "recommended_transport": recommended_transport,
        "transports": serde_json::Value::Object(transports),
    })
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
pub extern "C" fn feagi_config_set_agent_descriptor(
    cfg: *mut FeagiAgentConfigHandle,
    manufacturer: *const c_char,
    agent_name: *const c_char,
    agent_version: u32,
) -> FeagiStatus {
    clear_last_error();
    if cfg.is_null() {
        set_last_error("cfg must not be null");
        return FeagiStatus::NullPointer;
    }
    let Ok(manufacturer) = cstr_to_string(manufacturer, "manufacturer") else {
        return FeagiStatus::InvalidUtf8;
    };
    let Ok(agent_name) = cstr_to_string(agent_name, "agent_name") else {
        return FeagiStatus::InvalidUtf8;
    };
    if agent_version == 0 {
        set_last_error("agent_version must be > 0");
        return FeagiStatus::InvalidArgument;
    }
    let descriptor = match AgentDescriptor::new(&manufacturer, &agent_name, agent_version) {
        Ok(v) => v,
        Err(e) => {
            set_last_error(e.to_string());
            return FeagiStatus::InvalidArgument;
        }
    };
    unsafe {
        (*cfg).config.descriptor = Some(descriptor);
    }
    FeagiStatus::Ok
}

#[no_mangle]
pub extern "C" fn feagi_config_set_auth_token_base64(
    cfg: *mut FeagiAgentConfigHandle,
    auth_token_b64: *const c_char,
) -> FeagiStatus {
    clear_last_error();
    if cfg.is_null() {
        set_last_error("cfg must not be null");
        return FeagiStatus::NullPointer;
    }
    let Ok(auth_token_b64) = cstr_to_string(auth_token_b64, "auth_token_b64") else {
        return FeagiStatus::InvalidUtf8;
    };
    let Some(auth_token) = AuthToken::from_base64(&auth_token_b64) else {
        set_last_error("auth_token_b64 must decode to exactly 32 bytes");
        return FeagiStatus::InvalidArgument;
    };
    unsafe {
        (*cfg).config.auth_token = Some(auth_token);
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
            .with_vision_unit(
                modality,
                (width, height),
                channels,
                unit.as_contract_str().to_string(),
                group,
            );
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
            .with_motor_unit(
                modality,
                output_count,
                unit.as_contract_str().to_string(),
                group,
            );
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
    let units: Vec<MotorUnitCompat> = match serde_json::from_str(&json_str) {
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
        let manufacturer = CString::new("neuraville").unwrap();
        let agent_name = CString::new("java_viz").unwrap();
        let auth_b64 = CString::new("BwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwc=").unwrap();

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
            feagi_config_set_connection_timeout_ms(cfg, 1000),
            FeagiStatus::Ok
        );
        assert_eq!(
            feagi_config_set_registration_retries(cfg, 3),
            FeagiStatus::Ok
        );
        assert_eq!(
            feagi_config_set_agent_descriptor(cfg, manufacturer.as_ptr(), agent_name.as_ptr(), 1),
            FeagiStatus::Ok
        );
        assert_eq!(
            feagi_config_set_auth_token_base64(cfg, auth_b64.as_ptr()),
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

    let json = match serde_json::to_string(&chosen) {
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

    let Some(buffer) = maybe else {
        unsafe {
            *out_has_data = false;
            *out_buf = ptr::null_mut();
        }
        return FeagiStatus::Ok;
    };

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

