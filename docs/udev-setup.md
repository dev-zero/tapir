# USB Modeswitch & Permissions Setup

The Dymo LabelManager PnP enumerates as a USB mass-storage device (`0922:1001`) on plug-in.
It must be switched to printer mode (`0922:1002`) before it accepts print data.

## Manual Fix

```bash
# Switch from storage mode to printer mode
sudo usb_modeswitch -v 0922 -p 1001 -m 0x01 -r 0x81 -M 1b5a01

# Verify the device re-enumerated
lsusb | grep 0922
# Expected: Bus xxx Device xxx: ID 0922:1002 Dymo-CoStar Corp. LabelManager PnP
```

The `-m 0x01` is the OUT endpoint (interrupt), `-r 0x81` is the IN endpoint (interrupt),
both on HID interface 0. The payload `1b5a01` is the ESC-Z-0x01 firmware switch command.

After switching, set permissions so the app can open the device without root:

```bash
sudo chmod 666 /dev/bus/usb/$(lsusb | grep 0922:1002 | awk '{print $2}')/$(lsusb | grep 0922:1002 | awk '{print $4}' | tr -d :)
```

## Automated Fix (udev rules)

Create `/etc/udev/rules.d/99-dymo-labelmanager.rules`:

```udev
# Dymo LabelManager PnP — trigger modeswitch when storage mode detected
ACTION=="add", ATTR{idVendor}=="0922", ATTR{idProduct}=="1001", \
  RUN+="/usr/sbin/usb_modeswitch -v 0922 -p 1001 -m 0x01 -r 0x81 -M 1b5a01"

# Dymo LabelManager PnP — allow non-root access after modeswitch
ACTION=="add", ATTR{idVendor}=="0922", ATTR{idProduct}=="1002", \
  MODE="0666"

# Dymo LabelManager 420P
ACTION=="add", ATTR{idVendor}=="0922", ATTR{idProduct}=="1003", \
  RUN+="/usr/sbin/usb_modeswitch -v 0922 -p 1003 -m 0x01 -r 0x81 -M 1b5a01"
ACTION=="add", ATTR{idVendor}=="0922", ATTR{idProduct}=="1004", \
  MODE="0666"

# Dymo LabelManager 280
ACTION=="add", ATTR{idVendor}=="0922", ATTR{idProduct}=="1005", \
  RUN+="/usr/sbin/usb_modeswitch -v 0922 -p 1005 -m 0x01 -r 0x81 -M 1b5a01"
ACTION=="add", ATTR{idVendor}=="0922", ATTR{idProduct}=="1006", \
  MODE="0666"

# Dymo LabelManager Wireless PnP
ACTION=="add", ATTR{idVendor}=="0922", ATTR{idProduct}=="1007", \
  RUN+="/usr/sbin/usb_modeswitch -v 0922 -p 1007 -m 0x01 -r 0x81 -M 1b5a01"
ACTION=="add", ATTR{idVendor}=="0922", ATTR{idProduct}=="1008", \
  MODE="0666"
```

Then reload:

```bash
sudo udevadm control --reload-rules
sudo udevadm trigger
```

NOTE: On Debian with `usb-modeswitch-data` installed, the modeswitch rules already exist
in `/usr/lib/udev/rules.d/40-usb_modeswitch.rules`. You only need the custom rule above
if you want to bypass the dispatcher, or on distros without `usb-modeswitch-data`.
If using the built-in dispatcher with the override config (see below), you only need
the `MODE="0666"` rules for permissions:

## Dependencies

```bash
sudo apt install usb-modeswitch usb-modeswitch-data
```

## Preventing Phantom Block Device

The kernel's `usb-storage` driver races with modeswitch and briefly claims the device's
mass storage interface, creating a `/dev/sdX` block device. Even after modeswitch
succeeds, the stale block device may persist.

Fix: tell `usb-storage` to ignore all Dymo storage-mode product IDs.

Create `/etc/modprobe.d/dymo-no-storage.conf`:

```
options usb-storage quirks=0922:1001:i,0922:1003:i,0922:1005:i,0922:1007:i
```

Then rebuild initramfs (since `usb-storage` may be loaded from initrd):

```bash
sudo update-initramfs -u
```

To remove an existing stale block device without rebooting:

```bash
echo 1 | sudo tee /sys/block/sdb/device/delete
```

## Why Debian's Built-in Modeswitch Fails

Debian ships a udev rule and config for the Dymo (`usb-modeswitch-data` package).
The chain is:

1. Udev matches `0922:1001` → calls `/lib/udev/usb_modeswitch '/%k'`
2. Script starts `usb_modeswitch@DEVICE.service` via systemd
3. Service runs `usb_modeswitch_dispatcher --switch-mode DEVICE`
4. Dispatcher extracts config from `/usr/share/usb_modeswitch/configPack.tar.gz`

The config in the tarball is **buggy**:

```
# /usr/share/usb_modeswitch/configPack.tar.gz -> 0922:1001
TargetVendor=0x0922
TargetProduct=0x1002
MessageEndpoint=0x01
ResponseEndpoint=0x01   ← BUG: 0x01 is OUT, should be 0x81 (IN)
MessageContent="1b5a01"
```

This causes `usb_modeswitch` to abort with:
"Error: response endpoint not given or found. Abort"

## Fix for Built-in Dispatcher (alternative to custom udev rule)

Override the broken config by placing a corrected file in `/etc/usb_modeswitch.d/`:

```bash
sudo mkdir -p /etc/usb_modeswitch.d
sudo tee /etc/usb_modeswitch.d/0922:1001 << 'EOF'
TargetVendor=0x0922
TargetProduct=0x1002
MessageEndpoint=0x01
ResponseEndpoint=0x81
MessageContent="1b5a01"
EOF
```

The dispatcher checks `/etc/usb_modeswitch.d/` before the tarball and will use this override.
With this fix, the built-in systemd-triggered modeswitch works without any custom udev rule.

You still need a udev rule for permissions on the switched device:

```udev
# /etc/udev/rules.d/99-dymo-labelmanager.rules
ACTION=="add", ATTR{idVendor}=="0922", ATTR{idProduct}=="1002", MODE="0666"
ACTION=="add", ATTR{idVendor}=="0922", ATTR{idProduct}=="1004", MODE="0666"
ACTION=="add", ATTR{idVendor}=="0922", ATTR{idProduct}=="1006", MODE="0666"
ACTION=="add", ATTR{idVendor}=="0922", ATTR{idProduct}=="1008", MODE="0666"
```

## Verification

```bash
# Plug in the device, then:
lsusb | grep 0922
# Should show 0922:1002 (not 1001)

# Check permissions:
ls -la /dev/bus/usb/$(lsusb | grep 0922:1002 | awk '{print $2}')/$(lsusb | grep 0922:1002 | awk '{print $4}' | tr -d :)
# Should show crw-rw-rw-
```

## macOS

USB modeswitch is not possible from userspace on macOS. The `IOUSBMassStorageClass` kernel
extension holds exclusive access to the device in storage mode, and cannot be unloaded
(`kextunload` fails with "kext is in use"). The device must be switched on a Linux host.
