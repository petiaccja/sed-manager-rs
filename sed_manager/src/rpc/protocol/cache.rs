use std::collections::VecDeque as Queue;

use crate::messaging::packet::Packet;

pub struct Cache {
    sn_to_assign: u32,
    sn_to_send: u32,
    queue: Queue<Packet>,
}

impl Cache {
    pub fn new() -> Self {
        Self { sn_to_assign: 1, sn_to_send: 1, queue: Queue::new() }
    }

    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    pub fn enqueue(&mut self, packet: Packet) -> u32 {
        let sequence_number = self.sn_to_assign;
        let packet = Packet { sequence_number, ..packet };
        self.queue.push_back(packet);
        self.sn_to_assign += 1;
        sequence_number
    }

    pub fn ack(&mut self, sequence_number: u32) {
        while self.queue.front().is_some_and(|packet| packet.sequence_number <= sequence_number) {
            self.queue.pop_front();
        }
        self.sn_to_send = std::cmp::max(self.sn_to_send, self.front_sequence_number());
    }

    pub fn next(&mut self) -> Option<Packet> {
        if let Some(packet) = self.find(self.sn_to_send).cloned() {
            self.sn_to_send += 1;
            Some(packet)
        } else {
            None
        }
    }

    pub fn rewind(&mut self) {
        self.sn_to_send = self.front_sequence_number();
    }

    pub fn front(&self) -> Option<&Packet> {
        self.queue.front()
    }

    pub fn back(&self) -> Option<&Packet> {
        self.queue.back()
    }

    pub fn front_sequence_number(&self) -> u32 {
        self.queue.front().map(|packet| packet.sequence_number).unwrap_or(self.sn_to_assign)
    }

    pub fn back_sequence_number(&self) -> u32 {
        self.queue.back().map(|packet| packet.sequence_number).unwrap_or(self.sn_to_assign)
    }

    pub fn sequence_numbers(&self) -> std::ops::RangeInclusive<u32> {
        self.front_sequence_number()..=self.back_sequence_number()
    }

    fn find(&self, sequence_number: u32) -> Option<&Packet> {
        let range = self.sequence_numbers();
        if range.contains(&sequence_number) {
            Some(&self.queue[(sequence_number - range.start()) as usize])
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_empty_empty() {
        let cache = Cache::new();
        assert!(cache.is_empty());
    }

    #[test]
    fn is_empty_filled() {
        let mut cache = Cache::new();
        cache.enqueue(Packet::default());
        assert!(!cache.is_empty());
    }

    #[test]
    fn enqueue_ordering() {
        let mut cache = Cache::new();
        for _ in 1..=4 {
            cache.enqueue(Packet::default());
        }
        let sequence_numbers: Vec<_> = cache.queue.iter().map(|packet| packet.sequence_number).collect();
        assert_eq!(sequence_numbers, vec![1, 2, 3, 4]);
        assert_eq!(cache.sn_to_assign, 5);
        assert_eq!(cache.sn_to_send, 1);
    }

    #[test]
    fn ack_underflow() {
        let mut cache = Cache::new();
        for _ in 1..=4 {
            cache.enqueue(Packet::default());
        }
        cache.ack(0);
        let sequence_numbers: Vec<_> = cache.queue.iter().map(|packet| packet.sequence_number).collect();
        assert_eq!(sequence_numbers, vec![1, 2, 3, 4]);
        assert_eq!(cache.sn_to_assign, 5);
        assert_eq!(cache.sn_to_send, 1);
    }

    #[test]
    fn ack_normal() {
        let mut cache = Cache::new();
        for _ in 1..=4 {
            cache.enqueue(Packet::default());
        }
        cache.ack(2);
        let sequence_numbers: Vec<_> = cache.queue.iter().map(|packet| packet.sequence_number).collect();
        assert_eq!(sequence_numbers, vec![3, 4]);
        assert_eq!(cache.sn_to_assign, 5);
        assert_eq!(cache.sn_to_send, 3);
    }

    #[test]
    fn ack_overflow() {
        let mut cache = Cache::new();
        for _ in 1..=4 {
            cache.enqueue(Packet::default());
        }
        cache.ack(100);
        let sequence_numbers: Vec<_> = cache.queue.iter().map(|packet| packet.sequence_number).collect();
        assert_eq!(sequence_numbers, vec![]);
        assert_eq!(cache.sn_to_assign, 5);
        assert_eq!(cache.sn_to_send, 5);
    }

    #[test]
    fn next_advance() {
        let mut cache = Cache::new();
        for _ in 1..=4 {
            cache.enqueue(Packet::default());
        }
        assert_eq!(cache.next().unwrap().sequence_number, 1);
        assert_eq!(cache.sn_to_assign, 5);
        assert_eq!(cache.sn_to_send, 2);
    }

    #[test]
    fn next_final() {
        let mut cache = Cache::new();
        for _ in 1..=4 {
            cache.enqueue(Packet::default());
        }
        cache.sn_to_send = 4;
        assert_eq!(cache.next().unwrap().sequence_number, 4);
        assert_eq!(cache.sn_to_assign, 5);
        assert_eq!(cache.sn_to_send, 5);
        assert_eq!(cache.next(), None);
    }

    #[test]
    fn rewind() {
        let mut cache = Cache::new();
        cache.sn_to_assign = 3;
        cache.sn_to_send = 3;
        for _ in 1..=4 {
            cache.enqueue(Packet::default());
        }
        cache.sn_to_send = 6;
        cache.rewind();
        assert_eq!(cache.sn_to_assign, 7);
        assert_eq!(cache.sn_to_send, 3);
    }

    #[test]
    fn sequence_numbers() {
        let mut cache = Cache::new();
        for _ in 1..=4 {
            cache.enqueue(Packet::default());
        }
        assert_eq!(cache.sequence_numbers(), 1..=4);
    }
}
