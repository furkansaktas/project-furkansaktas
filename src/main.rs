#![no_std]
#![no_main]

use defmt::*;
use defmt_rtt as _;
use embassy_executor::Spawner;
use embassy_rp::gpio::{Level, Output};
use embassy_rp::i2c::{self, I2c};
use embassy_rp::peripherals::*;
use embassy_rp::pwm::{self, Pwm};
use embassy_time::{Delay, Timer};
use panic_probe as _;

use bmp280_ehal::BMP280;
use ag_lcd::{I2cLcd, Lcd};

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    info!("SmartAir project started.");

    let p = embassy_rp::init(Default::default());
    let mut delay = Delay;

    // === I2C for BMP280 ===
    let scl = p.PIN_1;
    let sda = p.PIN_0;
    let i2c_sensor = I2c::new_blocking(p.I2C0, scl, sda, i2c::Config::default());

    let mut bmp280 = match BMP280::new_primary(i2c_sensor, delay) {
        Ok(sensor) => sensor,
        Err(e) => {
            error!("BMP280 init error: {:?}", e);
            return;
        }
    };

    if let Err(e) = bmp280.init() {
        error!("BMP280 setup failed: {:?}", e);
        return;
    }

    // === I2C for LCD ===
    let scl_lcd = p.PIN_3;
    let sda_lcd = p.PIN_2;
    let i2c_lcd = I2c::new_blocking(p.I2C1, scl_lcd, sda_lcd, i2c::Config::default());

    let mut lcd = I2cLcd::new(i2c_lcd, 0x27, Delay);
    if let Err(e) = lcd.init(&mut Delay) {
        error!("LCD init error: {:?}", e);
        return;
    }
    let _ = lcd.clear(&mut Delay);

    // === Fan PWM (DC motor) ===
    let mut fan_pwm = Pwm::new(p.PWM_CH1);
    fan_pwm.set_output_b(p.PIN_6);
    fan_pwm.set_top(1000); // Set PWM resolution
    fan_pwm.enable();

    // === Passive Buzzer ===
    let mut buzzer = Output::new(p.PIN_10, Level::Low);

    loop {
        // Read temperature and pressure
        let temp = match bmp280.read_temperature() {
            Ok(t) => t,
            Err(_) => {
                error!("Temperature read failed");
                continue;
            }
        };

        let pressure = match bmp280.read_pressure() {
            Ok(p) => p,
            Err(_) => {
                error!("Pressure read failed");
                continue;
            }
        };

        info!("Temperature: {:.1}°C, Pressure: {:.0} Pa", temp, pressure);

        // Update LCD
        let _ = lcd.set_cursor_pos(0, 0, &mut Delay);
        let _ = lcd.write_str("Temp: ", &mut Delay);
        let _ = lcd.write_str_fmt(format_args!("{:.1}C", temp), &mut Delay);

        let _ = lcd.set_cursor_pos(1, 0, &mut Delay);
        let _ = lcd.write_str("Press: ", &mut Delay);
        let _ = lcd.write_str_fmt(format_args!("{:.0}Pa", pressure), &mut Delay);

        // Control logic
        if temp > 28.0 {
            fan_pwm.set_duty(fan_pwm.get_max_duty() / 2); // 50%
            buzzer.set_high();
        } else {
            fan_pwm.set_duty(0);
            buzzer.set_low();
        }

        Timer::after_secs(2).await;
    }
}
