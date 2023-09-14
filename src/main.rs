#![no_std]
#![no_main]

use esp_backtrace as _;
use esp_println::println;
use hal::{clock::ClockControl, peripherals::Peripherals, prelude::*, Delay, IO};

#[derive(Debug, Clone, Copy, PartialEq)]
enum ButtonState {
    Pressed,
    JustPressed,
    Released,
    JustReleased,
}

struct Button {
    pin: u8,
    state: ButtonState,
}

impl Button {
    pub fn new(pin: u8) -> Self {
        Button {
            pin,
            state: ButtonState::Released,
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
}

#[entry]
fn main() -> ! {
    let peripherals = Peripherals::take();
    let system = peripherals.DPORT.split();
    let clocks = ClockControl::boot_defaults(system.clock_control).freeze();
    let mut delay = Delay::new(&clocks);

    println!("Hello world!");

    let io = IO::new(peripherals.GPIO, peripherals.IO_MUX);

    let left_button_ctr = io.pins.gpio0.into_pull_up_input();
    let right_button_ctr = io.pins.gpio35.into_pull_up_input();

    let mut left_button = Button::new(left_button_ctr.number());
    let mut right_button = Button::new(right_button_ctr.number());

    loop {
        left_button.update(left_button_ctr.is_high().unwrap());
        right_button.update(right_button_ctr.is_high().unwrap());

        match (left_button.state(), right_button.state()) {
            (ButtonState::JustPressed, ButtonState::JustPressed) => {
                println!("Both buttons pressed")
            }
            (ButtonState::JustReleased, ButtonState::JustReleased) => {
                println!("Both buttons released")
            }
            (ButtonState::JustPressed, _) => println!("Left button pressed"),
            (_, ButtonState::JustPressed) => println!("Right button pressed"),
            (ButtonState::JustReleased, _) => println!("Left button released"),
            (_, ButtonState::JustReleased) => println!("Right button released"),
            _ => {}
        }

        delay.delay_ms(100u32);
    }
}
