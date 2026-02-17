use feagi_java_ffi::*;
use std::ffi::CString;

fn cstr(value: &str) -> CString {
    CString::new(value).unwrap()
}

#[test]
fn test_config_set_vision_unit() {
    let agent_id = cstr("vision_agent");
    let registration = cstr("tcp://feagi.invalid:30001");
    let sensory = cstr("tcp://feagi.invalid:5558");
    let modality = cstr("camera");
    let manufacturer = cstr("neuraville");
    let agent_name = cstr("java_vision");
    let auth_b64 = cstr("BwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwc=");

    let cfg = feagi_config_new(agent_id.as_ptr(), FeagiAgentType::Sensory);
    assert!(!cfg.is_null());

    assert_eq!(
        feagi_config_set_registration_endpoint(cfg, registration.as_ptr()),
        FeagiStatus::Ok
    );
    assert_eq!(
        feagi_config_set_sensory_endpoint(cfg, sensory.as_ptr()),
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
        feagi_config_set_vision_unit(
            cfg,
            modality.as_ptr(),
            640,
            480,
            3,
            FeagiSensoryUnit::Vision,
            0
        ),
        FeagiStatus::Ok
    );
    assert_eq!(feagi_config_validate(cfg), FeagiStatus::Ok);

    feagi_config_free(cfg);
}

#[test]
fn test_config_set_motor_units_json() {
    let agent_id = cstr("motor_agent");
    let registration = cstr("tcp://feagi.invalid:30001");
    let motor = cstr("tcp://feagi.invalid:5564");
    let modality = cstr("servo");
    let manufacturer = cstr("neuraville");
    let agent_name = cstr("java_motor");
    let auth_b64 = cstr("BwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwc=");
    let motor_units_json = cstr(
        r#"[{"unit":"rotary_motor","group":0},{"unit":"positional_servo","group":1}]"#,
    );

    let cfg = feagi_config_new(agent_id.as_ptr(), FeagiAgentType::Motor);
    assert!(!cfg.is_null());

    assert_eq!(
        feagi_config_set_registration_endpoint(cfg, registration.as_ptr()),
        FeagiStatus::Ok
    );
    assert_eq!(
        feagi_config_set_motor_endpoint(cfg, motor.as_ptr()),
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
        feagi_config_set_motor_units_json(cfg, modality.as_ptr(), 2, motor_units_json.as_ptr()),
        FeagiStatus::Ok
    );
    assert_eq!(feagi_config_validate(cfg), FeagiStatus::Ok);

    feagi_config_free(cfg);
}
