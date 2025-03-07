use dxl_packet::stream::Stream;

#[expect(async_fn_in_trait, reason = "fuck off")]
pub trait Comm {
    type SendError;
    type RecvError;
    async fn comm(
        &mut self,
        buffer: &[u8],
    ) -> Result<impl Stream<Item = Result<u8, Self::RecvError>> + 'static, Self::SendError>;
}
