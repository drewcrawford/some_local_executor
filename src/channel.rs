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
use std::sync::{Arc, Condvar, Mutex, Weak};
use some_executor::task::TaskID;

//taskid is known not to use this value
const BLANK_SLOT: u64 = u64::MAX;

pub struct Sender{
    task_id: TaskID,
    slot: Arc<AtomicU64>,
    shared: Arc<Shared>,
}

struct Shared {
    lock: Mutex<bool>,
    condvar: Condvar,
}

impl Sender {
    pub fn send_by_ref(&self) {
        self.slot.store(self.task_id.into(), std::sync::atomic::Ordering::Relaxed);
        *self.shared.lock.lock().unwrap() = true;
        self.shared.condvar.notify_one();
    }

    pub fn send(self) {
        //maybe this could be optimized?
        self.send_by_ref()
    }

    pub fn with_receiver(receiver: &mut Receiver, task_id: TaskID) -> Self {
        let new_slot = Arc::new(AtomicU64::new(task_id.into()));
        let new_slot_recv = Arc::downgrade(&new_slot);
        receiver.slots.push(new_slot_recv);
        Sender {
            task_id,
            slot: new_slot,
            shared: receiver.shared.clone(),
        }
    }
}

pub struct Receiver {
    shared: Arc<Shared>,
    //holds one slot per open sender.
    //The main idea is we update the value, and then we flag the mutex, then we signal the condvar.
    slots: Vec<Weak<AtomicU64>>,

}

impl Receiver {

    fn find_slot(slots: &mut Vec<Weak<AtomicU64>>) -> Option<u64> {
        //GC the slots
        slots.retain(|slot| slot.upgrade().is_some());

        for slot in slots.iter() {
            match slot.upgrade() {
                None => {
                    //not overly concerned about it - we will GC it later
                }
                Some(arc) => {
                    let read = arc.load(std::sync::atomic::Ordering::Relaxed);
                    if read != BLANK_SLOT {
                        //make the slot blank again, so that the message can be resent.
                        arc.store(BLANK_SLOT, std::sync::atomic::Ordering::Relaxed);
                        return Some(read);
                    }
                }
            }
        }
        None
    }
    pub fn recv_park(&mut self) -> TaskID {
        //grab guard to guarantee we are exclusive user
        let mut guard = self.shared.lock.lock().unwrap();
        while !*guard {
            guard = self.shared.condvar.wait(guard).unwrap();
        }
        let found_slot = Self::find_slot(&mut self.slots).expect("No slot seems to be posted");
        *guard = false;
        found_slot.into()
    }
    pub fn new() -> Self {
        Receiver {
            shared: Arc::new(Shared {
                lock: Mutex::new(false),
                condvar: Condvar::new(),
            }),
            slots: Vec::new(),
        }
    }
}

