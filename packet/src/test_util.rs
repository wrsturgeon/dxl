/*
use core::{pin::Pin, task};

pub(crate) fn trivial_future<F: Future>(pin: Pin<&mut F>) -> F::Output {
    match pin.poll(&mut const { task::Context::from_waker(task::Waker::noop()) }) {
        task::Poll::Pending => panic!("Input to `trivial_future` was not ready"),
        task::Poll::Ready(ready) => ready,
    }
}
*/
