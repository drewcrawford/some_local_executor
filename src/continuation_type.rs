/*!
The main contract is,

1.  It starts in 'suspended' state
2.  When it is signaled, it wakes up anyone parked
3.  When it is parked, it waits for a signal
4.  At the moment, send side needs to be 'Send' due to Waker requirements.
*/

pub struct Sender {

}

impl Sender {
    pub fn signal(&self) {
        todo!()
    }
}

pub struct Receiver {

}

impl Receiver {
    pub fn park(&self) {
        todo!()
    }
}


pub fn blocking_continuation_type() -> (Sender, Receiver) {
    (Sender {}, Receiver {})
}