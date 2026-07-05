# Playground for the TTGO ESP32 T-Display

## Installation

This is the steps I took to get this project up and running. I'm using VSCode as my IDE.

### Install [espup](https://github.com/esp-rs/espup#installation) and the toolchain.

```bash
# NOTE: `cargo install espup` FAILS from inside this repo — rust-toolchain.toml pins
# `channel = "esp"`, which isn't installed yet. Run it from your home dir, force
# +stable, or (fastest) grab the prebuilt binary:

# macOS (Apple Silicon), no compile:
curl -L https://github.com/esp-rs/espup/releases/latest/download/espup-aarch64-apple-darwin \
  -o ~/.cargo/bin/espup && chmod +x ~/.cargo/bin/espup
# ...or build it (run OUTSIDE this repo, or force stable):
cargo +stable install espup

# Check that it's installed
espup --version

# Install the esp toolchain + Xtensa target + LLVM (~1-2 GB). The default (latest) is
# fine: this project targets esp-hal 1.1, which builds on the current esp toolchain.
espup install
# 💡  Please, set up the environment variables by running: '. ~/export-esp.sh'
# ⚠️   This step must be done every time you open a new terminal.
# ✅  Installation successfully completed!
```

### Install [espflash](https://github.com/esp-rs/espflash/tree/main/espflash)
```bash
# Same caveat as espup: run OUTSIDE this repo (the .cargo/config.toml `target = xtensa`
# override otherwise makes cargo try to build espflash for the wrong target).
cargo install espflash
```

### Switch to the esp toolchain (optional as rust-toolchain.toml does it)
```bash
rustup override set esp

# Or if you want to switch back to the default toolchain
rustup override unset

# If you want to get rin of Better TOML errors, you'll need to set as default
rustup default esp

# And if you want to switch back to the default toolchain
rustup default stable
```

### USB serial device

#### macOS

Recent macOS ships the CP210x / CH34x USB-serial drivers built in, so the T-Display
usually just works. Plug it in and confirm it enumerates:

```bash
ls /dev/cu.*   # look for /dev/cu.usbserial-* or /dev/cu.wchusbserial-*
```

If nothing new appears when you plug it in, install the vendor driver
(Silicon Labs CP210x, or WCH CH34x for the CH9102).

#### Linux (udev)
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
cargo run --release

# ...or flash the already-built binary directly (lighter; no cargo/toolchain env needed):
espflash flash --monitor target/xtensa-esp32-none-elf/release/ttgo-playground
```

## References

- https://github.com/ivmarkov/rust-esp32-std-demo
- https://github.com/esp-rs/rust-build
- https://github.com/esp-rs/rust-build#espup-installation
- https://github.com/esp-rs/espup#installation
- https://github.com/esp-rs/no_std-training
- https://esp-rs.github.io/book/introduction.html
- https://github.com/Xinyuan-LilyGO/TTGO-T-Display#pinout
