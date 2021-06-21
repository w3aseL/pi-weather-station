use rppal::spi::{ Spi, Bus, SlaveSelect, Mode, BitOrder };

#[allow(dead_code)]
pub struct MCP3008 {
    bus: Bus,
    slave_select: SlaveSelect,
    clock_speed: u32,
    mode: Mode,
    spi: Spi
}

impl MCP3008 {
    pub fn new(bus: Bus, slave_select: SlaveSelect, clock_speed: u32, mode: Mode) -> Self {
        let spi = Spi::new(bus, slave_select, clock_speed, mode).unwrap();
        spi.set_bit_order(BitOrder::MsbFirst).unwrap();

        Self {
            bus,
            slave_select,
            clock_speed,
            mode,
            spi: spi
        }
    }

    pub fn read_from_channel(&mut self, channel: u8, buf: &mut [u8]) -> usize {
        let mut command_buf = [0x01u8, 0x80u8, 0u8];
        command_buf[1] |= channel << 5;

        self.spi.transfer(buf, &command_buf).unwrap()
    }
}