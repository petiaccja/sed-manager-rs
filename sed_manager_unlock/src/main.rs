use std::io;
use std::sync::Arc;

use sed_manager::device::{list_physical_drives, open_device, Device, Error as DeviceError};
use sed_manager::messaging::discovery::LockingDescriptor;
use sed_manager::rpc::discover;

struct DeviceList {
    shadowed: Vec<Arc<dyn Device>>,
    locked: Vec<Arc<dyn Device>>,
    other: Vec<Box<dyn Device>>,
    failed: Vec<(String, DeviceError)>,
}

fn get_device_list() -> Result<DeviceList, DeviceError> {
    let mut device_list = DeviceList::new();
    let paths = list_physical_drives()?;
    for path in paths {
        let device = match open_device(&path) {
            Ok(device) => device,
            Err(error) => {
                device_list.failed.push((path, error));
                continue;
            }
        };
        let Ok(discovery) = discover(&*device) else {
            device_list.other.push(device);
            continue;
        };
        let Some(locking_desc) = discovery.get::<LockingDescriptor>() else {
            device_list.other.push(device);
            continue;
        };
        if !locking_desc.mbr_done && locking_desc.mbr_enabled {
            device_list.shadowed.push(Arc::from(device));
        } else if locking_desc.locked {
            device_list.locked.push(Arc::from(device));
        } else {
            device_list.other.push(device);
        }
    }
    Ok(device_list)
}

fn print_devices(device_list: &DeviceList) -> Result<(), DeviceError> {
    println!("Shadowed devices:");
    for device in &device_list.shadowed {
        println!("  {} / {}", device.model_number(), device.serial_number());
    }

    println!("\nLocked devices:");
    for device in &device_list.locked {
        println!("  {} / {}", device.model_number(), device.serial_number());
    }

    println!("\nOther devices:");
    for device in &device_list.other {
        println!("  {} / {}", device.model_number(), device.serial_number());
    }

    println!("\nFailed devices:");
    for device in &device_list.failed {
        println!("  {} / {}", device.0, device.1);
    }

    Ok(())
}

fn main() -> io::Result<()> {
    println!("{BANNER}{VERSION}\n\n");

    let device_list = match get_device_list() {
        Ok(device_list) => device_list,
        Err(error) => {
            eprintln!("Failed to list devices:\n  {error}");
            return Ok(());
        }
    };

    let _ = print_devices(&device_list);

    println!("\nPress enter to exit...");
    let stdin = io::stdin();
    let mut buffer = String::new();
    stdin.read_line(&mut buffer)?;

    Ok(())
}

impl DeviceList {
    pub fn new() -> Self {
        Self { shadowed: vec![], locked: vec![], other: vec![], failed: vec![] }
    }
}

const BANNER: &str = r"
     ____  _____ ____  __  __                                   
    / ___|| ____|  _ \|  \/  | __ _ _ __   __ _  __ _  ___ _ __ 
    \___ \|  _| | | | | |\/| |/ _` | '_ \ / _` |/ _` |/ _ \ '__|
     ___) | |___| |_| | |  | | (_| | | | | (_| | (_| |  __/ |   
    |____/|_____|____/|_|  |_|\__,_|_| |_|\__,_|\__, |\___|_|   
                                                |___/           
                              v";

const VERSION: &str = env!("CARGO_PKG_VERSION");
