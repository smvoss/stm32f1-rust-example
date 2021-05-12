/* Information from stm32f107 datasheet, section 4's memory map (figure 5) */
MEMORY
{
    FLASH : ORIGIN = 0x08000000, LENGTH = 256K
    RAM : ORIGIN = 0x20000000, LENGTH = 64K
}
