#![no_std]
#![no_main]

use esp_backtrace as _;
use esp_println::print;
use esp_println::println;
use hal::clock::ClockControl;
use hal::peripherals::Peripherals;
use hal::prelude::*;
use hal::Delay;
use hal::IO;

use embedded_svc::ipv4::Interface;
use embedded_svc::wifi::{AccessPointInfo, ClientConfiguration, Configuration, Wifi};
use esp_wifi::wifi::utils::create_network_interface;
use esp_wifi::wifi::WifiMode;
use esp_wifi::wifi_interface::WifiStack;
use esp_wifi::{initialize, EspWifiInitFor};
use esp_wifi::current_millis;
use smoltcp::iface::SocketStorage;
use smoltcp::wire::IpAddress;
use smoltcp::wire::Ipv4Address;
use embedded_io::blocking::{Read, Write};

const SSID: &str = env!("SSID");
const PASSWORD: &str = env!("PASSWORD");

#[entry]
fn main() -> ! {
    let peripherals = Peripherals::take();
    let mut system = peripherals.DPORT.split();
    let clocks = ClockControl::max(system.clock_control).freeze();
    let mut delay = Delay::new(&clocks);

    println!("Hello world!");

    let io = IO::new(peripherals.GPIO, peripherals.IO_MUX);
    let pins = io.pins;

    // Wifi

    let timer = hal::timer::TimerGroup::new(
        peripherals.TIMG1,
        &clocks,
        &mut system.peripheral_clock_control,
    )
    .timer0;

    let init = initialize(
        EspWifiInitFor::Wifi,
        timer,
        hal::Rng::new(peripherals.RNG),
        system.radio_clock_control,
        &clocks,
    )
    .unwrap();

    let (wifi, ..) = peripherals.RADIO.split();
    let mut socket_set_entries: [SocketStorage; 3] = Default::default();
    let (iface, device, mut controller, sockets) =
        create_network_interface(&init, wifi, WifiMode::Sta, &mut socket_set_entries).unwrap();
    let wifi_stack = WifiStack::new(iface, device, sockets, current_millis);

    let client_config = Configuration::Client(ClientConfiguration {
        ssid: SSID.into(),
        password: PASSWORD.into(),
        ..Default::default()
    });

    let res = controller.set_configuration(&client_config);
    println!("wifi_set_configuration returned {:?}", res);

    controller.start().unwrap();
    println!("is wifi started: {:?}", controller.is_started());

    println!("Start Wifi Scan");
    type ScanResult =
        Result<(heapless::Vec<AccessPointInfo, 10>, usize), esp_wifi::wifi::WifiError>;
    let res: ScanResult = controller.scan_n();
    if let Ok((res, _count)) = res {
        for ap in res {
            println!("{:?}", ap);
        }
    }

    println!("{:?}", controller.get_capabilities());
    println!("wifi_connect {:?}", controller.connect());

    println!("Connecting to {SSID}");

    // wait to get connected
    println!("Wait to get connected");
    loop {
        let res = controller.is_connected();
        match res {
            Ok(connected) => {
                if connected {
                    break;
                }
            }
            Err(err) => {
                println!("Failed to connect {:?}", err);
                loop {}
            }
        }
    }

    println!("Connected {:?}", controller.is_connected());

    // wait for getting an ip address
    println!("Wait to get an ip address");
    loop {
        wifi_stack.work();

        if wifi_stack.is_iface_up() {
            println!("got ip {:?}", wifi_stack.get_ip_info());
            break;
        }
    }

    println!("Start busy loop on main");

    let mut rx_buffer = [0u8; 1536];
    let mut tx_buffer = [0u8; 1536];
    let mut socket = wifi_stack.get_socket(&mut rx_buffer, &mut tx_buffer);
    
    {
        println!("Sending message");
        socket.work();

        socket
            .open(IpAddress::Ipv4(Ipv4Address::new(192, 168, 0, 15)), 60500)
            .unwrap();

        socket
            .write(b"< Hello from ttgo!\n> ")
            .unwrap();

        socket.flush().unwrap();

        let wait_end = current_millis() + 20 * 1000;
        loop {
            let mut buffer = [0u8; 512];
            if let Ok(len) = socket.read(&mut buffer) {
                let msg = unsafe { core::str::from_utf8_unchecked(&buffer[..len]) };
                print!("> {}", msg);

                if msg.contains("END") {
                    break;
                }

                socket.write(b"< ACK \n> ").unwrap();
                socket.flush().unwrap();       
            } else {
                break;
            }
        }

        socket.disconnect();
    }

    println!("Done");

    loop {
        delay.delay_ms(1000u32);
    }
}
