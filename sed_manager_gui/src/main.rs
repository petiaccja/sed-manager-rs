#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{error::Error, rc::Rc};

use sed_manager::device::{list_physical_drives, open_device, Device as StorageDevice};
use sed_manager::fake_device::FakeDevice;
use sed_manager::messaging::discovery::FeatureDescriptor;
use sed_manager::tper::discover;

slint::include_modules!();

fn get_interface(device: &dyn StorageDevice) -> Interface {
    match device.interface() {
        sed_manager::device::Interface::ATA => Interface::Ata,
        sed_manager::device::Interface::SATA => Interface::Sata,
        sed_manager::device::Interface::SCSI => Interface::Scsi,
        sed_manager::device::Interface::NVMe => Interface::Nvme,
        sed_manager::device::Interface::SD => Interface::Sd,
        sed_manager::device::Interface::MMC => Interface::Mmc,
        sed_manager::device::Interface::Other => Interface::Unknown,
    }
}

fn get_ssc(device: &dyn StorageDevice) -> SSC {
    let Ok(discovery) = discover(&*device) else {
        return SSC::Unsupported;
    };
    for descriptor in discovery.descriptors.iter() {
        match descriptor {
            FeatureDescriptor::TPer(_) => (),
            FeatureDescriptor::Locking(_) => (),
            FeatureDescriptor::Geometry(_) => (),
            FeatureDescriptor::DataRemoval(_) => (),
            FeatureDescriptor::OpalV2(_) => return SSC::OpalV2,
            FeatureDescriptor::Unrecognized => (),
            FeatureDescriptor::Enterprise(_) => return SSC::Enterprise,
            FeatureDescriptor::OpalV1(_) => return SSC::OpalV1,
            FeatureDescriptor::Opalite(_) => return SSC::Opalite,
            FeatureDescriptor::PyriteV1(_) => return SSC::PyriteV1,
            FeatureDescriptor::PyriteV2(_) => return SSC::PyriteV2,
            FeatureDescriptor::Ruby(_) => return SSC::Ruby,
            FeatureDescriptor::KeyPerIO(_) => return SSC::KeyPerIo,
        }
    }
    return SSC::Unsupported;
}

fn get_info(device: &dyn StorageDevice, path: String) -> DeviceModel {
    DeviceModel {
        name: device.model_number().unwrap_or("Unknown".into()).into(),
        firmware: device.firmware_revision().unwrap_or("Unknown".into()).into(),
        interface: get_interface(&*device),
        serial: device.serial_number().unwrap_or("Unknown".into()).into(),
        ssc: get_ssc(&*device),
        path: path.into(),
    }
}

fn get_devices() -> Vec<DeviceModel> {
    let Ok(paths) = list_physical_drives() else {
        return Vec::new();
    };
    let mut devices = Vec::new();
    for path in paths {
        if let Ok(device) = open_device(&path) {
            devices.push(get_info(&*device, path));
        }
    }
    devices.push(get_info(&FakeDevice::new(), "/dev/fake_device".into()));
    devices
}

fn refresh_devices(main_window: &MainWindow) {
    let main_window = main_window.as_weak();
    std::thread::spawn(move || {
        let devices = get_devices();
        slint::invoke_from_event_loop(move || {
            let devices = Rc::new(slint::VecModel::from(devices));
            if let Some(main_window) = main_window.upgrade() {
                main_window.set_devices(devices.into());
            }
        })
    });
}

fn main() -> Result<(), Box<dyn Error>> {
    let _ = slint::BackendSelector::new().backend_name("winit".into()).renderer_name("software".into()).select();

    let main_window = MainWindow::new()?;

    refresh_devices(&main_window);
    main_window.run()?;

    Ok(())
}
