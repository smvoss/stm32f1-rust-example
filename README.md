# Simple POC for the Micrium STM32F107 Evaluation Board

## Motivation

My motivation for this project was very simple: I wanted to blink some LEDs at "some rate(tm)" using entirely rust. I
also did not want to use any of the embedded-std libs that were available, because I wanted to really get a feel for
bit-banging the registers (including documenting where I grabbed the info from).

## Where to start?

Defining *literally* everything myself is a bit unreasonable, especially when awesome projects like
[stm32-rs](https://github.com/stm32-rs/stm32-rs) exist. This project use a community-built collection
of patches to the basic SVD files, to build a *peripheral access crate*. This gives us a foundation
of the registers and fields available on our given chip. The PAC leverages [svd2rust](https://github.com/rust-embedded/svd2rust/)
which provides a great API for safely using the PAC.

## Getting Started

### Toolchain Setup

An before we can do anything else, we need to make sure we have a target for our chip as part of
our toolchain to support cross-compiling. To identify the toolchain I first checked which ARM family
the `STM32F107` is part of, which is `Cortex-M3`. I then deferred to the [Cortex M Quickstart](https://docs.rust-embedded.org/cortex-m-quickstart/cortex_m_quickstart/#usage)
documention for quick lookup table which showed that I wanted the `thumbv7m-none-eabi` target. There
are certanly more ways to establish this, but it was already written down here so I stopped looking.

```
$ rustup target add thumbv7m-none-eabi
```

At this point, you can verify that your target was installed with rustup by using `rustup show`:

```
installed targets for active toolchain
--------------------------------------

aarch64-unknown-linux-gnu
thumbv7m-none-eabi
x86_64-unknown-linux-gnu
```

You may not have all of the toolchains, but make sure the one you wished to install is now available.
This gives you a starting point to be able to compile an `ELF` for your target architecture, however
a microcontroller will need a standard binary. This is easily accomplished with `objcopy` which we can
install cross versions of simply using some great utilities:

```
$ cargo install cargo-binutils
$ rustup component add llvm-tools-preview
```

This will give us the environment we need to get started. I'm not going to explain these utilities
in depth here, but I would spend some time looking into them if you're curious.

### The Nitty-Gritty

For this code, we won't have any standard-library which means we're going to have to roll our own
versions of the utility functions we're used to having (and in this example, specifically `delay_us`).

In our main file, we'll have to define multiple items to make it possbile for us to run on our embedded target:

* `#![no_std]`
 * std libs are heavy, and [our platform doesn't provide them](https://doc.rust-lang.org/nightly/rustc/platform-support.html)
* `#![panic_handler]`
 * This is a relic of not having a standard lib, and we must define how we handle panics
 * In this example, I do nothing. Hopefully we don't panic!
* `#![no_main]`
 * Due to this, we also need to mark our `main()` as `#[no_mangle]` so the entrypoint can still be found

As you can tell, it very quickly goes into the weeds that embedded usually does (not that we're particularily surprised).

### Building our application

The target application, as said before is just to **blink an LED at some rate on this development kit I have access to**. Easy enough.

On the `Cortex-M3`, all of the peripheral clocks are disabled by default and they must be turned on, which means for our application
we must identify which timer, and which GPIO we want to use so we can adiquately turn them on.

From the devkit's user manual<sup>1</sup> one of the LEDs is on `PD13` which means we must turn on the clock for bank D. While we're here,
it makes sense to pick a timer to use. I arbirarily picked `TIM2` which could have been anything.

Starting at the processor datasheet<sup>2</sup>, I checked which Advanced Peripheral Bus (APB) the items I want are on (Block diagram in
section 2.3). Bank D for the GPIO is on `APB2` and `TIM2` is on `APB1`. At this point, it's time to move into the reference manual<sup>3</sup>
for register defintion.

#### Identifying required registers

This is an extremely tedious part if you don't know what you're looking for, so I'm going to cut to the chase on most of it. A lot of
learning how to do this is going to come from reading circles in datasheets and reference manuals, but this should give you a reasonable
idea of what you're looking for. All of this is done in the reference manual<sup>3</sup> for the
chip family.

Since we know we need to enable the peripheral clocks, we'll jump straight to the Reset and Clock Control (RCC) s  ectioi  a 7.3. From there,
we will jump to the registers which actually enable the clocks for our APB's: `RCC_APB1ENR` and `RCC_APB2ENR`. Looking at the registers
we see taht `TIM2` is bit 0 and `IOPDEN` is bit 5 of their respective registers. Since we are using the PAC crate the specifics of "which bit
do we need to set" matters a lot less, but while we're here it makes sense to make sure there are no side-effects or anything else we need
to do.

Because there isn't, we can very easily (using the PAC) enable this:

```rust
// identify TIM2 is on the APB1 from the 107 RM S2.3 (p13)
peripherals.RCC.apb1enr.write(|w| w.tim2en().bit(true));

// enable the clock for IO port D (as our LED is on GPIO D-13)
peripherals.RCC.apb2enr.write(|w| w.iopden().bit(true));
```

Note: I didn't describe setting up the `stm32-rs` library etc, that is better explained in their documentation.

Jumping ahead a bit, we'll also need to setup our GPIO to be an output, and finally drive it so we can turn our LED on. This can be found
in section 9, specifically 9.2 for the register map. We first register we see is `GPIOx_CRL` - if you haven't seen this syntax before it
can be a bit confusing, but the point is "this register map repeats itself for each bank (in this case A-E)". In the chip datasheet<sup>2</sup>
you can check the memory-map to see exactly where the "base address" of each bank is- but we won't need that here due to the PAC.

The first register gives us the mode and configuration bits for GPIO's 0-7, and since we need 13 we'll move onto the next register, `GPIOx_CRH`.
Here we see the fields we need, `MODE13[1:0]` and `CNF13[1:0]`. Using the information given below the table, knowing we want to drive an LED, we
will use output mode and general-purpose output push-pull. If we were doing this without the PAC, we would need to know exactly which bits to set
and where to set them but instead we can textually describe it:

```rust
// LED is on GPIO D13, set it to a push-pull output
peripherals.GPIOD.crh.write(|w| {
    w.mode13().output();
    w.cnf13().push_pull()
});
```

At this point, hopefully it's more obvious what you're looking for, but at the end of the day there is no way around it: it's a lot of reading.

### Compiling

On an embedded system, we need to describe a bit of extra information for the linker to be able to actually put together our binary. We will
define a `memory.x` file which will define the locations of our flash (memory) and our ram on the chip. Going back into our trusty datasheet<sup>2</sup>
we instead seek to the memory map this time (which can be found in section 4) and look for the two fields we need which will give us the start
and end addresses. For my chip, I find the following information:

```
MEMORY
{
    FLASH : ORIGIN = 0x08000000, LENGTH = 256K
    RAM : ORIGIN = 0x20000000, LENGTH = 64K
}
```

At this point, for convenience we'll create a file `.cargo/config` which will contain the following:

```
[target.thumbv7m-none-eabi]
runner = 'gdb-multiarch'
rustflags = [ 
  "-C", "link-arg=-Tlink.x",
]

[build]
target = "thumbv7m-none-eabi"
```

As you can easily recognize from earlier, this gives cargo extra information on our toolchain, including our new files. The `[build]` section's
`target` allows us to avoid specifying `--target thumbv7m-none-eabi` every time we invoke `cargo`, but is technically optional although a nice to have.

At this point, after doing a build (if everything went well), we can look in the `target/thumbv7m-none-eabi/release/` directory and find our binary! Unfortunately,
as I alluded to during the toolchain setup, this gave us an `ELF` file and we need a binary. Lucky for us, we're ready for that!

We can use the combination of our `llvm-tools-preview` and `cargo-binutils` to get a cross-ready `objcopy`, neatly aliased through `cargo`.

```
cargo objcopy --release -- -O binary target/thumbv7m-none-eabi/release/stm32f107.bin
```

This will do a build, and then do an `objcopy` to the specified location. The name is arbitrary, and I just didn't name it very creatively.

## Flashing your binary

At this point, it becomes extremely setup-dependent, so I won't go in too far, however a few tips if you have a Segger J-LINK:

1. You'll want to install the `JLinkExe` application (yes, it's named that on linux too)
2. When you run the application, make sure your debugger is already plugged in, it'll autodetect it over usb
3. Figure out how it's attached to your processor. On my devkit it was via SWD and not JTAG.
4. Connect to your CPU with the `connect` command (it'll give you an interactive prompt looking for more information to do this)
5. Use the `loadbin` command to flash your binary, the `r` command to reset the chip, and the `go` command to start your processor again!

At this point you may want other generally useful debugging commands like `mem32` to peek at registers, `w4` to write to them, and so on. This
can be a very powerful debugging tool to make sure that you're actually doing what you think you are. For example, if you forget to setup the
peripheral clocks, a write to the GPIO bank will just "not work". A hard but fair reminder to turn them on.

Good luck!

## Resources

1. [Devkit PDF](https://www.element14.com/community/servlet/JiveServlet/previewBody/55737-102-1-276843/STMicroelectronics.User_Manual_1.pdf)
2. [STM32F107 Datasheet](https://www.st.com/resource/en/datasheet/stm32f107vc.pdf)
3. [STM32F107XX Reference Manual](https://www.st.com/resource/en/reference_manual/cd00171190-stm32f101xx-stm32f102xx-stm32f103xx-stm32f105xx-and-stm32f107xx-advanced-armbased-32bit-mcus-stmicroelectronics.pdf)
