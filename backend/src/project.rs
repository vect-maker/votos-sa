use crate::cli::ProjectArgs;
use crate::constants::*;
use nle_cloud_sdk::models::{
    ActuatorAddUpdate, DeviceAddUpdateDto, DeviceQueryParams, ProjectAddUpdateDto,
    ProjectQueryParams, SensorAddUpdate,
};

use anyhow::{bail, Result as AnyResult};

pub async fn get_client(args: &ProjectArgs) -> AnyResult<NleCloudClient> {
    let env_base_url = std::env::var("NLE_BASE_URL").ok();
    let base_url = args
        .base_url
        .as_deref()
        .or(env_base_url.as_deref())
        .unwrap_or(DEFAULT_BASE_URL);

    let client = NleCloudClient::builder().base_url(base_url).build()?;

    // 1. Explicit token from arguments
    if let Some(token) = &args.token {
        if !token.trim().is_empty() {
            return Ok(client.with_token(token));
        }
    }

    // 2. Token from environment variable
    if let Ok(tok) = std::env::var("NLE_TOKEN").or_else(|_| std::env::var("NLECLOUD_TOKEN")) {
        if !tok.trim().is_empty() {
            return Ok(client.with_token(tok));
        }
    }

    // 3. Credentials from arguments or environment
    let account = args
        .account
        .clone()
        .or_else(|| std::env::var("NLE_ACCOUNT").ok())
        .or_else(|| std::env::var("NLECLOUD_ACCOUNT").ok());

    let password = args
        .password
        .clone()
        .or_else(|| std::env::var("NLE_PASSWORD").ok())
        .or_else(|| std::env::var("NLECLOUD_PASSWORD").ok());

    match (account, password) {
        (Some(acc), Some(pwd)) if !acc.trim().is_empty() && !pwd.trim().is_empty() => {
            tracing::info!("Logging into NLECloud as '{acc}'...");
            let login_res = client.login_with_credentials(&acc, &pwd, false).await?;
            tracing::info!("Successfully authenticated with NLECloud.");
            Ok(client.with_token(login_res.access_token))
        }
        _ => bail!("NLECloud credentials required. Please set NLE_ACCOUNT and NLE_PASSWORD in your .env file (see .env.example) or provide them via CLI options (--account, --password)."),
    }
}

/// Resolves a globally unique device name and tag using an optional namespace or project_id.
/// NLECloud requires Tag to match `^[a-zA-Z0-9_]{6,30}$` and Name to be 6..=15 characters.
pub fn resolve_device_name_and_tag(
    namespace: Option<&str>,
    project_id: i32,
    base_name: &str,
) -> (String, String) {
    let ns = namespace
        .filter(|s| !s.trim().is_empty())
        .map(|s| s.trim().replace('-', "_"))
        .unwrap_or_else(|| format!("p{}", project_id));

    let clean_base = base_name.replace('-', "_");

    // Tag: 6 to 30 chars, alphanumeric and underscore
    let mut tag = format!("{}_{}", ns, clean_base);
    if tag.len() > 30 {
        tag.truncate(30);
    }
    while tag.len() < 6 {
        tag.push('_');
    }

    // Name: 6 to 15 chars (NLECloud API limit)
    let suffix = format!("_{}", clean_base);
    let mut name = if ns.len() + suffix.len() <= 15 {
        format!("{}{}", ns, suffix)
    } else {
        let max_ns = 15usize.saturating_sub(suffix.len()).max(1);
        let prefix: String = ns.chars().take(max_ns).collect();
        format!("{}{}", prefix, suffix)
    };
    if name.len() > 15 {
        name.truncate(15);
    }
    while name.len() < 6 {
        name.push('1');
    }

    (name, tag)
}

