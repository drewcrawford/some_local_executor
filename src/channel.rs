/*!
A channel type organized around Waker-style semantics.

Requirements are:

1.  Multiple producer, single consumer
2.  However each producer can only really send one message.  Sending multiple messages is ok, but the consumer will only see the first one, and the rest will be dropped.
3.  The consumer must be able to block until a message is available from any producer
4.  Messages queue without limit.  If the consumer is not ready to receive a message, the producer must be able to send it anyway.
 */

use some_executor::task::TaskID;

#[derive(Clone)]
pub struct Sender(std::sync::mpsc::Sender<TaskID>);

pub struct Receiver(std::sync::mpsc::Receiver<TaskID>);

impl Receiver {
    pub fn recv_park(&self) -> TaskID {
        self.0.recv().expect("Channel was hung up")
    }
}
pub fn channel() -> (Sender, Receiver) {
    let (tx, rx) = std::sync::mpsc::channel();
    (Sender(tx), Receiver(rx))
}

