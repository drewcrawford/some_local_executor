//SPDX-License-Identifier: MIT OR Apache-2.0

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

//We borrow the high values to represent our own internal state.
//TASK_ID is known not to use MAX, it can use the others.  But regardless
//we check at runtime.
const BLANK_SLOT: u64 = u64::MAX;
const NEW_TASK_SUBMITTED: u64 = u64::MAX - 1;
const INVALID_TASK_ID: u64 = NEW_TASK_SUBMITTED;

#[derive(Debug)]
pub struct Sender{
    task_id: u64,
    slot: Arc<AtomicU64>,
    shared: Arc<Shared>,
}

#[derive(Debug)]
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

    pub fn with_receiver_for_task_submit(receiver: &mut Receiver) -> Self {
        let new_slot = Arc::new(AtomicU64::new(BLANK_SLOT));
        let new_slot_recv = Arc::downgrade(&new_slot);
        receiver.slots.push(new_slot_recv);
        Sender {
            task_id: NEW_TASK_SUBMITTED,
            slot: new_slot,
            shared: receiver.shared.clone(),
        }
    }

    pub fn with_receiver(receiver: &mut Receiver, task_id: TaskID) -> Self {
        let new_slot = Arc::new(AtomicU64::new(task_id.into()));
        let new_slot_recv = Arc::downgrade(&new_slot);
        assert!(task_id.as_ref() < &INVALID_TASK_ID, "TaskID is too large");
        receiver.slots.push(new_slot_recv);
        Sender {
            task_id: task_id.into(),
            slot: new_slot,
            shared: receiver.shared.clone(),
        }
    }
}

#[derive(Debug)]
pub struct Receiver {
    shared: Arc<Shared>,
    //holds one slot per open sender.
    //The main idea is we update the value, and then we flag the mutex, then we signal the condvar.
    slots: Vec<Weak<AtomicU64>>,

}

pub enum FindSlot {
    FoundSlot(TaskID),
    FoundTaskSubmitted,
    NoSlot,
}

impl Receiver {

    fn find_slot(slots: &mut Vec<Weak<AtomicU64>>) -> FindSlot {
        //GC the slots
        slots.retain(|slot| slot.upgrade().is_some());

        for slot in slots.iter() {
            match slot.upgrade() {
                None => {
                    //not overly concerned about it - we will GC it later
                }
                Some(arc) => {
                    let read = arc.load(std::sync::atomic::Ordering::Relaxed);
                    match read {
                        BLANK_SLOT => {
                            //ignore
                        }
                        NEW_TASK_SUBMITTED => {
                            //make the slot blank again, so that the message can be resent.
                            arc.store(BLANK_SLOT, std::sync::atomic::Ordering::Relaxed);
                            return FindSlot::FoundTaskSubmitted;
                        }
                        a if a < INVALID_TASK_ID => {
                            //make the slot blank again, so that the message can be resent.
                            arc.store(BLANK_SLOT, std::sync::atomic::Ordering::Relaxed);
                            return FindSlot::FoundSlot(TaskID::from(a));
                        }
                        _ => {
                            unreachable!("Invalid value in slot");
                        }
                    }
                }
            }
        }
        FindSlot::NoSlot
    }
    pub fn recv_park(&mut self) -> FindSlot {
        //grab guard to guarantee we are exclusive user
        let mut guard = self.shared.lock.lock().unwrap();
        while !*guard {
            guard = self.shared.condvar.wait(guard).unwrap();
        }
        let found_slot = Receiver::find_slot(&mut self.slots);
        *guard = false;
        found_slot
    }
    pub fn has_data(&self) -> bool {
        *self.shared.lock.lock().unwrap()
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

