use tonic::{transport::Server, Request, Response, Status};
use skopos::ormos_server::{Ormos, OrmosServer};
use skopos::{ListUsbDevicesRequest, ListUsbDevicesResponse, UsbDevice, MountUsbDeviceRequest, MountUsbDeviceResponse, UnmountUsbDeviceRequest, UnmountUsbDeviceResponse, ListImageArchivesRequest, ListImageArchivesResponse, ImageArchive, LoadImageArchiveRequest, LoadImageArchiveResponse, InspectImageArchiveRequest, InspectImageArchiveResponse};
use std::io;
use std::fs;
use std::process::Command;
use std::path::Path;
use sha2::{Sha256, Digest};

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

fn mount_usb_device(device_path: &str, mount_point: &str) -> Result<(), io::Error> {
    if let Some(parent) = std::path::Path::new(mount_point).parent() {
        fs::create_dir_all(parent)?;
    }
    fs::create_dir_all(mount_point)?;
    
    let output = Command::new("mount")
        .arg(device_path)
        .arg(mount_point)
        .output()?;

    if output.status.success() {
        println!("Successfully mounted {} to {}", device_path, mount_point);
        Ok(())
    } else {
        let error_msg = String::from_utf8_lossy(&output.stderr);
        eprintln!("Mount failed: {}", error_msg);
        Err(io::Error::new(io::ErrorKind::Other, error_msg.to_string()))
    }
}

fn unmount_usb_device(mount_point: &str) -> Result<(), io::Error> {
    let output = Command::new("umount")
        .arg(mount_point)
        .output()?;

    if output.status.success() {
        println!("Successfully unmounted {}", mount_point);
        Ok(())
    }
    else {
        let error_msg = String::from_utf8_lossy(&output.stderr);
        eprintln!("Unmount failed: {}", error_msg);
        Err(io::Error::new(io::ErrorKind::Other, error_msg.to_string()))
    }
}

fn is_file_a_container_image_archive(path: &Path) -> bool {
    let output = Command::new("skopeo")
        .arg("inspect")
        .arg(format!("docker-archive:{}", path.display()))
        .output();
    
    match output {
        Ok(result) => result.status.success(),
        Err(_) => false,
    }
}

fn list_container_image_archives(dir_path: &str) -> std::io::Result<Vec<String>> {
    let mut images = Vec::new();
    
    for entry in std::fs::read_dir(dir_path)? {
        let entry = entry?;
        let path = entry.path();
        
        if path.extension().map_or(false, |ext| ext == "tar") {
            if is_file_a_container_image_archive(&path) {
                images.push(path.display().to_string());
            }
        }
    }
    
    Ok(images)
}

fn inspect_container_image_archive(path: &Path) -> std::io::Result<String> {
    let output = Command::new("skopeo")
        .arg("inspect")
        .arg(format!("docker-archive:{}", path.display()))
        .output()?;

    if output.status.success() {
        let inspect_msg = String::from_utf8_lossy(&output.stdout);
        Ok(inspect_msg.to_string())
    } else {
        let error_msg = String::from_utf8_lossy(&output.stderr);
        eprintln!("Insepct failed: {}", error_msg);
        Err(io::Error::new(io::ErrorKind::Other, error_msg.to_string()))
    }
}

fn create_container_image_archives(archive_paths: Vec<String>) -> Result<Vec<ImageArchive>, Box<dyn std::error::Error>> {
    archive_paths
        .into_iter()
        .map(|archive_path| {
            let metadata = fs::metadata(&archive_path)?;
            let archive_size_bytes = metadata.len() as i64;
            
            let mut file = fs::File::open(&archive_path)?;
            let mut hasher = Sha256::new();
            io::copy(&mut file, &mut hasher)?;
            let hash_bytes = hasher.finalize();
            let hash_str = hex::encode(hash_bytes);
            
            Ok(ImageArchive {
                file_path: archive_path,
                file_size_bytes: archive_size_bytes,
                sha256_checksum: hash_str,
            })
        })
        .collect()
}

