#![no_std]
#![no_main]

use defmt::*;
use defmt_rtt as _;
use embassy_executor::Spawner;
use embassy_rp::peripherals::*;
use embassy_rp::gpio::{Level, Output};
use embassy_rp::pwm::Pwm;
use embassy_rp::i2c::I2c;
use embassy_time::{Delay, Timer};
use embassy_rp::i2c;
use panic_probe as _;

use bmp280_ehal::BMP280;
use ag_lcd::{I2cLcd, Lcd};

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    info!("SmartAir project started.");

    let p = embassy_rp::init(Default::default());

    // === I2C for BMP280 and LCD ===
    let scl = p.PIN_1; // Adjust these according to your wiring
    let sda = p.PIN_0;
    let i2c = I2c::new_blocking(p.I2C0, scl, sda, i2c::Config::default());

    let mut delay = Delay;

    // === Initialize BMP280 ===
    let mut bmp280 = BMP280::new_primary(i2c, delay).unwrap();
    bmp280.init().unwrap();

    // === Re-split I2C for LCD (simple workaround, use real bus sharing in prod) ===
    let scl_lcd = p.PIN_3;
    let sda_lcd = p.PIN_2;
    let i2c_lcd = I2c::new_blocking(p.I2C1, scl_lcd, sda_lcd, i2c::Config::default());

    let mut lcd = I2cLcd::new(i2c_lcd, 0x27, Delay);
    lcd.init(&mut Delay).unwrap();
    lcd.clear(&mut Delay).unwrap();

    // === DC Motor (PWM Pin) ===
    let mut fan_pwm = Pwm::new_output_b(p.PWM_CH1, p.PIN_6); // Use the correct PWM channel and pin
    fan_pwm.set_duty(fan_pwm.get_max_duty() / 2); // 50% speed
    fan_pwm.enable();

    // === Buzzer (GPIO Output) ===
    let mut buzzer = Output::new(p.PIN_10, Level::Low);

    loop {
        // Read temp from BMP280
        let temp = bmp280.read_temperature().unwrap();
        let pressure = bmp280.read_pressure().unwrap();

        info!("Temperature: {} °C, Pressure: {} Pa", temp, pressure);

        // Write to LCD
        lcd.set_cursor_pos(0, 0, &mut Delay).unwrap();
        lcd.write_str("Temp: ", &mut Delay).unwrap();
        lcd.write_str_fmt(format_args!("{:.1}C", temp), &mut Delay).unwrap();

        lcd.set_cursor_pos(1, 0, &mut Delay).unwrap();
        lcd.write_str("Press: ", &mut Delay).unwrap();
        lcd.write_str_fmt(format_args!("{:.0}Pa", pressure), &mut Delay).unwrap();

        // Logic: if temperature > threshold, turn on fan + buzzer
        if temp > 28.0 {
            fan_pwm.set_duty(fan_pwm.get_max_duty() / 2);
            buzzer.set_high();
        } else {
            fan_pwm.set_duty(0);
            buzzer.set_low();
        }

        Timer::after_secs(2).await;
    }
}
