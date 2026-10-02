use crate::cli::ProjectArgs;
use crate::constants::*;
use crate::seed::DeviceSeed;
use anyhow::{bail, Result as AnyResult};
use nle_cloud_sdk::models::{
    ActuatorAddUpdate, DeviceAddUpdateDto, DeviceQueryParams, ProjectAddUpdateDto,
    ProjectQueryParams, SensorAddUpdate,
};
use std::collections::HashSet;

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
        _ => bail!("NLECloud credentials required. Please set NLE_ACCOUNT and NLE_PASSWORD in your .env file or provide them via CLI options (--account, --password)."),
    }
}

/// Resolves a device name and tag using an optional namespace or project_id fallback.
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

    // Avoid double prefixing if the name/tag already starts with the prefix
    let prefix = format!("{}_", ns);
    let effective_base = if clean_base.starts_with(&prefix) {
        &clean_base[prefix.len()..]
    } else if clean_base == ns {
        ""
    } else {
        clean_base.as_str()
    };

    // Tag: 6 to 30 chars, alphanumeric and underscore
    let mut tag = if effective_base.is_empty() {
        ns.clone()
    } else {
        format!("{}_{}", ns, effective_base)
    };
    if tag.len() > 30 {
        tag.truncate(30);
    }
    while tag.len() < 6 {
        tag.push('_');
    }

    // Name: 6 to 15 chars (NLECloud API limit)
    let suffix = if effective_base.is_empty() {
        String::new()
    } else {
        format!("_{}", effective_base)
    };
    let mut name = if ns.len() + suffix.len() <= 15 {
        format!("{}{}", ns, suffix)
    } else {
        let max_ns = 15usize.saturating_sub(suffix.len()).max(1);
        let p: String = ns.chars().take(max_ns).collect();
        format!("{}{}", p, suffix)
    };
    if name.len() > 15 {
        name.truncate(15);
    }
    while name.len() < 6 {
        name.push('1');
    }

    (name, tag)
}

/// Reconciles all sensors and actuators for a device against declarative seed definitions.
/// Deletes any peripherals in the cloud that are not defined in the JSON, and creates missing ones.
pub async fn reconcile_device_peripherals(
    client: &NleCloudClient,
    device_id: i32,
    seed_dev: &DeviceSeed,
) -> AnyResult<()> {
    let full_info = client.get_device_info(device_id, None).await?;
    let cloud_sensors = full_info.sensors.unwrap_or_default();

    let expected_tags: HashSet<&str> = seed_dev
        .sensors
        .iter()
        .map(|s| s.api_tag.as_str())
        .chain(seed_dev.actuators.iter().map(|a| a.api_tag.as_str()))
        .collect();

    // 1. Delete obsolete peripherals not present in seed JSON
    for cs in &cloud_sensors {
        let api_tag = cs.api_tag.as_str();
        if !expected_tags.contains(api_tag) {
            tracing::warn!(
                tag = %api_tag,
                device = %seed_dev.name,
                "Deleting obsolete peripheral from device"
            );
            client.delete_sensor(device_id, api_tag, None).await?;
            tracing::info!(tag = %api_tag, "Successfully deleted obsolete peripheral");
        }
    }

    // 2. Reconcile sensors defined in JSON
    for s in &seed_dev.sensors {
        let exists = cloud_sensors.iter().any(|cs| cs.api_tag == s.api_tag);

        if exists {
            tracing::info!(
                tag = %s.api_tag,
                device = %seed_dev.name,
                "Sensor already exists on device"
            );
        } else {
            tracing::info!(
                tag = %s.api_tag,
                name = %s.name,
                device = %seed_dev.name,
                "Adding sensor to device..."
            );
            let sensor_dto = SensorAddUpdate::builder()
                .name(&s.name)
                .api_tag(&s.api_tag)
                .trans_type(s.trans_type_kind())
                .data_type(s.data_type_kind())
                .maybe_type_attrs(s.type_attrs.clone())
                .maybe_unit(s.unit.clone())
                .precision(s.precision.unwrap_or(2))
                .build();

            client.add_sensor(device_id, &sensor_dto, None).await?;
            tracing::info!(
                tag = %s.api_tag,
                device = %seed_dev.name,
                "Successfully added sensor to device"
            );
        }
    }

    // 3. Reconcile actuators defined in JSON
    for a in &seed_dev.actuators {
        let exists = cloud_sensors.iter().any(|cs| cs.api_tag == a.api_tag);

        if exists {
            tracing::info!(
                tag = %a.api_tag,
                device = %seed_dev.name,
                "Actuator already exists on device"
            );
        } else {
            tracing::info!(
                tag = %a.api_tag,
                name = %a.name,
                device = %seed_dev.name,
                "Adding actuator to device..."
            );
            let actuator_dto = ActuatorAddUpdate::builder()
                .name(&a.name)
                .api_tag(&a.api_tag)
                .trans_type(a.trans_type_kind())
                .data_type(a.data_type_kind())
                .oper_type(a.oper_type_kind())
                .serial_number(a.serial_number.unwrap_or(0))
                .build();

            client.add_sensor(device_id, &actuator_dto, None).await?;
            tracing::info!(
                tag = %a.api_tag,
                device = %seed_dev.name,
                "Successfully added actuator to device"
            );
        }
    }

    Ok(())
}

