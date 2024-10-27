/*!
It's a simple single-threaded executor.
*/

use std::future::Future;
use std::marker::PhantomData;
use std::pin::Pin;
use std::task::{Context, RawWaker, RawWakerVTable, Waker};

const VTABLE: RawWakerVTable = RawWakerVTable::new(
    |data| {
        todo!()
    },
    |data| {
        todo!()
    },
    |data| {
        todo!()
    },
    |data| {
        todo!()
    }
);


enum RunResult {
    ///The executor performed no work.
    DidNothing,
    ///The executor did some work, and more is available to do.
    DidSome,
    /// The executor did some work, and there is no more work to do at present.
    Done
}

struct Task<'a> {
    future: Pin<Box<dyn Future<Output=()> + 'a>>
}

impl<'a> Task<'a> {
    fn new<F>(future: F) -> Self
    where
        F: Future<Output = ()> + 'a
    {
        Task {
            future: Box::pin(future)
        }
    }
}

pub struct Executor<'tasks> {
    tasks: Vec<Task<'tasks>>,
    //todo: remove
    data: PhantomData<&'tasks ()>,
    waker: Waker,
}

impl<'tasks> Executor<'tasks> {
    pub fn new() -> Self {

        let waker = unsafe{Waker::from_raw(RawWaker::new(std::ptr::null(), &VTABLE))};
        Executor {
            tasks: Vec::new(),
            data: PhantomData,
            waker
        }
    }
    /**
    Spawns the task onto the executor.
    */
    pub fn spawn(&mut self, future: impl Future<Output = ()> + 'tasks) {
        self.tasks.push(Task::new(future));
    }
    /**
    Runs the executor a small amount if there is some work to be performed.

    */
    pub fn do_some(&mut self) -> RunResult {
        let mut context = Context::from_waker(&self.waker);
        if let Some(mut task) = self.tasks.pop() {
            let e = Pin::as_mut(&mut task.future).poll(&mut context);
            todo!("Add task back to the queue if it's not done");

        }
        else {
            RunResult::DidNothing
        }
    }
    /**
    Drains the executor. After this call, the executor can no longer be used.

    This function will return when all spawned tasks complete.
*/
    pub fn drain(self) {
        todo!()
    }
}

