use dxl_packet::stream::Stream;

pub struct RxStream {}

impl Stream for RxStream {
    type Item = u8;
}
