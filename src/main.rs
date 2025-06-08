use tonic::{transport::Server, Request, Response, Status};
use skopos::ormos_server::{Ormos, OrmosServer};
use skopos::{ListUsbDevicesRequest, ListUsbDevicesResponse, UsbDevice};
use std::io;
use std::fs;

pub mod skopos {
    tonic::include_proto!("unit.containers.v0");

    pub(crate) const FILE_DESCRIPTOR_SET: &[u8] =
        tonic::include_file_descriptor_set!("skopos_descriptor");
}

#[derive(Default)]
pub struct MyOrmos {}

fn list_usb_block_devices() -> io::Result<Vec<String>> {
    let mut usb_devices = Vec::new();
    let dev_path = "/dev";
    let entries = fs::read_dir(dev_path)?;
    
    for entry in entries {
        let entry = entry?;
        let device_name = entry.file_name();
        let device_str = device_name.to_string_lossy();
        
        if device_str.starts_with("sd") && device_str.len() == 3 {
            if is_usb_storage_device(&device_str) {
                let device_path = format!("/dev/{}", device_str);
                usb_devices.push(device_path);
                
                let partitions = get_partitions(&device_str)?;
                usb_devices.extend(partitions);
            }
        }
    }
    
    Ok(usb_devices)
}

fn is_usb_storage_device(device_name: &str) -> bool {
    let sys_path = format!("/sys/block/{}/removable", device_name);
    
    if let Ok(content) = fs::read_to_string(&sys_path) {
        return content.trim() == "1";
    }
    
    false
}

fn get_partitions(device_name: &str) -> io::Result<Vec<String>> {
    let mut partitions = Vec::new();
    let dev_path = "/dev";
    let entries = fs::read_dir(dev_path)?;
    
    for entry in entries {
        let entry = entry?;
        let partition_name = entry.file_name();
        let partition_str = partition_name.to_string_lossy();
        
        if partition_str.starts_with(device_name) && partition_str.len() > device_name.len() {
            let partition_path = format!("/dev/{}", partition_str);
            partitions.push(partition_path);
        }
    }
    
    Ok(partitions)
}

fn get_mount_info(device_path: &str) -> (bool, String) {
    if let Ok(mounts) = fs::read_to_string("/proc/mounts") {
        for line in mounts.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 && parts[0] == device_path {
                return (true, parts[1].to_string());
            }
        }
    }
    (false, String::new())
}

fn create_usb_devices(device_paths: Vec<String>) -> Vec<UsbDevice> {
    device_paths
        .into_iter()
        .map(|device_path| {
            let (is_mounted, mount_point) = get_mount_info(&device_path);
            
            UsbDevice {
                device_path,
                is_mounted,
                mount_point,
            }
        })
        .collect()
}

#[tonic::async_trait]
impl Ormos for MyOrmos {
    async fn list_usb_devices(
        &self,
        _request: Request<ListUsbDevicesRequest>,
    ) -> Result<Response<ListUsbDevicesResponse>, Status> {
        let usb_device_paths = list_usb_block_devices()
            .map_err(|e| Status::internal(format!("Failed to list USB devices: {}", e)))?;
        
        let devices = create_usb_devices(usb_device_paths);
        
        let response = ListUsbDevicesResponse {
            devices,
        };
        
        Ok(Response::new(response))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "0.0.0.0:50052".parse().unwrap();
    let ormos = MyOrmos::default();
    
    let reflection = tonic_reflection::server::Builder::configure()
        .register_encoded_file_descriptor_set(skopos::FILE_DESCRIPTOR_SET)
        .build_v1()?;

    println!("Listening on {}", addr);

    Server::builder()
        .add_service(reflection)
        .add_service(OrmosServer::new(ormos))
        .serve(addr)
        .await?;

    Ok(())
}