fn load_container_image_archive(path: &Path, image_name: &str, image_tag: &str) -> Result<(), io::Error> {
    let output = Command::new("skopeo")
        .arg("copy")
        .arg(format!("docker-archive:{}", path.display()))
        .arg(format!("docker://localhost:5000/{}:{}", image_name, image_tag))
        .output()?;
    
    if output.status.success() {
        println!("Successfully loaded {}", path.display());
        Ok(())
    }
    else {
        let error_msg = String::from_utf8_lossy(&output.stderr);
        eprintln!("Loading image failed: {}", error_msg);
        Err(io::Error::new(io::ErrorKind::Other, error_msg.to_string()))
    }
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
    
    async fn mount_usb_device(
        &self,
        request: Request<MountUsbDeviceRequest>,
    ) -> Result<Response<MountUsbDeviceResponse>, Status> {
        let req = request.into_inner();
        let device_path = req.device_path;
        let mount_point = if req.mount_point.is_empty() {
            "/mnt/usb".to_string()
        } else {
            req.mount_point
        };
        
        match mount_usb_device(&device_path, &mount_point) {
            Ok(()) => {
                let response = MountUsbDeviceResponse {
                    is_success: true,
                    error_message: String::new(),
                };
                Ok(Response::new(response))
            }
            Err(e) => {
                let response = MountUsbDeviceResponse {
                    is_success: false,
                    error_message: e.to_string(),
                };
                Ok(Response::new(response))
            }
        }
    }

    async fn unmount_usb_device(
        &self,
        request: Request<UnmountUsbDeviceRequest>,
    ) -> Result<Response<UnmountUsbDeviceResponse>, Status> {
        let req = request.into_inner();
        let mount_point = if req.mount_point.is_empty() {
            "/mnt/usb".to_string()
        } else {
            req.mount_point
        };

        match unmount_usb_device(&mount_point) {
            Ok(()) => {
                let response = UnmountUsbDeviceResponse {
                    is_success: true,
                    error_message: String::new(),
                };
                Ok(Response::new(response))
            }
            Err(e) => {
                let response = UnmountUsbDeviceResponse {
                    is_success: false,
                    error_message: e.to_string(),
                };
                Ok(Response::new(response))
            }
        }
    }

    async fn list_image_archives(
        &self,
        request: Request<ListImageArchivesRequest>,
    ) -> Result<Response<ListImageArchivesResponse>, Status> {
        let req = request.into_inner();
        let path = if req.path.is_empty() {
            "/mnt/usb".to_string()
        } else {
            req.path
        };
        let container_image_archive_paths = list_container_image_archives(&path)?;
         let image_archives = create_container_image_archives(container_image_archive_paths)
            .map_err(|e| Status::internal(format!("Failed to create image archive response: {}", e)))?;
        let response = ListImageArchivesResponse {
            image_archives,
        };
        Ok(Response::new(response))
    }

    async fn load_image_archive(
        &self,
        request: Request<LoadImageArchiveRequest>,
    ) -> Result<Response<LoadImageArchiveResponse>, Status> {
        let req = request.into_inner();
        let file_path = req.file_path;
        
        match load_container_image_archive(Path::new(&file_path), &req.image_name, &req.image_tag) {
            Ok(()) => {
                let response = LoadImageArchiveResponse {
                    is_success: true,
                    error_message: String::new(),
                };
                Ok(Response::new(response))
            }
            Err(e) => {
                let response = LoadImageArchiveResponse {
                    is_success: false,
                    error_message: e.to_string(),
                };
                Ok(Response::new(response))
            }
        }
    }

    async fn inspect_image_archive(
        &self,
        request: Request<InspectImageArchiveRequest>,
    ) -> Result<Response<InspectImageArchiveResponse>, Status> {
        let req = request.into_inner();
        let file_path = req.file_path;

        match inspect_container_image_archive(Path::new(&file_path)) {
            Ok(output) => {
                let response = InspectImageArchiveResponse {
                    is_success: true,
                    stdout: output,
                    stderr: String::new(),
                };
                Ok(Response::new(response))
            }
            Err(e) => {
                let response = InspectImageArchiveResponse {
                    is_success: false,
                    stdout: String::new(),
                    stderr: e.to_string(),
                };
                Ok(Response::new(response))
            }
        }
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
