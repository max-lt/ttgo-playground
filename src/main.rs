
use embedded_storage::{ReadStorage, Storage};
// use embedded_hal::digital::v2::OutputPin;
use esp_backtrace as _;
use esp_println::println;
use esp_storage::FlashStorage;
use hal::{clock::ClockControl, peripherals::Peripherals, prelude::*, Delay, IO};
use esp_idf_hal::gpio;

#[derive(Debug, Clone, Copy, PartialEq)]
enum ButtonState {
    Pressed,
    JustPressed,
    Released,
    JustReleased,
}

struct Button {
    pin: u8,
    flash_offset: u32,
    state: ButtonState,
    count: u32,
}

impl Button {
    pub fn new(pin: u8, flash_offset: u32) -> Self {
        Button {
            pin,
            flash_offset,
            state: ButtonState::Released,
            count: 0,
        }
    }

    pub fn state(&self) -> ButtonState {
        self.state
    }

    pub fn update(&mut self, is_high: bool) {
        // High means the button is not pressed
        self.state = match is_high {
            true => match self.state {
                ButtonState::Pressed | ButtonState::JustPressed => ButtonState::JustReleased,
                ButtonState::Released | ButtonState::JustReleased => ButtonState::Released,
            },
            false => match self.state {
                ButtonState::Pressed | ButtonState::JustPressed => ButtonState::Pressed,
                ButtonState::Released | ButtonState::JustReleased => ButtonState::JustPressed,
            },
        };
    }

    pub fn count(&self) -> u32 {
        self.count
    }

    pub fn increment(&mut self) {
        self.count += 1;
    }

    pub fn write(&self, flash: &mut FlashStorage) {
        flash
            .write(self.flash_offset, self.count.to_le_bytes().as_ref())
            .expect("Failed to write to flash");
    }

    pub fn read(&mut self, flash: &mut FlashStorage) {
        let mut data = [0u8; 4];

        flash
            .read(self.flash_offset, &mut data)
            .expect("Failed to read from flash");

        self.count = u32::from_le_bytes(data);
    }
}

const MAGIC: u32 = 0xab01cd02;

/// Check if the magic is written, if not, write it and clear the rest of the used flash
fn check_memory(flash: &mut FlashStorage, flash_offset: u32) {
    println!("Flash size = {}", flash.capacity());

    if flash.capacity() < 128 {
        panic!("Flash is too small");
    }

    let mut magic_buf = [0u8; 4];
    flash.read(flash_offset, &mut magic_buf).unwrap();

    if u32::from_le_bytes(magic_buf) != MAGIC {
        println!("Writing magic");

        flash
            .write(flash_offset, MAGIC.to_le_bytes().as_ref())
            .unwrap();
        flash.write(flash_offset + 4, [0u8; 124].as_ref()).unwrap();
    }
}

#[entry]
fn main() -> ! {
    let peripherals = Peripherals::take();
    let system = peripherals.DPORT.split();
    let clocks = ClockControl::boot_defaults(system.clock_control).freeze();
    let mut delay = Delay::new(&clocks);

    println!("Hello world!");

    let io = IO::new(peripherals.GPIO, peripherals.IO_MUX);
    let pins = io.pins;

    let left_button_ctr = pins.gpio0.into_pull_up_input();
    let right_button_ctr = pins.gpio35.into_pull_up_input();

    let mut flash = FlashStorage::new();
    let flash_offset = 0x9000;
    check_memory(&mut flash, flash_offset);

    let mut left_button = Button::new(left_button_ctr.number(), flash_offset + 4);
    left_button.read(&mut flash);

    let mut right_button = Button::new(right_button_ctr.number(), flash_offset + 8);
    right_button.read(&mut flash);

    println!(
        "Left button count: {}, right button count: {}",
        left_button.count(),
        right_button.count()
    );

    {
        let mut backlight = pins.gpio4.into_push_pull_output();

        let dc = pins.gpio16; // Data/Command (data or command signal from main to subs)
        let mut rst = pins.gpio23.into_push_pull_output(); // Reset (active low signal from main to reset subs)
        let spi = peripherals.SPI2; // Serial Peripheral Interface
        let sclk = pins.gpio18; // SCLK : Serial Clock (clock signal from main)
        let sdo = pins.gpio19; // mosi: Main Out Sub In (data output from main)
        let cs = pins.gpio5; // Chip Select (active low signal from main to address subs and initiate transmission)

        backlight.set_high().unwrap();

        println!("Reset screen");

        // rst.set_low().unwrap();

        // let di = SPIInterfaceNoCS::new(
        //     spi::SpiDeviceDriver::new_single(
        //         spi,
        //         sclk,
        //         sdo,
        //         Option::<gpio::Gpio21>::None,
        //         Some(cs),
        //         &spi::SpiDriverConfig::new().dma(spi::Dma::Disabled),
        //         &spi::SpiConfig::new().baudrate(26.MHz().into()),
        //     )?,
        //     gpio::PinDriver::output(dc)?,
        // );

        // let mut display = mipidsi::Builder::st7789(di)
        //     .init(&mut delay::Ets, Some(gpio::PinDriver::output(rst)?))
        //     .map_err(|e| anyhow::anyhow!("Display error: {:?}", e))?;

        // display
        //     .set_orientation(mipidsi::options::Orientation::Portrait(false))
        //     .map_err(|e| anyhow::anyhow!("Display error: {:?}", e))?;

        // // The TTGO board's screen does not start at offset 0x0, and the physical size is 135x240, instead of 240x320
        // let top_left = Point::new(52, 40);
        // let size = Size::new(135, 240);

        // led_draw(&mut display.cropped(&Rectangle::new(top_left, size)))
        //     .map_err(|e| anyhow::anyhow!("Display error: {:?}", e))
    };

    loop {
        {
            left_button.update(left_button_ctr.is_high().unwrap());
            right_button.update(right_button_ctr.is_high().unwrap());
        }

        match (left_button.state(), right_button.state()) {
            // If both buttons are pressed, print counters and save them to flash
            (ButtonState::JustPressed, ButtonState::JustPressed) => {
                println!("Both buttons pressed, writing counters to flash");
                println!(
                    "Left button count: {}, right button count: {}",
                    left_button.count(),
                    right_button.count()
                );
                left_button.write(&mut flash);
                right_button.write(&mut flash);
            }
            (ButtonState::JustReleased, ButtonState::JustReleased) => {
                println!("Both buttons released");
            }
            (ButtonState::JustPressed, _) => {
                left_button.increment();
                println!("Left button pressed: {}", left_button.count());
            }
            (_, ButtonState::JustPressed) => {
                right_button.increment();
                println!("Right button pressed: {}", right_button.count());
            }
            (ButtonState::JustReleased, _) => println!("Left button released"),
            (_, ButtonState::JustReleased) => println!("Right button released"),
            _ => {}
        }

        delay.delay_ms(100u32);
    }
}
