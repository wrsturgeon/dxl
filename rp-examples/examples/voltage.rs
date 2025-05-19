#![no_std]
#![no_main]
#![feature(impl_trait_in_assoc_type)]

use {
    defmt_rtt as _,
    embassy_executor::Spawner,
    embassy_rp::{
        adc::{self, Adc},
        bind_interrupts, gpio,
        peripherals::USB,
        usb,
    },
    embassy_time::{Duration, Ticker},
    panic_probe as _,
};

bind_interrupts!(struct Irqs {
    ADC_IRQ_FIFO => adc::InterruptHandler;
    USBCTRL_IRQ => usb::InterruptHandler<USB>;
});

// ADC measures from a voltage divider (10K/1K: 11x reduction),
// and ADC samples with 12-bit resolution.
// We want the maximum measurement (1 << 12, exclusive)
// to represent 36V3 (that is, 3V3 * 11), so we have to
// multiply raw ADC samples by (36V3 / (1 << 12)).
const ADC_CORRECTION: f32 = 36.3_f32 / ((1 << 12) as f32);

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_rp::init(Default::default());

    {
        // USB background task:
        #[embassy_executor::task]
        pub async fn task(driver: usb::Driver<'static, USB>) {
            embassy_usb_logger::run!(1024, log::LevelFilter::Info, driver);
        }
        match spawner.spawn(task(usb::Driver::new(p.USB, Irqs))) {
            Ok(()) => {
                let () = defmt::info!("Spawned USB task");
                let () = log::info!("Spawned USB task");
            }
            Err(e) => {
                let () = log::error!("Error spawning USB task: {}", e);
                let () = defmt::panic!("Error spawning USB task: {}", e);
            }
        }
    }

    let mut adc = Adc::new(p.ADC, Irqs, adc::Config::default());
    let mut adc_pin = p.PIN_28;
    let mut adc_channel = adc::Channel::new_pin(&mut adc_pin, gpio::Pull::None);

    let mut ticker = Ticker::every(Duration::from_secs(1));
    'forever: loop {
        let voltage = {
            let adc_sample: u16 = match adc.read(&mut adc_channel).await {
                Ok(sample) => sample,
                Err(e) => {
                    defmt::error!("ADC error: {}", e);
                    log::error!("ADC error: {:?}", e);
                    continue 'forever;
                }
            };
            adc_sample as f32 * ADC_CORRECTION
        };

        // defmt::info!("Power line voltage (to Dynamixels): {=f32:2.1} volts", voltage);
        defmt::info!("Power line voltage (to Dynamixels): {=f32} volts", voltage);
        log::info!("Power line voltage (to Dynamixels): {voltage:2.1} volts");
        let () = ticker.next().await;
    }
}
