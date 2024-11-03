/*!
It's a simple single-threaded executor.
*/

use std::any::Any;
use std::future::Future;
use std::marker::PhantomData;
use std::pin::Pin;
use std::task::{Context, RawWaker, RawWakerVTable, Waker};
use some_executor::{LocalExecutorExt, SomeExecutor, SomeLocalExecutor};
use some_executor::observer::{ExecutorNotified, NoNotified, Observer, ObserverNotified};
use some_executor::task::{DynLocalSpawnedTask, SpawnedLocalTask, SpawnedTask, Task};

struct WakerInfo;


const VTABLE: RawWakerVTable = RawWakerVTable::new(
    |data| {
        //drop is also unimplemented
        todo!()

    },
    |data| {
        //drop is also unimplemented
        todo!()

    },
    |data| {
        //drop is also unimplemented
        todo!()
    },
    |data| {
        //hard to implement this one first
    }
);

#[derive(Debug,PartialEq)]
enum RunResult {
    ///The executor performed no work.
    DidNothing,
    ///The executor did some work, and more is available to do.
    DidSome,
    /// The executor did some work, and there is no more work to do at present.
    Done
}




pub struct Executor<'tasks> {
    tasks: Vec<Pin<Box<dyn DynLocalSpawnedTask<Self> + 'tasks>>>,
    //todo: remove
    data: PhantomData<&'tasks ()>,
    //option so we can take and untake
    waker: Option<Waker>,
}

impl<'tasks> Executor<'tasks> {
    pub fn new() -> Self {

        let waker = unsafe{Waker::from_raw(RawWaker::new(std::ptr::null(), &VTABLE))};
        Executor {
            tasks: Vec::new(),
            data: PhantomData,
            waker: Some(waker)
        }
    }
    /**
    Runs the executor a small amount if there is some work to be performed.

    */
    pub fn do_some(&mut self) -> RunResult {
        let attempt_tasks = self.tasks.drain(..).collect::<Vec<_>>();
        let move_waker = self.waker.take().expect("No waker");
        let mut context = Context::from_waker(&move_waker);

        let mut new_tasks = Vec::new();
        let mut did_some = false;
        let now = std::time::Instant::now();
        for mut task in attempt_tasks {
            if task.poll_after() > now {
                new_tasks.push(task);
                continue;
            }
            //otherwise...
            did_some = true;

            let e = task.as_mut().poll(&mut context, self);
            match e {
                std::task::Poll::Ready(_) => {
                    // do nothing
                }
                std::task::Poll::Pending => {
                    new_tasks.push(task);
                }
            }
        }

        self.tasks = new_tasks;
        self.waker = Some(move_waker);
        if self.tasks.is_empty() {
            if did_some {
                RunResult::Done
            }
            else {
                RunResult::DidNothing
            }
        }
        else {
            RunResult::DidSome
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

impl<'future> SomeLocalExecutor<'future> for Executor<'future> {
    type ExecutorNotifier = NoNotified;

    fn spawn_local<F: Future, Notifier: ObserverNotified<F::Output>>(&mut self, task: Task<F, Notifier>) -> Observer<F::Output, Self::ExecutorNotifier>
    where
        Self: Sized,
        F: 'future,
        <F as Future>::Output: Unpin,
    {
        let (task,observer) = task.spawn_local(self);
        self.tasks.push(Box::pin(task));
        observer
    }

    fn spawn_local_async<F: Future, Notifier: ObserverNotified<F::Output>>(&mut self, task: Task<F, Notifier>) -> impl Future<Output=Observer<F::Output, Self::ExecutorNotifier>>
    where
        Self: Sized,
        F: 'future,
    {
        async {
            let (spawn,observer)  = task.spawn_local(self);
            self.tasks.push(Box::pin(spawn));
            observer
        }
    }
    fn spawn_local_objsafe(&mut self, task: Task<Pin<Box<dyn Future<Output=Box<dyn Any>>>>, Box<dyn ObserverNotified<(dyn Any + 'static)>>>) -> Observer<Box<dyn Any>, Box<dyn ExecutorNotified>> {
        let (spawn, observer) = task.spawn_local_objsafe(self);
        self.tasks.push(Box::pin(spawn));
        observer
    }

    fn spawn_local_objsafe_async<'executor>(&'executor mut self, task: Task<Pin<Box<dyn Future<Output=Box<dyn Any>>>>, Box<dyn ObserverNotified<(dyn Any + 'static)>>>) -> Box<dyn Future<Output=Observer<Box<dyn Any>, Box<dyn ExecutorNotified>>> + 'executor> {
        Box::new(async {
            let (spawn, observer) = task.spawn_local_objsafe(self);
            self.tasks.push(Box::pin(spawn));
            observer
        })
    }

    fn executor_notifier(&mut self) -> Option<Self::ExecutorNotifier> {
        None
    }
}


impl<'tasks> LocalExecutorExt<'tasks> for Executor<'tasks> {

}

#[cfg(test)] mod tests {
    use std::future::Future;
    use std::pin::Pin;
    use some_executor::observer::{NoNotified, Observation};
    use some_executor::SomeLocalExecutor;
    use some_executor::task::{Configuration, Task};
    use crate::{Executor, RunResult};

    struct PollCounter(u8);
    impl Future for PollCounter {
        type Output = ();
        fn poll(self: Pin<&mut Self>, _: &mut std::task::Context<'_>) -> std::task::Poll<Self::Output> {
            let this = self.get_mut();
            this.0 += 1;
            if this.0 < 10 {
                std::task::Poll::Pending
            }
            else {
                std::task::Poll::Ready(())
            }
        }
    }

    #[test] fn test_do() {
        let mut executor = Executor::new();
        assert_eq!(executor.do_some(), RunResult::DidNothing);
        let counter = PollCounter(0);
        let task = Task::without_notifications("test_do".to_string(), counter, Configuration::default());

        let observer = executor.spawn_local(task);
        for _ in 0..9 {
            assert_eq!(executor.do_some(), RunResult::DidSome);

        }
        assert_eq!(executor.do_some(), RunResult::Done);

    }
}

