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




pub struct Executor<'tasks> {
    tasks: Vec<Pin<Box<dyn DynLocalSpawnedTask + 'tasks>>>,
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
    Runs the executor a small amount if there is some work to be performed.

    */
    pub fn do_some(&mut self) -> RunResult {
        let mut context = Context::from_waker(&self.waker);
        if let Some(mut task) = self.tasks.pop() {
            // let e = Pin::new(&mut task).poll(&mut context);
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
            let pinned_spawn = Box::pin(spawn);
            self.tasks.push(pinned_spawn);
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

impl Clone for Executor<'_> {
    fn clone(&self) -> Self {
        todo!()
    }
}

impl<'tasks> LocalExecutorExt<'tasks> for Executor<'tasks> {

}

