
struct CoreSummary {
    name: String,
    serial: String,
    path: String,
    firmware: String,
    interface: String,
}

struct EnumerationError {
    path: String,
    error: DeviceError,
}

fn enumerate_devices() -> Result<Vec<Result<Box<dyn Device>, EnumerationError>>, DeviceError> {
    let drive_paths = list_physical_drives()?;
    let mut drives: Vec<_> = drive_paths
        .into_iter()
        .map(move |path| open_device(&path).map_err(|error| EnumerationError { path, error }))
        .collect();
    #[cfg(debug_assertions)]
    drives.push(Ok(Box::new(FakeDevice::new())));
    Ok(drives)
}

fn get_core_summary(device: &dyn Device) -> CoreSummary {
    CoreSummary {
        name: device.model_number().unwrap_or("Unknown".into()),
        serial: device.serial_number().unwrap_or("Unknown".into()).into(),
        path: device.path().unwrap_or("Not a file device".into()).into(),
        firmware: device.firmware_revision().unwrap_or("Unknown".into()).into(),
        interface: device.interface().unwrap_or(Interface::Other).to_string(),
    }
}

struct DeviceList {
    pub active: Vec<Box<dyn Device>>,
    pub failed: Vec<EnumerationError>,
    pub enumeration_error: Option<DeviceError>,
}

async fn update_device_list() -> impl FnOnce(&mut App, AppWindow) {
    let (tx, rx) = oneshot::channel();
    std::thread::spawn(|| {
        let maybe_devices = enumerate_devices();
        let _ = tx.send(maybe_devices);
    });
    let maybe_devices = rx.await.expect("the thread should not be terminated before we recv the value");
    let device_list = match maybe_devices {
        Ok(results) => {
            let mut device_list = DeviceList { active: vec![], failed: vec![], enumeration_error: None };
            for result in results {
                match result {
                    Ok(device) => device_list.active.push(device),
                    Err(error) => device_list.failed.push(error),
                }
            }
            device_list
        }
        Err(err) => DeviceList { active: vec![], failed: vec![], enumeration_error: Some(err) },
    };

    move |app, _| app.device_list = Arc::new(device_list)
}
