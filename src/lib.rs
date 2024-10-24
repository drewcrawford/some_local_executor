/*!
It's a simple single-threaded executor.
*/

use std::future::Future;
use std::marker::PhantomData;

pub struct Executor<'tasks> {
    data: PhantomData<&'tasks ()>
}

impl<'tasks> Executor<'tasks> {
    pub fn new() -> Self {
        todo!()
    }
    /**
    Spawns the task onto the executor.
    */
    pub fn spawn(&mut self, future: impl Future<Output = ()> + 'tasks) {
        todo!()
    }
    /**
    Runs the executor if there is work to be performed.
    */
    pub fn do_some(&mut self) {

    }
    /**
    Drains the executor. After this call, the executor can no longer be used.

    This function will return when all spawned tasks complete.
*/
    pub fn drain(self) {
        todo!()
    }
}

