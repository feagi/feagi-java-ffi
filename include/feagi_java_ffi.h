// Copyright 2026 Neuraville Inc.
// SPDX-License-Identifier: Apache-2.0
//
// @cursor:ffi-safe
// C ABI header for feagi-java-ffi (JNI will call into these functions).

#pragma once

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef enum FeagiStatus {
  FEAGI_STATUS_OK = 0,
  FEAGI_STATUS_NULL_POINTER = 1,
  FEAGI_STATUS_INVALID_ARGUMENT = 2,
  FEAGI_STATUS_INVALID_UTF8 = 3,
  FEAGI_STATUS_JSON_ERROR = 4,
  FEAGI_STATUS_SDK_ERROR = 5,
  FEAGI_STATUS_ALLOCATION_FAILED = 6,
} FeagiStatus;

typedef enum FeagiAgentType {
  FEAGI_AGENT_TYPE_SENSORY = 0,
  FEAGI_AGENT_TYPE_MOTOR = 1,
  FEAGI_AGENT_TYPE_BOTH = 2,
  FEAGI_AGENT_TYPE_VISUALIZATION = 3,
  FEAGI_AGENT_TYPE_INFRASTRUCTURE = 4,
} FeagiAgentType;

typedef enum FeagiSensoryUnit {
  FEAGI_SENSORY_UNIT_INFRARED = 0,
  FEAGI_SENSORY_UNIT_PROXIMITY = 1,
  FEAGI_SENSORY_UNIT_SHOCK = 2,
  FEAGI_SENSORY_UNIT_BATTERY = 3,
  FEAGI_SENSORY_UNIT_SERVO = 4,
  FEAGI_SENSORY_UNIT_ANALOG_GPIO = 5,
  FEAGI_SENSORY_UNIT_DIGITAL_GPIO = 6,
  FEAGI_SENSORY_UNIT_MISC_DATA = 7,
  FEAGI_SENSORY_UNIT_TEXT_ENGLISH_INPUT = 8,
  FEAGI_SENSORY_UNIT_COUNT_INPUT = 9,
  FEAGI_SENSORY_UNIT_VISION = 10,
  FEAGI_SENSORY_UNIT_SEGMENTED_VISION = 11,
  FEAGI_SENSORY_UNIT_ACCELEROMETER = 12,
  FEAGI_SENSORY_UNIT_GYROSCOPE = 13,
} FeagiSensoryUnit;

typedef enum FeagiMotorUnit {
  FEAGI_MOTOR_UNIT_ROTARY_MOTOR = 0,
  FEAGI_MOTOR_UNIT_POSITIONAL_SERVO = 1,
  FEAGI_MOTOR_UNIT_GAZE = 2,
  FEAGI_MOTOR_UNIT_MISC_DATA = 3,
  FEAGI_MOTOR_UNIT_TEXT_ENGLISH_OUTPUT = 4,
  FEAGI_MOTOR_UNIT_COUNT_OUTPUT = 5,
  FEAGI_MOTOR_UNIT_OBJECT_SEGMENTATION = 6,
  FEAGI_MOTOR_UNIT_SIMPLE_VISION_OUTPUT = 7,
} FeagiMotorUnit;

typedef struct FeagiAgentConfigHandle FeagiAgentConfigHandle;
typedef struct FeagiAgentClientHandle FeagiAgentClientHandle;
typedef struct FeagiByteBufferHandle FeagiByteBufferHandle;

// Error reporting (per-thread)
char* feagi_last_error_message_alloc(void);
uint32_t feagi_abi_version(void);
char* feagi_library_version_alloc(void);
void feagi_string_free(char* s);

// Registration response (JSON)
char* feagi_client_registration_response_json_alloc(const FeagiAgentClientHandle* client);
char* feagi_client_registration_zmq_ports_json_alloc(const FeagiAgentClientHandle* client);
char* feagi_client_registration_chosen_transport_json_alloc(const FeagiAgentClientHandle* client, const char* preference_or_null);
char* feagi_client_registration_recommended_transport_alloc(const FeagiAgentClientHandle* client);

// Byte buffer helpers
const uint8_t* feagi_buffer_ptr(const FeagiByteBufferHandle* buf);
size_t feagi_buffer_len(const FeagiByteBufferHandle* buf);
void feagi_buffer_free(FeagiByteBufferHandle* buf);

// Config lifecycle
FeagiAgentConfigHandle* feagi_config_new(const char* agent_id, FeagiAgentType agent_type);
void feagi_config_free(FeagiAgentConfigHandle* cfg);

