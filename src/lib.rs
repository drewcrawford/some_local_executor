/*!
It's a simple single-threaded executor.
*/

use std::future::Future;
use std::marker::PhantomData;

struct Task<'a> {
    future: Box<dyn Future<Output=()> + 'a>
}

impl<'a> Task<'a> {
    fn new<F>(future: F) -> Self
    where
        F: Future<Output = ()> + 'a
    {
        Task {
            future: Box::new(future)
        }
    }
}

pub struct Executor<'tasks> {
    tasks: Vec<Task<'tasks>>,
    //todo: remove
    data: PhantomData<&'tasks ()>
}

impl<'tasks> Executor<'tasks> {
    pub fn new() -> Self {
        Executor {
            tasks: Vec::new(),
            data: PhantomData
        }
    }
    /**
    Spawns the task onto the executor.
    */
    pub fn spawn(&mut self, future: impl Future<Output = ()> + 'tasks) {
        self.tasks.push(Task::new(future));
    }
    /**
    Runs the executor if there is work to be performed.
    */
    pub fn do_some(&mut self) {
        todo!()
    }
    /**
    Drains the executor. After this call, the executor can no longer be used.

    This function will return when all spawned tasks complete.
*/
    pub fn drain(self) {
        todo!()
    }
}

