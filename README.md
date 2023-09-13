# Playground for the TTGO ESP32 T-Display

## Installation

This is the steps I took to get this project up and running. I'm using VSCode as my IDE.

### Install [espup](https://github.com/esp-rs/espup#installation) and the toolchain.

```bash
cargo install espup

# Check that it's installed
espup --version

# Install the toolchain
espup install
# [2023-09-13T17:17:50Z WARN ] 💡  Please, set up the environment variables by running: '. ~/export-esp.sh'
# [2023-09-13T17:17:50Z WARN ] ⚠️   This step must be done every time you open a new terminal.
# [2023-09-13T17:17:50Z INFO ] ✅  Installation successfully completed!
```

### Install [espflash](https://github.com/esp-rs/espflash/tree/main/espflash)
```bash
cargo install espflash
```

### Switch to the esp toolchain
```bash
rustup override set esp

# Or if you want to switch back to the default toolchain
rustup override unset

# If you want to get rin of Better TOML errors, you'll need to set as default
rustup default esp

# And if you want to switch back to the default toolchain
rustup default stable
```

### Set permissions for the USB device
```bash
# Get the USB device id
lsusb # Bus 001 Device 017: ID 1a86:55d4 QinHeng Electronics USB Single Serial

# Create a new udev rule
sudo nano /etc/udev/rules.d/98-esp32.rules

# Add the following line
ATTRS{idVendor}=="1a86", ATTRS{idProduct}=="55d4", MODE="0666", GROUP="plugdev"

# Reload the rules
sudo udevadm control --reload-rules

# Create the plugdev group
sudo groupadd plugdev

# Add yourself to the plugdev group
sudo usermod -a -G plugdev $USER

# Unplug and plug the device back in
```

## Build and flash the project
```bash
# Runner configured in .cargo/config.toml 
cargo run

# Flash the device
espflash flash target/xtensa-esp32-none-elf/release/ttgo-playground --monitor
```

## References

- https://github.com/ivmarkov/rust-esp32-std-demo
- https://github.com/esp-rs/rust-build
- https://github.com/esp-rs/rust-build#espup-installation
- https://github.com/esp-rs/espup#installation
- https://github.com/esp-rs/no_std-training
- https://esp-rs.github.io/book/introduction.html