/// Provisions a device within a project and populates its sensors/actuators.
/// This function is generalized so it can be called for multiple devices in the future.
pub async fn provision_device(
    client: &NleCloudClient,
    project_id: i32,
    device_name: &str,
    device_tag: &str,
) -> AnyResult<i32> {
    tracing::info!("Checking for existing device (Name: '{device_name}', Tag: '{device_tag}') in project {project_id}...");
    let query = DeviceQueryParams::builder()
        .tag(device_tag)
        .project_key_word(project_id.to_string())
        .page_size(100)
        .build();

    let paged = client.get_devices(&query, None).await?;
    let existing = paged
        .page_set
        .into_iter()
        .find(|d| d.tag.as_deref() == Some(device_tag) || d.name.as_deref() == Some(device_name));

    let device_id = match existing {
        Some(d) => {
            println!(
                "Device '{}' already exists (ID: {}, Tag: {}).",
                device_name,
                d.device_id,
                d.tag.as_deref().unwrap_or("N/A")
            );
            d.device_id
        }
        None => {
            println!("Device '{device_name}' (Tag: '{device_tag}') not found. Provisioning new device...");
            let dto = DeviceAddUpdateDto::builder()
                .project_id_or_tag(project_id.to_string())
                .name(device_name)
                .tag(device_tag)
                .protocol(DeviceProtocol::Tcp)
                .build();
            let id = client.add_device(&dto, None).await?;
            println!("Successfully provisioned device '{device_name}' (ID: {id}, Tag: '{device_tag}')");
            id
        }
    };

    // Populate all sensors and actuators for this device
    populate_device_peripherals(client, device_id, device_name).await?;

    Ok(device_id)
}

/// Populates all sensors and actuators for a device in an idempotent manner.
pub async fn populate_device_peripherals(
    client: &NleCloudClient,
    device_id: i32,
    device_name: &str,
) -> AnyResult<()> {
    // 1. Light LDR Sensor (brightness, float, flux)
    let brightness_exists = client
        .get_sensor_info(device_id, TAG_BRIGHTNESS, None)
        .await
        .is_ok();

    if brightness_exists {
        println!("Sensor '{TAG_BRIGHTNESS}' already exists on device '{device_name}'.");
    } else {
        println!("Adding sensor '{TAG_BRIGHTNESS}' (LDR, Float, {SENSOR_UNIT_FLUX}) to device '{device_name}'...");
        let brightness_sensor = SensorAddUpdate::builder()
            .name(SENSOR_NAME_BRIGHTNESS)
            .api_tag(TAG_BRIGHTNESS)
            .trans_type(TransType::ReportOnly)
            .data_type(DataType::Float)
            .type_attrs(SENSOR_TYPE_LDR)
            .unit(SENSOR_UNIT_FLUX)
            .precision(2)
            .build();
        client.add_sensor(device_id, &brightness_sensor, None).await?;
        println!("Successfully added sensor '{TAG_BRIGHTNESS}' to device '{device_name}'.");
    }

    // 2. Servo X Actuator (Scale 0..180 deg)
    let servo_x_exists = client
        .get_sensor_info(device_id, TAG_SERVO_X, None)
        .await
        .is_ok();

    if servo_x_exists {
        println!("Actuator '{TAG_SERVO_X}' already exists on device '{device_name}'.");
    } else {
        println!("Adding actuator '{TAG_SERVO_X}' (Scale, 0-{SERVO_MAX_ANGLE}{SERVO_UNIT_DEGREE}) to device '{device_name}'...");
        let servo_x = ActuatorAddUpdate::builder()
            .name(ACTUATOR_NAME_SERVO_X)
            .api_tag(TAG_SERVO_X)
            .trans_type(TransType::ReportAndControl)
            .data_type(DataType::Float)
            .oper_type(ActuatorOperType::Scale)
            .serial_number(1)
            .build();
        client.add_sensor(device_id, &servo_x, None).await?;
        println!("Successfully added actuator '{TAG_SERVO_X}' to device '{device_name}'.");
    }

    // 3. Servo Y Actuator (Scale 0..180 deg)
    let servo_y_exists = client
        .get_sensor_info(device_id, TAG_SERVO_Y, None)
        .await
        .is_ok();

    if servo_y_exists {
        println!("Actuator '{TAG_SERVO_Y}' already exists on device '{device_name}'.");
    } else {
        println!("Adding actuator '{TAG_SERVO_Y}' (Scale, 0-{SERVO_MAX_ANGLE}{SERVO_UNIT_DEGREE}) to device '{device_name}'...");
        let servo_y = ActuatorAddUpdate::builder()
            .name(ACTUATOR_NAME_SERVO_Y)
            .api_tag(TAG_SERVO_Y)
            .trans_type(TransType::ReportAndControl)
            .data_type(DataType::Float)
            .oper_type(ActuatorOperType::Scale)
            .serial_number(2)
            .build();
        client.add_sensor(device_id, &servo_y, None).await?;
        println!("Successfully added actuator '{TAG_SERVO_Y}' to device '{device_name}'.");
    }

    // 4. Boolean Switch Actuators (lamp, fan, lock)
    let boolean_actuators = [
        (TAG_LAMP, ACTUATOR_NAME_LAMP, 3),
        (TAG_FAN, ACTUATOR_NAME_FAN, 4),
        (TAG_LOCK, ACTUATOR_NAME_LOCK, 5),
    ];

    for (tag, name, serial) in boolean_actuators {
        let exists = client.get_sensor_info(device_id, tag, None).await.is_ok();
        if exists {
            println!("Actuator '{tag}' already exists on device '{device_name}'.");
        } else {
            println!("Adding boolean actuator '{tag}' (Switch) to device '{device_name}'...");
            let actuator = ActuatorAddUpdate::builder()
                .name(name)
                .api_tag(tag)
                .trans_type(TransType::ReportAndControl)
                .data_type(DataType::Boolean)
                .oper_type(ActuatorOperType::Switch)
                .serial_number(serial)
                .build();
            client.add_sensor(device_id, &actuator, None).await?;
            println!("Successfully added actuator '{tag}' to device '{device_name}'.");
        }
    }

    Ok(())
}

