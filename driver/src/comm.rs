use dxl_packet::stream::Stream;

#[expect(async_fn_in_trait, reason = "fuck off")]
pub trait Comm {
    type SendError: defmt::Format;
    type RecvError: defmt::Format;
    async fn comm<'rx>(
        &'rx mut self,
        buffer: &[u8],
    ) -> Result<impl 'rx + Stream<Item = Result<u8, Self::RecvError>>, Self::SendError>;
    async fn yield_to_other_tasks();
}
