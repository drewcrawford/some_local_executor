/*!
It's a simple single-threaded executor.
*/

mod continuation_type;

use std::any::Any;
use std::future::Future;
use std::marker::PhantomData;
use std::pin::Pin;
use std::rc::Rc;
use std::sync::Arc;
use std::task::{Context, RawWaker, RawWakerVTable, Waker};
use some_executor::{LocalExecutorExt, SomeExecutor, SomeLocalExecutor};
use some_executor::observer::{ExecutorNotified, NoNotified, Observer, ObserverNotified};
use some_executor::task::{DynLocalSpawnedTask, SpawnedLocalTask, SpawnedTask, Task};
use crate::continuation_type::BlockingContinuationType;



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
        unsafe{Arc::from_raw(data as *const Arc<WakeContext>)}; //drop the arc
    }
);

//wrapped in RC
struct WakeContext;

struct SubmittedTask<'task> {
    task: Pin<Box<dyn DynLocalSpawnedTask<Executor<'task>> + 'task>>,
    ///Providing a stable Waker for the task is optimal.
    waker: Waker,
}

impl<'executor> SubmittedTask<'executor> {
    fn new(task: Pin<Box<(dyn DynLocalSpawnedTask<Executor<'executor>> + 'executor)>>) -> Self {
        let waker = unsafe {
            Waker::from_raw(RawWaker::new(Arc::into_raw(Arc::new(WakeContext)) as *const (), &VTABLE))
        };
        SubmittedTask {
            task,
            waker,
        }
    }
}




pub struct Executor<'tasks> {
    tasks: Vec<SubmittedTask<'tasks>>,
}

impl<'tasks> Executor<'tasks> {
    pub fn new() -> Self {
        //Note that Waker requires Send/sync.  Accordingly, we need Arc, not Rc, for safety.
        //see https://github.com/rust-lang/rust/issues/118959#issuecomment-2453582510
        let waker_context = Arc::into_raw(Arc::new(WakeContext));

        Executor {
            tasks: Vec::new(),
        }
    }
    /**
    Runs the executor until there is no more immediate work to be performed.

    */
    pub fn do_some(&mut self) -> BlockingContinuationType {
        let attempt_tasks = self.tasks.drain(..).collect::<Vec<_>>();
        let b = BlockingContinuationType;


        let mut new_tasks = Vec::new();
        let now = std::time::Instant::now();
        for mut task in attempt_tasks {
            if task.task.poll_after() > now {
                new_tasks.push(task);
                continue;
            }

            let mut context = Context::from_waker(&task.waker);

            let e = task.task.as_mut().poll(&mut context, self);
            match e {
                std::task::Poll::Ready(_) => {
                    // do nothing
                }
                std::task::Poll::Pending => {

                }
            }
        }

        self.tasks = new_tasks;

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

impl<'future> SomeLocalExecutor<'future> for Executor<'future> {
    type ExecutorNotifier = NoNotified;

    fn spawn_local<F: Future, Notifier: ObserverNotified<F::Output>>(&mut self, task: Task<F, Notifier>) -> Observer<F::Output, Self::ExecutorNotifier>
    where
        Self: Sized,
        F: 'future,
        <F as Future>::Output: Unpin,
    {
        let (task,observer) = task.spawn_local(self);
        self.tasks.push(SubmittedTask::new(Box::pin(task)));
        observer
    }

    fn spawn_local_async<F: Future, Notifier: ObserverNotified<F::Output>>(&mut self, task: Task<F, Notifier>) -> impl Future<Output=Observer<F::Output, Self::ExecutorNotifier>>
    where
        Self: Sized,
        F: 'future,
    {
        async {
            let (spawn,observer)  = task.spawn_local(self);
            self.tasks.push(SubmittedTask::new(Box::pin(spawn)));
            observer
        }
    }
    fn spawn_local_objsafe(&mut self, task: Task<Pin<Box<dyn Future<Output=Box<dyn Any>>>>, Box<dyn ObserverNotified<(dyn Any + 'static)>>>) -> Observer<Box<dyn Any>, Box<dyn ExecutorNotified>> {
        let (spawn, observer) = task.spawn_local_objsafe(self);
        self.tasks.push(SubmittedTask::new(Box::pin(spawn)));
        observer
    }

    fn spawn_local_objsafe_async<'executor>(&'executor mut self, task: Task<Pin<Box<dyn Future<Output=Box<dyn Any>>>>, Box<dyn ObserverNotified<(dyn Any + 'static)>>>) -> Box<dyn Future<Output=Observer<Box<dyn Any>, Box<dyn ExecutorNotified>>> + 'executor> {
        Box::new(async {
            let (spawn, observer) = task.spawn_local_objsafe(self);
            self.tasks.push(SubmittedTask::new(Box::pin(spawn)));
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
    use crate::{Executor};

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

    #[test] fn test_do_empty() {
        let mut executor = Executor::new();
        executor.do_some();
    }

    #[test] fn test_do() {
        let mut executor = Executor::new();
        let counter = PollCounter(0);
        let task = Task::without_notifications("test_do".to_string(), counter, Configuration::default());

        let observer = executor.spawn_local(task);
        for _ in 0..9 {
            executor.do_some();
            assert_eq!(observer.observe(), Observation::Pending);
        }

    }
}

