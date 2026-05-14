//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use core::task::Poll;

use super::pipe::{SinkPipe, SourcePipe};

pub fn gather<Item>(inputs: &mut [&mut dyn SourcePipe<Item>], output: &mut dyn SinkPipe<Item>) {
    for input in inputs.iter_mut() {
        while let Poll::Ready(Some(item)) = (*input).pop() {
            output.push(item);
        }
    }
    if !inputs.is_empty() && inputs.iter().all(|input| input.is_done()) {
        output.close();
    }
}

#[cfg(test)]
mod tests {
    use crate::rpc::protocol::shared::buffer::Buffer;

    use super::*;

    fn setup() -> (Vec<Buffer<i32>>, Buffer<i32>) {
        let sources = vec![Buffer::<i32>::new(), Buffer::<i32>::new()];
        let sink = Buffer::<i32>::new();
        (sources, sink)
    }

    fn input_slice(inputs: &mut Vec<Buffer<i32>>) -> Vec<&mut dyn SourcePipe<i32>> {
        inputs.iter_mut().map(|b| b as &mut dyn SourcePipe<i32>).collect()
    }

    #[test]
    fn with_data() {
        let (mut sources, mut sink) = setup();
        sources[0].push(4);
        sources[1].push(5);
        sources[0].push(6);
        gather(&mut input_slice(&mut sources), &mut sink);
        assert_eq!(sink.pop(), Poll::Ready(Some(4)));
        assert_eq!(sink.pop(), Poll::Ready(Some(6)));
        assert_eq!(sink.pop(), Poll::Ready(Some(5)));
    }

    #[test]
    fn with_no_data() {
        let (mut sources, mut sink) = setup();
        gather(&mut input_slice(&mut sources), &mut sink);
        assert_eq!(sink.pop(), Poll::Pending);
    }

    #[test]
    fn with_closed() {
        let (mut sources, mut sink) = setup();
        for source in &mut sources {
            source.close();
        }
        gather(&mut input_slice(&mut sources), &mut sink);
        assert_eq!(sink.pop(), Poll::Ready(None));
    }
}
