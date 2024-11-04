/*!
The main contract is,

1.  It starts in 'suspended' state
2.  When it is signaled, it wakes up anyone parked
3.  When it is parked, it waits for a signal. Spurious wakeups are ok.
4.  At the moment, send side needs to be 'Send' due to Waker requirements.
*/

use std::sync::{Arc, Condvar, Mutex, MutexGuard};

struct Shared {
    condvar: Condvar,
    mutex: Mutex<bool>,
}

pub(crate) struct Sender {
}

impl Sender {
    pub fn signal(&self) {
        todo!()
    }
}

pub(crate) struct Receiver {
    shared: Arc<Shared>,
}

#[must_use]
pub (crate) struct Transaction<'a> {
    shared: Arc<Shared>,
    guard: MutexGuard<'a, bool>,
}

pub (crate) struct Parker {
    shared: Arc<Shared>,
}


impl Receiver {
    /**
    Begins a transaction.  Events that happen logically after this point, will result in a later park operation to waking.

    Stated alternatively, calling this method marks the point in time where the current thread is willing to park, and signals that occur after this point
    may happen "later".
    */
    pub fn will_park(&mut self) -> Transaction {
        let mut guard = self.shared.mutex.lock().unwrap();
        *guard = false; //suspend
        Transaction {
            shared: self.shared.clone(),
            guard,
        }
    }

}

impl Parker {
    pub fn park(mut self) {
        let mut guard = self.shared.mutex.lock().unwrap();
        while !*guard {
            guard = self.shared.condvar.wait(guard).unwrap();
        }
    }

    pub(crate) fn finalize_transaction(transaction: Transaction) -> Self {
        Parker {
            shared: transaction.shared,
        }
    }
}


pub(crate) fn blocking_continuation_type() -> (Sender, Receiver) {
    let shared = Arc::new(Shared {
        condvar: Condvar::new(),
        mutex: Mutex::new(false),
    });
    (Sender {}, Receiver {shared})
}