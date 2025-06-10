use embassy_rp::gpio;

pub struct PullHigh<'high, 'pin> {
    pin: &'high mut gpio::Output<'pin>,
}

impl<'high, 'pin> PullHigh<'high, 'pin> {
    #[inline]
    pub fn new(pin: &'high mut gpio::Output<'pin>) -> Self {
        let () = pin.set_high();
        Self { pin }
    }
}

impl<'high, 'pin> Drop for PullHigh<'high, 'pin> {
    #[inline]
    fn drop(&mut self) {
        let () = self.pin.set_low();
    }
}
