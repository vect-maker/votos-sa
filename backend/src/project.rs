use crate::cli::ProjectArgs;
use crate::constants::*;
use nle_cloud_sdk::models::{
    DeviceAddUpdateDto, DeviceFuzzyQryPagingParas, ProjectAddUpdateDto, ProjectFuzzyQryPagingParas,
    SensorAddUpdate,
};
use nle_cloud_sdk::prelude::*;

type AnyResult<T> = std::result::Result<T, Box<dyn std::error::Error>>;

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
        _ => Err("NLECloud credentials required. Please set NLE_ACCOUNT and NLE_PASSWORD in your .env file (see .env.example) or provide them via CLI options (--account, --password).".into()),
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
    let query = DeviceFuzzyQryPagingParas {
        tag: Some(device_tag.to_string()),
        project_key_word: Some(project_id.to_string()),
        page_size: Some(100),
        ..Default::default()
    };

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
            let dto = DeviceAddUpdateDto::new(
                project_id.to_string(),
                device_name,
                device_tag,
                DeviceProtocol::Tcp.into(),
            );
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
    // 1. Check if the brightness sensor already exists
    let sensor_exists = client
        .get_sensor_info(device_id, TAG_BRIGHTNESS, None)
        .await
        .is_ok();

    // 2. Light LDR Sensor (brightness, float, flux)
    if sensor_exists {
        println!("Sensor '{TAG_BRIGHTNESS}' already exists on device '{device_name}'.");
    } else {
        println!("Adding sensor '{TAG_BRIGHTNESS}' (LDR, Float, {SENSOR_UNIT_FLUX}) to device '{device_name}'...");
        let brightness_sensor = SensorAddUpdate {
            name: SENSOR_NAME_BRIGHTNESS.to_string(),
            api_tag: TAG_BRIGHTNESS.to_string(),
            trans_type: TransType::ReportOnly.into(),
            data_type: DataType::Float.into(),
            type_attrs: Some(SENSOR_TYPE_LDR.to_string()),
            unit: Some(SENSOR_UNIT_FLUX.to_string()),
            precision: 2,
        };
        client.add_sensor(device_id, &brightness_sensor, None).await?;
        println!("Successfully added sensor '{TAG_BRIGHTNESS}' to device '{device_name}'.");
    }

    Ok(())
}

pub async fn provision(args: &ProjectArgs) -> AnyResult<()> {
    let project_name = args.resolved_name();
    let client = get_client(args).await?;

    tracing::info!("Searching for existing project '{project_name}'...");
    let query = ProjectFuzzyQryPagingParas {
        keyword: Some(project_name.clone()),
        page_size: Some(100),
        ..Default::default()
    };

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
            let dto = ProjectAddUpdateDto::new(&project_name, args.industry, args.network_kind);
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
    let query = ProjectFuzzyQryPagingParas {
        keyword: Some(project_name.clone()),
        page_size: Some(100),
        ..Default::default()
    };

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