/// Declarative, idempotent provisioner that treats the seed JSON file as the source of truth.
pub async fn provision(args: &ProjectArgs) -> AnyResult<()> {
    let project_name = args.resolved_name();
    let client = get_client(args).await?;

    // 1. Load declarative device definitions from JSON
    let devices_file = args.resolve_devices_file();
    let seed_devices = crate::seed::load_devices_from_file(&devices_file)?;
    tracing::info!(
        count = seed_devices.len(),
        path = %devices_file.display(),
        "Loaded declarative device definitions"
    );

    // 2. Discover or create target project
    tracing::info!("Searching for project '{project_name}' on NLECloud...");
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
            tracing::info!(
                project = %project_name,
                id = p.project_id,
                tag = ?p.project_tag,
                "Project already exists"
            );
            p.project_id
        }
        None => {
            tracing::info!("Project '{project_name}' not found. Provisioning new project...");
            let industry = IndustryKind::try_from(args.industry).unwrap_or(IndustryKind::SmartHome);
            let net_work_kind = NetworkKind::try_from(args.network_kind).unwrap_or(NetworkKind::Wifi);
            let dto = ProjectAddUpdateDto::builder()
                .name(&project_name)
                .industry(industry)
                .net_work_kind(net_work_kind)
                .build();
            let id = client.add_project(&dto, None).await?;
            tracing::info!(
                project = %project_name,
                id = id,
                "Successfully provisioned project"
            );
            id
        }
    };

    // Automatically apply required cloud prefixes (e.g. p{project_id}_dev1) to user-declared seed devices
    let seed_devices: Vec<crate::seed::DeviceSeed> = seed_devices
        .into_iter()
        .map(|mut d| {
            let (resolved_name, _) = resolve_device_name_and_tag(
                args.device_namespace.as_deref(),
                project_id,
                &d.name,
            );
            let (_, resolved_tag) = resolve_device_name_and_tag(
                args.device_namespace.as_deref(),
                project_id,
                &d.tag,
            );
            d.name = resolved_name;
            d.tag = resolved_tag;
            d
        })
        .collect();

    // 3. Query all devices currently attached to this project in the cloud
    let dev_query = DeviceQueryParams::builder()
        .project_key_word(project_id.to_string())
        .page_size(100)
        .build();
    let cloud_devices = client.get_devices(&dev_query, None).await?.page_set;

    // 4. Delete obsolete cloud devices not declared in the JSON seed
    let seed_tags: HashSet<&str> = seed_devices.iter().map(|d| d.tag.as_str()).collect();
    let seed_names: HashSet<&str> = seed_devices.iter().map(|d| d.name.as_str()).collect();

    for dev in &cloud_devices {
        let dev_tag = dev.tag.as_deref().unwrap_or("");
        let dev_name = dev.name.as_deref().unwrap_or("");
        if !seed_tags.contains(dev_tag) && !seed_names.contains(dev_name) {
            tracing::warn!(
                name = %dev_name,
                id = dev.device_id,
                tag = %dev_tag,
                "Deleting obsolete cloud device not present in JSON seed"
            );
            client.delete_device(dev.device_id, None).await?;
            tracing::info!(id = dev.device_id, "Successfully deleted obsolete device");
        }
    }

    // 5. Reconcile seed devices and their peripherals
    for seed_dev in &seed_devices {
        let matched = cloud_devices.iter().find(|d| {
            d.tag.as_deref() == Some(&seed_dev.tag) || d.name.as_deref() == Some(&seed_dev.name)
        });

        let device_id = match matched {
            Some(d) => {
                tracing::info!(
                    name = %seed_dev.name,
                    tag = %seed_dev.tag,
                    id = d.device_id,
                    "Device already exists"
                );
                d.device_id
            }
            None => {
                tracing::info!(
                    name = %seed_dev.name,
                    tag = %seed_dev.tag,
                    "Device not found. Provisioning new device..."
                );
                let dto = DeviceAddUpdateDto::builder()
                    .project_id_or_tag(project_id.to_string())
                    .name(&seed_dev.name)
                    .tag(&seed_dev.tag)
                    .protocol(seed_dev.protocol_kind())
                    .build();
                let id = client.add_device(&dto, None).await?;
                tracing::info!(
                    name = %seed_dev.name,
                    tag = %seed_dev.tag,
                    id = id,
                    "Successfully provisioned device"
                );
                id
            }
        };

        // Reconcile sensors and actuators for this device
        reconcile_device_peripherals(&client, device_id, seed_dev).await?;
    }

    tracing::info!("Declarative provisioning complete for project '{project_name}'.");
    Ok(())
}

/// Deletes the project by name.
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
        tracing::info!("Project '{project_name}' was not found. Nothing to delete.");
        return Ok(());
    }

    let ids: Vec<i32> = matching.iter().map(|p| p.project_id).collect();
    client.delete_projects(&ids, None).await?;

    tracing::info!(
        project = %project_name,
        ids = ?ids,
        "Successfully deleted project"
    );
    Ok(())
}
