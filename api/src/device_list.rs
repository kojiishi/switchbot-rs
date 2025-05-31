use super::Device;
use std::ops::{Deref, DerefMut};

#[derive(Debug, Default, serde::Deserialize)]
#[serde(transparent)] // This allows DeviceList to be deserialized as if it were just Vec<Device>
pub struct DeviceList {
    devices: Vec<Device>,
}

impl DeviceList {
    pub fn new() -> Self {
        Self {
            devices: Vec::new(),
        }
    }

    #[allow(dead_code)]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            devices: Vec::with_capacity(capacity),
        }
    }

    pub fn push(&mut self, device: Device) {
        self.devices.push(device);
    }

    pub fn extend<T: IntoIterator<Item = Device>>(&mut self, iter: T) {
        self.devices.extend(iter);
    }

    pub fn index_by_device_id(&self, device_id: &str) -> Option<usize> {
        self.devices.iter().position(|d| d.device_id() == device_id)
    }

    // Delegate common Vec methods
    pub fn iter(&self) -> std::slice::Iter<'_, Device> {
        self.devices.iter()
    }

    pub fn iter_mut(&mut self) -> std::slice::IterMut<'_, Device> {
        self.devices.iter_mut()
    }

    pub fn get(&self, index: usize) -> Option<&Device> {
        self.devices.get(index)
    }

    pub fn get_mut(&mut self, index: usize) -> Option<&mut Device> {
        self.devices.get_mut(index)
    }

    pub fn is_empty(&self) -> bool {
        self.devices.is_empty()
    }

    pub fn len(&self) -> usize {
        self.devices.len()
    }
}

impl Deref for DeviceList {
    type Target = Vec<Device>;

    fn deref(&self) -> &Self::Target {
        &self.devices
    }
}

impl DerefMut for DeviceList {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.devices
    }
}

impl IntoIterator for DeviceList {
    type Item = Device;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.devices.into_iter()
    }
}
