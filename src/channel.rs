/*!
A channel type organized around Waker-style semantics.

Requirements are:

1.  Multiple producer, single consumer
2.  Value known type, known to fit into u64.
3.  Value locked at the time Sender is created, and cannot be changed.
3a. So effectively the channel is bounded by the number of Senders.
4.  The message can be sent multiple times, which "shouldn't" block, for some loose definition of shouldn't.
5.  The consumer must be able to block until a message is available from any outstanding producer
6.  Sender must be send/sync.
7.  Also need an operation to attach a "new" sender with a different value to the channel.
 */

use std::sync::atomic::AtomicU64;
use some_executor::task::TaskID;

pub struct Sender{
    task_id: TaskID,
}

impl Sender {
    pub fn send_by_ref(&self) {
        todo!()
    }

    pub fn send(self) {
        todo!()
    }

    pub fn with_receiver(receiver: &mut Receiver) -> Self {
        todo!()
    }
}

pub struct Receiver {

}

impl Receiver {
    pub fn recv_park(&self) -> TaskID {
        todo!()
    }
    pub fn new() -> Self {
        todo!()
    }
}

