#![no_std]
#![no_main]

mod snake;

use embedded_storage::ReadStorage;
use embedded_storage::Storage;
use esp_backtrace as _;
use esp_println::println;
use esp_storage::FlashStorage;

use esp_hal::delay::Delay;
use esp_hal::gpio::{Input, InputConfig, Level, Output, OutputConfig, Pull};
use esp_hal::main;
use esp_hal::spi::master::{Config as SpiConfig, Spi};
use esp_hal::spi::Mode;
use esp_hal::time::Rate;

use core::fmt::Write;

// Screen
use embedded_graphics::draw_target::DrawTarget;
use embedded_graphics::mono_font::{ascii::FONT_10X20, MonoTextStyle};
use embedded_graphics::pixelcolor::RgbColor;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::*;
use embedded_graphics::text::Text;
use embedded_hal_bus::spi::ExclusiveDevice;
use mipidsi::interface::SpiInterface;
use mipidsi::models::ST7789;
use mipidsi::options::Orientation;
use mipidsi::Builder;

// The ESP-IDF bootloader (what espflash writes) needs an app descriptor to boot.
esp_bootloader_esp_idf::esp_app_desc!();

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

    pub fn write(&self, flash: &mut FlashStorage<'_>) {
        flash
            .write(self.flash_offset, self.count.to_le_bytes().as_ref())
            .expect("Failed to write to flash");
    }

    pub fn read(&mut self, flash: &mut FlashStorage<'_>) {
        let mut data = [0u8; 4];

        flash
            .read(self.flash_offset, &mut data)
            .expect("Failed to read from flash");

        self.count = u32::from_le_bytes(data);
    }
}

const MAGIC: u32 = 0xab01cd02;

/// Check if the magic is written, if not, write it and clear the rest of the used flash
fn check_memory(flash: &mut FlashStorage<'_>, flash_offset: u32) {
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

fn draw_text<D>(display: &mut D, x: i32, y: i32, text: &str) -> Result<Point, D::Error>
where
    D: DrawTarget + Dimensions,
    D::Color: RgbColor,
{
    Text::new(
        text,
        Point::new(x, y),
        MonoTextStyle::new(&FONT_10X20, RgbColor::WHITE),
    )
    .draw(display)
}

fn clear_zone<D>(display: &mut D, x: i32, y: i32, w: u32, color: D::Color) -> Result<(), D::Error>
where
    D: DrawTarget + Dimensions,
    D::Color: RgbColor,
{
    let top_left = Point::new(x, y - 15);
    let size = Size::new(w * 10, 20);
    let mut area = display.cropped(&Rectangle::new(top_left, size));

    area.clear(color)
}

#[derive(Debug)]
struct Buf {
    pub len: usize,
    pub data: [u8; 128],
}

impl Default for Buf {
    fn default() -> Self {
        Buf {
            len: 0,
            data: [0u8; 128],
        }
    }
}

impl Buf {
    fn to_str(&self) -> &str {
        core::str::from_utf8(&self.data[..self.len]).unwrap()
    }
}

impl Write for Buf {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        self.len = 0;
        for c in s.chars() {
            self.data[self.len] = c as u8;
            self.len += 1;
        }

        Ok(())
    }
}

