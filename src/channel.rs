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
use std::sync::{Condvar, Mutex};
use some_executor::task::TaskID;

//taskid is known not to use this value
const BLANK_SLOT: u64 = u64::MAX;

pub struct Sender{
    task_id: TaskID,

}

impl Sender {
    pub fn send_by_ref(&self) {
        todo!()
    }

    pub fn send(self) {
        //maybe this could be optimized?
        self.send_by_ref()
    }

    pub fn with_receiver(receiver: &mut Receiver, task_id: TaskID) -> Self {
        Sender {
            task_id
        }
    }
}

pub struct Receiver {
    condvar: Condvar,
    mutex: Mutex<bool>,
    //holds one slot per open sender.
    //The main idea is we update the value, and then we flag the mutex, then we signal the condvar.
    slots: Vec<u64>,

}

impl Receiver {

    fn find_slot(slots: &mut Vec<u64>) -> Option<u64> {
        for slot in slots.iter_mut() {
            if *slot == BLANK_SLOT {
                let r = Some(*slot);
                *slot = BLANK_SLOT;
                return r;
            }
        }
        None
    }
    pub fn recv_park(&mut self) -> TaskID {
        //grab guard to guarantee we are exclusive user
        let mut guard = self.mutex.lock().unwrap();
        while !*guard {
            guard = self.condvar.wait(guard).unwrap();
        }
        let found_slot = Self::find_slot(&mut self.slots).expect("No slot seems to be posted");
        *guard = false;
        found_slot.into()
    }
    pub fn new() -> Self {
        Receiver {
            condvar: Condvar::new(),
            mutex: Mutex::new(false),
            slots: Vec::new(),
        }
    }
}