FeagiStatus feagi_config_set_registration_endpoint(FeagiAgentConfigHandle* cfg, const char* endpoint);
FeagiStatus feagi_config_set_sensory_endpoint(FeagiAgentConfigHandle* cfg, const char* endpoint);
FeagiStatus feagi_config_set_motor_endpoint(FeagiAgentConfigHandle* cfg, const char* endpoint);
FeagiStatus feagi_config_set_visualization_endpoint(FeagiAgentConfigHandle* cfg, const char* endpoint);
FeagiStatus feagi_config_set_control_endpoint(FeagiAgentConfigHandle* cfg, const char* endpoint);

FeagiStatus feagi_config_set_feagi_endpoints(
    FeagiAgentConfigHandle* cfg,
    const char* host,
    uint16_t registration_port,
    uint16_t sensory_port,
    uint16_t motor_port,
    uint16_t visualization_port,
    uint16_t control_port);

FeagiStatus feagi_config_set_heartbeat_interval_s(FeagiAgentConfigHandle* cfg, double heartbeat_interval_s);
FeagiStatus feagi_config_set_connection_timeout_ms(FeagiAgentConfigHandle* cfg, uint64_t connection_timeout_ms);
FeagiStatus feagi_config_set_registration_retries(FeagiAgentConfigHandle* cfg, uint32_t registration_retries);
FeagiStatus feagi_config_set_agent_descriptor(
    FeagiAgentConfigHandle* cfg,
    const char* manufacturer,
    const char* agent_name,
    uint32_t agent_version);
FeagiStatus feagi_config_set_auth_token_base64(FeagiAgentConfigHandle* cfg, const char* auth_token_b64);
FeagiStatus feagi_config_set_retry_backoff_ms(FeagiAgentConfigHandle* cfg, uint64_t retry_backoff_ms);
FeagiStatus feagi_config_set_sensory_socket_config(FeagiAgentConfigHandle* cfg, int32_t send_hwm, int32_t linger_ms, bool immediate);

// Capabilities (must satisfy config validation)
FeagiStatus feagi_config_set_sensory_capability(FeagiAgentConfigHandle* cfg, double rate_hz, const char* shm_path_or_null);
FeagiStatus feagi_config_set_vision_capability(
    FeagiAgentConfigHandle* cfg,
    const char* modality,
    size_t width,
    size_t height,
    size_t channels,
    const char* target_cortical_area);
FeagiStatus feagi_config_set_vision_unit(
    FeagiAgentConfigHandle* cfg,
    const char* modality,
    size_t width,
    size_t height,
    size_t channels,
    FeagiSensoryUnit unit,
    uint8_t group);
FeagiStatus feagi_config_set_motor_capability(
    FeagiAgentConfigHandle* cfg,
    const char* modality,
    size_t output_count,
    const char* source_cortical_areas_json);
FeagiStatus feagi_config_set_motor_unit(
    FeagiAgentConfigHandle* cfg,
    const char* modality,
    size_t output_count,
    FeagiMotorUnit unit,
    uint8_t group);
FeagiStatus feagi_config_set_motor_units_json(
    FeagiAgentConfigHandle* cfg,
    const char* modality,
    size_t output_count,
    const char* motor_units_json);
FeagiStatus feagi_config_set_visualization_capability(
    FeagiAgentConfigHandle* cfg,
    const char* visualization_type,
    bool has_resolution,
    size_t resolution_width,
    size_t resolution_height,
    bool has_refresh_rate,
    double refresh_rate_hz,
    bool bridge_proxy);
FeagiStatus feagi_config_set_custom_capability_json(FeagiAgentConfigHandle* cfg, const char* key, const char* json_value);

FeagiStatus feagi_config_validate(const FeagiAgentConfigHandle* cfg);

// Client lifecycle
FeagiStatus feagi_client_new(const FeagiAgentConfigHandle* cfg, FeagiAgentClientHandle** out_client);
void feagi_client_free(FeagiAgentClientHandle* client);

FeagiStatus feagi_client_connect(FeagiAgentClientHandle* client);

FeagiStatus feagi_client_send_sensory_bytes(FeagiAgentClientHandle* client, const uint8_t* bytes, size_t len);
FeagiStatus feagi_client_try_send_sensory_bytes(
    FeagiAgentClientHandle* client,
    const uint8_t* bytes,
    size_t len,
    bool* out_sent);

FeagiStatus feagi_client_receive_motor_buffer(
    FeagiAgentClientHandle* client,
    FeagiByteBufferHandle** out_buf,
    bool* out_has_data);

#ifdef __cplusplus
}  // extern "C"
#endif

