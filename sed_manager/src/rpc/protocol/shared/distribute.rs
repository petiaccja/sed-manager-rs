use core::task::Poll;
use std::collections::BTreeMap;

use super::pipe::{SinkPipe, SourcePipe};

pub fn distribute<Input, Key: Ord + Eq, Value>(
    input: &mut dyn SourcePipe<Input>,
    outputs: &mut BTreeMap<Key, &mut dyn SinkPipe<Value>>,
    key: impl Fn(&Input) -> Key,
    value: impl Fn(Input) -> Value,
) {
    while let Poll::Ready(Some(item)) = input.pop() {
        if let Some(output) = outputs.get_mut(&key(&item)) {
            (*output).push(value(item));
        }
    }
    if input.is_done() {
        for output in outputs.values_mut() {
            (*output).close();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use super::super::buffer::Buffer;

    fn parity(number: &i32) -> bool {
        number % 2 == 0
    }

    fn output_map<'a>(
        even: &'a mut dyn SinkPipe<i32>,
        odd: &'a mut dyn SinkPipe<i32>,
    ) -> BTreeMap<bool, &'a mut dyn SinkPipe<i32>> {
        [(true, even), (false, odd)].into_iter().collect()
    }

    fn setup() -> (Buffer<i32>, Buffer<i32>, Buffer<i32>) {
        let input = Buffer::<i32>::new();
        let even = Buffer::<i32>::new();
        let odd = Buffer::<i32>::new();
        (input, even, odd)
    }

    #[test]
    fn with_data() {
        let (mut source, mut even, mut odd) = setup();
        source.push(1);
        source.push(2);
        source.push(3);
        source.push(4);
        source.push(6);
        distribute(&mut source, &mut output_map(&mut even, &mut odd), parity, |x| x);
        let mut evens = Vec::new();
        let mut odds = Vec::new();
        while let Poll::Ready(Some(number)) = even.pop() {
            evens.push(number);
        }
        while let Poll::Ready(Some(number)) = odd.pop() {
            odds.push(number);
        }
        assert_eq!(evens, vec![2, 4, 6]);
        assert_eq!(odds, vec![1, 3]);
    }

    #[test]
    fn with_no_data() {
        let (mut source, mut even, mut odd) = setup();
        distribute(&mut source, &mut output_map(&mut even, &mut odd), parity, |x| x);
        assert_eq!(Poll::Pending, even.pop());
        assert_eq!(Poll::Pending, odd.pop());
    }

    #[test]
    fn with_closed() {
        let (mut source, mut even, mut odd) = setup();
        source.close();
        distribute(&mut source, &mut output_map(&mut even, &mut odd), parity, |x| x);
        assert_eq!(Poll::Ready(None), even.pop());
        assert_eq!(Poll::Ready(None), odd.pop());
    }
}
