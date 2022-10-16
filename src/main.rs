#![no_std]
#![no_main]

use panic_halt as _;
use gd32vf103xx_hal::pac::Interrupt;
use gd32vf103xx_hal::timer;
use gd32vf103xx_hal::timer::Timer;
use nb::block;
use heapless::{String,Vec};
use embedded_graphics::mono_font::{
    ascii::FONT_7X13_BOLD,
    MonoTextStyleBuilder,
};
use embedded_graphics::text::Text;
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{Rectangle, PrimitiveStyle};
use longan_nano::hal::{
    rcu::RcuExt,
    serial::{Config, Parity, Serial, StopBits},
    delay::McycleDelay,
};
use longan_nano::hal::{pac, prelude::*, pac::*, eclic::*};
use longan_nano::led::{Led, rgb};
use longan_nano::{lcd, lcd_pins};
use riscv_rt::entry;
use riscv::asm;

static mut INTERRUPT_FLAG:bool = false;
static mut G_TIMER1: Option<Timer<TIMER1>> = None;

#[entry]
fn main() -> ! {
    let p = pac::Peripherals::take().unwrap();
      // Configure clocks
      let mut rcu = p
      .RCU
      .configure()
      .ext_hf_clock(8.mhz())
      .sysclk(108.mhz())
      .freeze();

    let mut afio = p.AFIO.constrain(&mut rcu);

    let usart0= p.USART0;
    let gpioa = p.GPIOA.split(&mut rcu);
    let gpiob = p.GPIOB.split(&mut rcu);
    let gpioc = p.GPIOC.split(&mut rcu);
    /*******************************************************************************************************************/
    let lcd_pins = lcd_pins!(gpioa, gpiob);
    let mut lcd = lcd::configure(p.SPI0, lcd_pins, &mut afio, &mut rcu);
    let (width, height) = (lcd.size().width as i32, lcd.size().height as i32);
    /*******************************************************************************************************************/
    let delay_value = 500;
    let mut delay = McycleDelay::new(&rcu.clocks);
    let (mut red, mut green, mut blue) = rgb(gpioc.pc13, gpioa.pa1, gpioa.pa2);
    let leds: [&mut dyn Led; 3] = [&mut red, &mut green, &mut blue];
    leds[0].off();
    leds[1].off();
    leds[2].off(); 
    /*******************************************************************************************************************/
    let mut start_flag:bool = false;
    let mut data_flag:bool = false;

    let tx = gpioa.pa9.into_alternate_push_pull();
    let rx = gpioa.pa10.into_floating_input();

    let config = Config {
        baudrate: 115_200.bps(),
        parity: Parity::ParityNone,
        stopbits: StopBits::STOP1, 
    };

    let serial = Serial::new(usart0,(tx,rx),config,&mut afio,&mut rcu);
    let (mut tx, mut rx) = serial.split();

    let s: String<5> = String::from("Hello");
    let mut display_message: String<30> = String::from("UART Message: ");
    _=display_message.push_str(s.as_str());

    let b = s.into_bytes();
    let mut vec = Vec::<u8, 5>::new();

    /*******************************************************************************************************************/
    // Clear screen
    Rectangle::new(Point::new(0, 0), Size::new(width as u32, height as u32))
        .into_styled(PrimitiveStyle::with_fill(Rgb565::BLACK))
        .draw(&mut lcd)
        .unwrap();

    let style = MonoTextStyleBuilder::new()
        .font(&FONT_7X13_BOLD)
        .text_color(Rgb565::BLACK)
        .background_color(Rgb565::WHITE)
        .build();

    // Create a text at position (20, 30) and draw it using style defined above
    Text::new("Challenge 3", Point::new(40, 35), style)
        .draw(&mut lcd)
        .unwrap();
    delay.delay_ms(2*delay_value);

    // Clear screen
    Rectangle::new(Point::new(0, 0), Size::new(width as u32, height as u32))
    .into_styled(PrimitiveStyle::with_fill(Rgb565::BLACK))
    .draw(&mut lcd)
    .unwrap();
       
    /*******************************************************************************************************************/
    ECLIC::reset();
    ECLIC::set_threshold_level(Level::L0);
    ECLIC::set_level_priority_bits(LevelPriorityBits::L3P1);

    // Timer
    let mut timer =  Timer::timer1(p.TIMER1, 1.hz(), &mut rcu);
    timer.listen(timer::Event::Update);
    unsafe {G_TIMER1 = Some(timer)};

    ECLIC::setup(
        Interrupt::TIMER1,
        TriggerType::Level,
        Level::L1,
        Priority::P1,
    );
    unsafe { 
        ECLIC::unmask(Interrupt::TIMER1);
        riscv::interrupt::enable();
    };
    
    loop 
    {
        unsafe
        {
            if INTERRUPT_FLAG == true
            {
                INTERRUPT_FLAG = false; 
                start_flag = true; 
            }
        }
        if start_flag == true
        {
            // Write to the USART
            for i in 0..b.len() {
                block!(tx.write(b[i])).ok();
                _=vec.push(block!(rx.read()).unwrap());
            }
            
            for i in 0..b.len() 
            {
                if vec[i] == b[i] {
                    data_flag = true;
                }
                else {
                    // Create a text at position (20, 30) and draw it using style defined above
                    Text::new("No response from UART ", Point::new(1, 35), style)
                    .draw(&mut lcd)
                    .unwrap();
                    leds[0].on();
                    delay.delay_ms(delay_value);
                    start_flag = false;
                    data_flag = false;
                    break;
                }

            }
            if data_flag == true
            {
                start_flag = false;
                data_flag = false;
                leds[1].on();
                // Create a text at position (20, 30) and draw it using style defined above
                Text::new(display_message.as_str(), Point::new(10,35 ), style)
                .draw(&mut lcd)
                .unwrap();
                delay.delay_ms(delay_value);
                display_message.clear();
            }
        }
        else 
        {
            leds[0].off();
            leds[1].off();
            unsafe { asm::wfi();}  
        }
    }
}

#[allow(non_snake_case)]
#[no_mangle]
fn TIMER1() {
    unsafe {
        if let Some(timer1) = G_TIMER1.as_mut() {
            timer1.clear_update_interrupt_flag();
            INTERRUPT_FLAG = true;
        }
    }
}