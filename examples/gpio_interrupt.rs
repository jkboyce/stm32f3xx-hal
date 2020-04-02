//! Example of an interrupt handler driven by a GPIO input.
//! Target board: STM32F3DISCOVERY
#![no_std]
#![no_main]

use panic_semihosting as _;

use core::cell::RefCell;
use core::ops::DerefMut;
use cortex_m::{
    interrupt::{free, Mutex},
    peripheral::NVIC,
};
use cortex_m_rt::entry;
use stm32f3xx_hal as hal;
use crate::hal::{
    gpio::{gpioe::PE13, Output, PushPull},
    interrupt,
    prelude::*,
    stm32,
};
use crate::stm32::EXTI;

// Set up global state as Mutex, for safe concurrent access from interrupt handler.
static MUTEX_LED: Mutex<RefCell<Option<PE13<Output<PushPull>>>>> = Mutex::new(RefCell::new(None));

#[entry]
fn main() -> ! {
    // Get our peripherals
    let dp = stm32::Peripherals::take().unwrap();

    // enable SYSCFG clock to enable external interrupts; must come before RCC.constrain()
    dp.RCC.apb2enr.write(|w| w.syscfgen().enabled());

    // Configure clocks
    let mut flash = dp.FLASH.constrain();
    let mut rcc = dp.RCC.constrain();
    let _clocks = rcc.cfgr.sysclk(16.mhz()).freeze(&mut flash.acr);

    // select PA0 (user button) as source input for EXTI0 external interrupt
    dp.SYSCFG
        .exticr1
        .modify(|r, w| unsafe { w.bits(r.bits() & !0xf) });

    // configure EXTI0 to trigger on rising edge, disable trigger on falling edge
    dp.EXTI.rtsr1.modify(|r, w| unsafe { w.bits(r.bits() | 1) });
    dp.EXTI
        .ftsr1
        .modify(|r, w| unsafe { w.bits(r.bits() & !1) });

    // unmask interrupt line 0 (EXTI0)
    dp.EXTI.imr1.modify(|r, w| unsafe { w.bits(r.bits() | 1) });

    // configure "South" led on board (pin PE13)
    let mut gpioe = dp.GPIOE.split(&mut rcc.ahb);
    let mut led = gpioe
        .pe13
        .into_push_pull_output(&mut gpioe.moder, &mut gpioe.otyper);
    led.set_high().unwrap();

    // move value into our static RefCell so we can access from interrupt handler
    free(|cs| MUTEX_LED.borrow(cs).replace(Some(led)));

    unsafe { NVIC::unmask(interrupt::EXTI0) };

    loop {}
}

#[interrupt]
fn EXTI0() {
    free(|cs| {
        // reset pending bit for interrupt line 0
        unsafe { (*EXTI::ptr()).pr1.write(|w| w.bits(1)) };

        if let Some(ref mut led) = MUTEX_LED.borrow(cs).borrow_mut().deref_mut() {
            led.toggle().unwrap();
        }
    });
}
