# skopos
A gRPC server that facilitates provisioning and updating containers.

## Usage
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
