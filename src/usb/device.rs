use nusb::MaybeFuture;
use nusb::transfer::{Bulk, Out};
use serde::Serialize;
use std::fmt;
use std::io::Write;
use std::time::Duration;

const DYMO_VENDOR_ID: u16 = 0x0922;
const USB_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Debug)]
pub enum UsbError {
    DeviceNotFound,
    EndpointNotFound,
    Nusb(nusb::Error),
    Transfer(nusb::transfer::TransferError),
    Io(std::io::Error),
}

impl fmt::Display for UsbError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DeviceNotFound => write!(f, "Dymo device not found"),
            Self::EndpointNotFound => write!(f, "Bulk endpoint not found on device"),
            Self::Nusb(e) => write!(f, "USB error: {e}"),
            Self::Transfer(e) => write!(f, "Transfer error: {e}"),
            Self::Io(e) => write!(f, "IO error: {e}"),
        }
    }
}

impl From<nusb::Error> for UsbError {
    fn from(e: nusb::Error) -> Self {
        Self::Nusb(e)
    }
}

impl From<nusb::transfer::TransferError> for UsbError {
    fn from(e: nusb::transfer::TransferError) -> Self {
        Self::Transfer(e)
    }
}

impl From<std::io::Error> for UsbError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct DetectedDevice {
    pub name: String,
    pub vendor_id: u16,
    pub product_id: u16,
    pub needs_modeswitch: bool,
}

pub fn enumerate_devices() -> Vec<DetectedDevice> {
    let mut found = Vec::new();

    let devices = match nusb::list_devices().wait() {
        Ok(d) => d,
        Err(_) => return found,
    };

    for info in devices {
        if info.vendor_id() != DYMO_VENDOR_ID {
            continue;
        }

        let (name, needs_modeswitch) = match info.product_id() {
            0x1001 => ("LabelManager PnP (storage mode)", true),
            0x1002 => ("LabelManager PnP", false),
            0x1003 => ("LabelManager 420P (storage mode)", true),
            0x1004 => ("LabelManager 420P", false),
            0x1005 => ("LabelManager 280 (storage mode)", true),
            0x1006 => ("LabelManager 280", false),
            0x1007 => ("LabelManager Wireless PnP (storage mode)", true),
            0x1008 => ("LabelManager Wireless PnP", false),
            0x0011 => ("LabelMANAGER PC", false),
            0x0015 => ("LabelPoint 350", false),
            _ => continue,
        };

        found.push(DetectedDevice {
            name: name.to_string(),
            vendor_id: info.vendor_id(),
            product_id: info.product_id(),
            needs_modeswitch,
        });
    }

    found
}

pub fn is_device_connected(product_id: u16) -> bool {
    nusb::list_devices()
        .wait()
        .map(|devices| {
            devices.into_iter().any(|d| {
                d.vendor_id() == DYMO_VENDOR_ID && d.product_id() == product_id
            })
        })
        .unwrap_or(false)
}

pub fn modeswitch(product_id_storage: u16, payload: &[u8]) -> Result<(), UsbError> {
    let device_info = nusb::list_devices()
        .wait()?
        .find(|d| d.vendor_id() == DYMO_VENDOR_ID && d.product_id() == product_id_storage)
        .ok_or(UsbError::DeviceNotFound)?;

    let device = device_info.open().wait()?;
    let interface = device.detach_and_claim_interface(0).wait()?;

    interface
        .control_out(
            nusb::transfer::ControlOut {
                control_type: nusb::transfer::ControlType::Class,
                recipient: nusb::transfer::Recipient::Interface,
                request: 0,
                value: 0,
                index: 0,
                data: payload,
            },
            USB_TIMEOUT,
        )
        .wait()?;

    Ok(())
}

pub fn send_print_data(product_id: u16, data: &[u8]) -> Result<(), UsbError> {
    let device_info = nusb::list_devices()
        .wait()?
        .find(|d| d.vendor_id() == DYMO_VENDOR_ID && d.product_id() == product_id)
        .ok_or(UsbError::DeviceNotFound)?;

    let device = device_info.open().wait()?;
    let interface = device.detach_and_claim_interface(0).wait()?;

    let ep_desc = interface
        .descriptor()
        .ok_or(UsbError::EndpointNotFound)?
        .endpoints()
        .find(|ep| ep.address() & 0x80 == 0)
        .ok_or(UsbError::EndpointNotFound)?;

    let ep_addr = ep_desc.address();
    let ep: nusb::Endpoint<Bulk, Out> =
        interface.endpoint(ep_addr)?;

    let mut writer = ep.writer(4096);
    writer.write_all(data)?;
    writer.flush()?;

    Ok(())
}