#[main]
fn main() -> ! {
    let peripherals = esp_hal::init(esp_hal::Config::default());

    let mut delay = Delay::new();

    println!("Hello world!");

    // Buttons: GPIO0 (left) and GPIO35 (right), pulled up (low = pressed)
    let input_config = InputConfig::default().with_pull(Pull::Up);
    let left_input = Input::new(peripherals.GPIO0, input_config);
    let right_input = Input::new(peripherals.GPIO35, input_config);

    let mut flash = FlashStorage::new(peripherals.FLASH);
    let flash_offset = 0x9000;
    check_memory(&mut flash, flash_offset);

    let mut left_button = Button::new(0, flash_offset + 4);
    left_button.read(&mut flash);

    let mut right_button = Button::new(35, flash_offset + 8);
    right_button.read(&mut flash);

    println!(
        "Left button count: {}, right button count: {}",
        left_button.count(),
        right_button.count()
    );

    // Buffer used to format text
    let mut buf = Buf {
        data: [0u8; 128],
        len: 0,
    };

    // Backlight pin, kept alive for the whole program. Off until the display is ready.
    let mut bl = Output::new(peripherals.GPIO4, Level::Low, OutputConfig::default());

    // Scratch buffer for the mipidsi SPI interface; must outlive `display`.
    let mut buffer = [0u8; 512];

    // Display
    let mut display = {
        // https://github.com/Xinyuan-LilyGO/TTGO-T-Display#pinout
        let cs = Output::new(peripherals.GPIO5, Level::High, OutputConfig::default()); // Chip Select
        let dc = Output::new(peripherals.GPIO16, Level::Low, OutputConfig::default()); // Data/Command
        let rst = Output::new(peripherals.GPIO23, Level::High, OutputConfig::default()); // Reset

        // SPI bus: SCLK=GPIO18, MOSI=GPIO19 (MISO/GPIO21 unused, display is write-only)
        let spi_bus = Spi::new(
            peripherals.SPI2,
            SpiConfig::default()
                .with_frequency(Rate::from_mhz(26))
                .with_mode(Mode::_0),
        )
        .unwrap()
        .with_sck(peripherals.GPIO18)
        .with_mosi(peripherals.GPIO19);

        // SpiBus + CS -> SpiDevice; embedded-hal-bus drives CS around each transfer
        let spi_device = ExclusiveDevice::new_no_delay(spi_bus, cs).unwrap();
        let di = SpiInterface::new(spi_device, dc, &mut buffer);

        let mut display = Builder::new(ST7789, di)
            .orientation(Orientation::new())
            .reset_pin(rst)
            .init(&mut delay)
            .unwrap();

        match display.clear(RgbColor::BLUE) {
            Ok(_) => println!("Screen cleared"),
            Err(_) => println!("Failed to clear screen"),
        }

        display
    };

    // Turn on the backlight now that the display is initialized
    bl.set_high();

    let mut area = {
        // The TTGO board's screen does not start at offset 0x0, and the physical size is 135x240, instead of 240x320
        let top_left = Point::new(52, 40);
        let size = Size::new(135, 240);
        let mut area = display.cropped(&Rectangle::new(top_left, size));

        Rectangle::new(area.bounding_box().top_left, area.bounding_box().size)
            .into_styled(
                PrimitiveStyleBuilder::new()
                    // .fill_color(RgbColor::YELLOW)
                    .stroke_color(RgbColor::RED)
                    .stroke_width(1)
                    .build(),
            )
            .draw(&mut area)
            .unwrap();

        draw_text(&mut area, 10, 20, "Hello World!").unwrap();
        draw_text(&mut area, 10, 60, "Left:").unwrap();
        draw_text(&mut area, 10, 80, "Right:").unwrap();

        write!(buf, "{}", left_button.count()).unwrap();
        draw_text(&mut area, 80, 60, buf.to_str()).unwrap();

        write!(buf, "{}", right_button.count()).unwrap();
        draw_text(&mut area, 80, 80, buf.to_str()).unwrap();

        area
    };

    let mut game = snake::Game::new();

    game.init(&mut area);

    loop {
        {
            left_button.update(left_input.is_high());
            right_button.update(right_input.is_high());
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

                write!(buf, "{}", left_button.count()).unwrap();
                clear_zone(&mut area, 80, 60, buf.len as u32, RgbColor::MAGENTA).unwrap();
                draw_text(&mut area, 80, 60, buf.to_str()).unwrap();

                println!("Left button pressed: {}", left_button.count());

                game.change_direction(snake::DirectionChange::Left);
            }
            (_, ButtonState::JustPressed) => {
                right_button.increment();

                write!(buf, "{}", right_button.count()).unwrap();
                clear_zone(&mut area, 80, 80, buf.len as u32, RgbColor::RED).unwrap();
                draw_text(&mut area, 80, 80, buf.to_str()).unwrap();

                println!("Right button pressed: {}", right_button.count());

                game.change_direction(snake::DirectionChange::Right);
            }
            (ButtonState::JustReleased, _) => println!("Left button released"),
            (_, ButtonState::JustReleased) => println!("Right button released"),
            _ => {}
        }

        game.move_snake(&mut area);

        delay.delay_millis(100);
    }
}
