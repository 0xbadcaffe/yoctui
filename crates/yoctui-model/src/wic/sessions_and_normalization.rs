#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WicSessionId(pub u64);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WicOperation {
    Create(WicCreateRequest),
    Write(WicWriteRequest),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WicSession {
    pub id: WicSessionId,
    pub background_job_id: crate::BackgroundJobId,
    pub operation: WicOperation,
    pub exit_code: Option<i32>,
    pub error_detail: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WicOutputStream {
    Stdout,
    Stderr,
}

pub fn normalize_wic_outputs(
    output_directory: &Path,
    mut outputs: Vec<WicOutput>,
) -> Result<Vec<WicOutput>, &'static str> {
    if !absolute_normal_path(output_directory) {
        return Err("Wic output inventory roots must be normalized and absolute");
    }
    outputs.retain(|output| {
        output.identity.validate().is_ok() && output.identity.path.starts_with(output_directory)
    });
    outputs.sort_by(|left, right| left.identity.path.cmp(&right.identity.path));
    outputs.dedup_by(|left, right| left.identity == right.identity);
    outputs.truncate(MAX_WIC_OUTPUTS);
    Ok(outputs)
}

pub fn normalize_wic_devices(mut devices: Vec<WicDevice>) -> Vec<WicDevice> {
    devices.retain(|device| device.identity.validate().is_ok());
    for device in &mut devices {
        device
            .descendant_mounts
            .retain(|mount| absolute_normal_path(mount));
        device.descendant_mounts.sort();
        device.descendant_mounts.dedup();
        device.descendant_mounts.truncate(MAX_WIC_DEVICE_MOUNTS);
        device.unavailable_reason = device
            .unavailable_reason
            .take()
            .filter(|reason| !reason.is_empty() && !reason.chars().any(char::is_control))
            .map(|reason| reason.chars().take(2_048).collect());
    }
    devices.sort_by(|left, right| left.identity.path.cmp(&right.identity.path));
    devices.dedup_by(|left, right| left.identity == right.identity);
    devices.truncate(MAX_WIC_DEVICES);
    devices
}

pub fn normalize_wic_limitations(limitations: Vec<String>) -> Vec<String> {
    normalize_messages(limitations)
}

fn normalize_messages(messages: Vec<String>) -> Vec<String> {
    let mut messages: Vec<_> = messages
        .into_iter()
        .filter(|message| !message.is_empty() && !message.chars().any(char::is_control))
        .map(|message| message.chars().take(2_048).collect())
        .collect();
    messages.sort();
    messages.dedup();
    messages.truncate(MAX_WIC_LIMITATIONS);
    messages
}

fn safe_name(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 256
        && value.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.' | '+')
        })
}

pub(crate) fn absolute_normal_path(path: &Path) -> bool {
    yoctui_utils::is_absolute_normal_path(path)
}

