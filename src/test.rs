use std::collections::HashMap;
use std::fs;
use std::path::Path;

const SYS_BLOCK_PATH: &str = "/sys/block";
const DEV_PATH_PREFIX: &str = "/dev";
const DISK_BY_UUID_PATH: &str = "/dev/disk/by-uuid";
const DISK_BY_LABEL_PATH: &str = "/dev/disk/by-label";

#[derive(Debug)]
pub struct BlockDevice {
    pub name: String,
    pub path: String,
    pub uuid: Option<String>,
    pub label: Option<String>,
    pub mountpoint: Option<String>,
}

pub fn list_block_devices() -> Vec<BlockDevice> {
    let mut devices = Vec::new();
    let mounts = read_mounts();

    let Ok(entries) = fs::read_dir(SYS_BLOCK_PATH) else {
        return devices;
    };

    for entry in entries.flatten() {
        let device_name = entry.file_name().to_string_lossy().to_string();
        let sysfs_device_path = entry.path();
        collect_partitions(&sysfs_device_path, &device_name, &mounts, &mut devices);
    }

    devices
}

fn collect_partitions(
    dev_path: &Path,
    dev_name: &str,
    mounts: &HashMap<String, String>,
    devices: &mut Vec<BlockDevice>,
) {
    if let Some(device) = build_device(dev_name, mounts) {
        devices.push(device);
    }

    let Ok(entries) = fs::read_dir(dev_path) else {
        return;
    };

    for entry in entries.flatten() {
        let partition_name = entry.file_name().to_string_lossy().to_string();
        if partition_name.starts_with(dev_name) {
            if let Some(device) = build_device(&partition_name, mounts) {
                devices.push(device);
            }
        }
    }
}

fn build_device(name: &str, mounts: &HashMap<String, String>) -> Option<BlockDevice> {
    let path = device_path(name);
    let uuid = find_in_disk_by(DISK_BY_UUID_PATH, &path);
    let label = find_in_disk_by(DISK_BY_LABEL_PATH, &path);

    if uuid.is_none() && label.is_none() {
        return None;
    }

    let mountpoint = mounts.get(&path).cloned();

    Some(BlockDevice {
        name: name.to_string(),
        path,
        uuid,
        label,
        mountpoint,
    })
}

fn device_path(name: &str) -> String {
    format!("{}/{}", DEV_PATH_PREFIX, name)
}

/// Sucht in einem /dev/disk/by-* Verzeichnis nach dem Symlink, der auf `dev_path` zeigt
fn find_in_disk_by(directory: &str, dev_path: &str) -> Option<String> {
    let entries = fs::read_dir(directory).ok()?;

    for entry in entries.flatten() {
        if let Ok(target) = fs::canonicalize(entry.path()) {
            if target.to_string_lossy() == dev_path {
                return Some(entry.file_name().to_string_lossy().to_string());
            }
        }
    }

    None
}

/// Liest /proc/mounts → HashMap<device, mountpoint>
fn read_mounts() -> HashMap<String, String> {
    let Ok(content) = fs::read_to_string("/proc/mounts") else {
        return HashMap::new();
    };

    content
        .lines()
        .filter_map(|line| {
            let mut parts = line.split_whitespace();
            let dev = parts.next()?.to_string();
            let mountpoint = parts.next()?.to_string();
            Some((dev, mountpoint))
        })
        .collect()
}