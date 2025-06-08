# skopos
A gRPC server that facilitates provisioning and updating containers.

## Usage
### List Services
Replace IP address accordingly.
```bash
grpcurl -plaintext 10.0.0.42:50052 list
```

Example output:
```bash
grpc.reflection.v1.ServerReflection
unit.containers.v0.Ormos
```

### List Ormos Service Methods
Replace IP address accordingly.
```bash
grpcurl -plaintext 10.0.0.42:50052 list unit.containers.v0.Ormos
```

Example output:
```bash
unit.containers.v0.Ormos.InspectImageArchive
unit.containers.v0.Ormos.ListImageArchives
unit.containers.v0.Ormos.ListUsbDevices
unit.containers.v0.Ormos.LoadImageArchive
unit.containers.v0.Ormos.MountUsbDevice
unit.containers.v0.Ormos.UnmountUsbDevice
```

### List USB Devices
Replace IP address accordingly.
```bash
grpcurl -plaintext -import-path ./proto -proto ormos.proto -d '{}' '10.0.0.42:50052' unit.containers.v0.Ormos.ListUsbDevices
```

Example output (device not mounted):
```json
{
  "devices": [
    {
      "devicePath": "/dev/sda"
    },
    {
      "devicePath": "/dev/sda1"
    }
  ]
}
```

Example output (device mounted):
```json
{
  "devices": [
    {
      "devicePath": "/dev/sda"
    },
    {
      "devicePath": "/dev/sda1",
      "isMounted": true,
      "mountPoint": "/mnt/usb"
    }
  ]
}
```

### Mount USB Device
Replace IP address accordingly.
```bash
grpcurl -plaintext -import-path ./proto -proto ormos.proto -d '{"device_path": "/dev/sda1", "mount_point": "/mnt/usb"}' '10.0.0.42:50052' unit.containers.v0.Ormos.MountUsbDevice
```

### List Container Image Archives
Replace IP address accordingly.
```bash
grpcurl -plaintext -import-path ./proto -proto ormos.proto -d '{"path": "/mnt/usb"}' '10.0.0.42:50052' unit.containers.v0.Ormos.ListImageArchives
```

Example output:
```json
{
  "imageArchives": [
    {
      "filePath": "/mnt/usb/alpine-arm64.tar",
      "fileSizeBytes": "8468992",
      "sha256Checksum": "0b070f92dd64e2bfead9f0c494511c9ce486f18900ec8297f374e5000a5f2994"
    }
  ]
}

```

### Inspect Container Image Archive
Replace IP address and archive path accordingly, pretty printing underlying skopeo commands stdout if request successful.
```bash
grpcurl -plaintext -import-path ./proto -proto ormos.proto -d '{"file_path": "/mnt/usb/alpine-arm64.tar"}' '10.0.0.42:50052' unit.containers.v0.Ormos.InspectImageArchive | jq 'if .isSuccess then .stdout | fromjson else . end'
```

Example output (success):
```json
{
  "Digest": "sha256:058c92d86112aa6f641b01ed238a07a3885b8c0815de3e423e5c5f789c398b45",
  "RepoTags": [],
  "Created": "2025-02-14T03:28:36Z",
  "DockerVersion": "",
  "Labels": null,
  "Architecture": "arm64",
  "Os": "linux",
  "Layers": [
    "sha256:a16e98724c05975ee8c40d8fe389c3481373d34ab20a1cf52ea2accc43f71f4c"
  ],
  "LayersData": [
    {
      "MIMEType": "application/vnd.docker.image.rootfs.diff.tar.gzip",
      "Digest": "sha256:a16e98724c05975ee8c40d8fe389c3481373d34ab20a1cf52ea2accc43f71f4c",
      "Size": 8461312,
      "Annotations": null
    }
  ],
  "Env": [
    "PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin"
  ]
}
```

Example output (failed):
```json
{
  "stderr": "time=\"2025-06-08T16:15:16Z\" level=fatal msg=\"Error parsing image name \\\"docker-archive:/mnt/usb/dne.tar\\\": opening file \\\"/mnt/usb/dne.tar\\\": open /mnt/usb/dne.tar: no such file or directory\"\n"
}
```

### Unmount USB Device
Replace IP address accordingly.
```bash
grpcurl -plaintext -import-path ./proto -proto ormos.proto -d '{"mount_point": "/mnt/usb"}' '10.0.0.42:50052' unit.containers.v0.Ormos.UnmountUsbDevice
```

### Copy Container Image Archive to Local Registry
Replace IP address accordingly.
```bash
grpcurl -plaintext -import-path ./proto -proto ormos.proto -d '{"file_path": "/mnt/usb/alpine-arm64.tar", "image_name": "alpine", "image_tag": "latest"}' '10.0.0.42:50052' unit.containers.v0.Ormos.LoadImageArchive
```