pub async fn provision(args: &ProjectArgs) -> AnyResult<()> {
    let project_name = args.resolved_name();
    let client = get_client(args).await?;

    tracing::info!("Searching for existing project '{project_name}'...");
    let query = ProjectQueryParams::builder()
        .keyword(project_name.clone())
        .page_size(100)
        .build();

    let paged = client.get_projects(&query, None).await?;
    let existing = paged
        .page_set
        .into_iter()
        .find(|p| p.name.as_deref() == Some(&project_name));

    let project_id = match existing {
        Some(p) => {
            println!(
                "Project '{}' already exists (ID: {}, Tag: {}).",
                project_name,
                p.project_id,
                p.project_tag.as_deref().unwrap_or("N/A")
            );
            p.project_id
        }
        None => {
            println!("Project '{project_name}' not found. Provisioning new project...");
            let industry = IndustryKind::try_from(args.industry).unwrap_or(IndustryKind::SmartHome);
            let net_work_kind = NetworkKind::try_from(args.network_kind).unwrap_or(NetworkKind::Wifi);
            let dto = ProjectAddUpdateDto::builder()
                .name(&project_name)
                .industry(industry)
                .net_work_kind(net_work_kind)
                .build();
            let id = client.add_project(&dto, None).await?;
            println!("Successfully provisioned project '{project_name}' (ID: {id})");
            id
        }
    };

    // Resolve unique namespaced device name and tag to avoid platform-wide collisions
    let (device_name, device_tag) = resolve_device_name_and_tag(
        args.device_namespace.as_deref(),
        project_id,
        DEFAULT_DEVICE_BASE_NAME,
    );

    // Provision the device and all its sensors/actuators
    provision_device(&client, project_id, &device_name, &device_tag).await?;

    println!("Project '{project_name}' provisioning complete.");
    Ok(())
}

pub async fn delete(args: &ProjectArgs) -> AnyResult<()> {
    let project_name = args.resolved_name();
    let client = get_client(args).await?;

    tracing::info!("Searching for project '{project_name}' to delete...");
    let query = ProjectQueryParams::builder()
        .keyword(project_name.clone())
        .page_size(100)
        .build();

    let paged = client.get_projects(&query, None).await?;
    let matching: Vec<_> = paged
        .page_set
        .into_iter()
        .filter(|p| p.name.as_deref() == Some(&project_name))
        .collect();

    if matching.is_empty() {
        println!("Project '{project_name}' was not found. Nothing to delete.");
        return Ok(());
    }

    let ids: Vec<i32> = matching.iter().map(|p| p.project_id).collect();
    client.delete_projects(&ids, None).await?;

    println!(
        "Successfully deleted project '{}' (ID(s): {:?})",
        project_name, ids
    );
    Ok(())
}
