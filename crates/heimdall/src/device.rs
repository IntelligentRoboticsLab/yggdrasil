use crate::Result;
use linuxvideo::Device;

/// Lists camera device capabilities
///
/// # Arguments
/// * `device` -> a `linuxvideo::Device`
///
/// # Examples
/// ```no_run
/// use linuxvideo::Device;
/// use heimdall::device::{
///     list_capabilities,
/// };
///
/// let path = std::path::Path::new("/dev/video0");
/// let device = Device::open(path).unwrap();
/// list_capabilities(device);
/// ```
pub fn list_capabilities(device: Device) -> Result<()> {
    let capabilities = device.capabilities()?;
    println!("- {}: {}", device.path()?.display(), capabilities.card());
    println!("  driver: {}", capabilities.driver());
    println!("  bus info: {}", capabilities.bus_info());
    println!(
        "  all capabilities:    {:?}",
        capabilities.all_capabilities()
    );
    println!(
        "  avail. capabilities: {:?}",
        capabilities.device_capabilities()
    );

    Ok(())
}

/// Prints all video devices and their capabilities
///
/// # Examples
/// ```no_run
/// use linuxvideo::Device;
/// use heimdall::device::print_device_list;
///
/// print_device_list();
/// ```
pub fn print_device_list() -> Result<()> {
    for device in linuxvideo::list()? {
        match device {
            Ok(device) => list_capabilities(device)?,
            Err(e) => {
                eprintln!("Skipping device due to error: {e:?}");
            }
        }
    }

    Ok(())
}
