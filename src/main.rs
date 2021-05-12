#![no_main]
#![no_std]
use stm32f1::stm32f107;
use core::panic::PanicInfo;
use stm32f1::stm32f107::Peripherals;


#[panic_handler]
fn handler(_info: &PanicInfo) -> ! {
    loop {
        // p a n i c c
    }
}

// note this is totally wrong, I have the prescalers setup to make it delay long enough
//   to see an LED blink but I didn't do the math. Don't trust this, and I wouldn't use this.
fn delay_us(peripherals : &Peripherals, delay: u16) {
    peripherals.TIM2.cnt.write(|w| w.cnt().bits(0));

    peripherals.TIM2.cr1.write(|w| {
        w.cen().bit(true)  // 1: enable / start
    });

    // wait until bit is not set
    while peripherals.TIM2.cnt.read().cnt().bits() < delay {}

    peripherals.TIM2.cr1.write(|w| {
        w.cen().bit(false)  // stop the counter
    })
}

#[no_mangle]
pub fn main() {
    let peripherals : Peripherals = stm32f107::Peripherals::take().unwrap();

    // identify TIM2 is on the APB1 from the 107 RM S2.3 (p13)
    peripherals.RCC.apb1enr.write(|w| w.tim2en().bit(true));
    peripherals.TIM2.psc.write(|w| w.psc().bits(0xffff));
    peripherals.RCC.cfgr.write(|w| w.ppre1().div16());

    // enable the clock for IO port D (as our LED is on GPIO D-13)
    peripherals.RCC.apb2enr.write(|w| w.iopden().bit(true));

    // LED is on GPIO D13, set it to a push-pull output
    peripherals.GPIOD.crh.write(|w| {
        w.mode13().output();
        w.cnf13().push_pull()
    });

    loop {
        // set GPIO D-13 ON
        peripherals.GPIOD.odr.write(|w| w.odr13().bit(true));
        delay_us(&peripherals, 50000);

        // set GPIO D-13 OFF
        peripherals.GPIOD.odr.write(|w| w.odr13().bit(false));
        delay_us(&peripherals, 50000);
    }
}
