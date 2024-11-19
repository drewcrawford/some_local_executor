//SPDX-License-Identifier: MIT OR Apache-2.0
/*!
Provides an implementation of [some_executor::SomeExecutorExt], so that the executor
can be used from multiple threads.

Using the adapter has some overhead, but it provides strong compatibility.
*/

use std::any::Any;
use std::convert::Infallible;
use std::future::Future;
use std::hash::Hash;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use some_executor::{DynExecutor, SomeExecutor, SomeExecutorExt};
use some_executor::observer::{ExecutorNotified, Observer, ObserverNotified};
use some_executor::task::{DynSpawnedTask, Task};
use crate::channel::{Receiver, Sender};
use crate::Executor;

#[derive(Debug)]
pub(crate) struct Shared {
    pending_tasks: Mutex<Vec<Box<dyn for<'a> DynSpawnedTask<Executor<'a>>>>>,
    new_pending_task_sender: Sender,
}
impl Shared {
    pub fn new(receiver: &mut Receiver) -> Self {
        let sender = Sender::with_receiver_for_task_submit(receiver);
        Self {
            pending_tasks: Mutex::new(Vec::new()),
            new_pending_task_sender: sender,

        }
    }
    pub fn take_pending_tasks(&self) -> Vec<Box<dyn for<'a> DynSpawnedTask<Executor<'a>>>> {
        self.pending_tasks.lock().unwrap().drain(..).collect()
    }
}
/**
A type that implements [SomeExecutor] by using an [Executor] as the backend.
*/
#[derive(Debug,Clone)]
pub struct SomeExecutorAdapter {
    shared: Arc<Shared>,
}

impl SomeExecutorAdapter {
    /**
    Creates a new adapter that uses the given executor as the backend.
    */
    pub fn new(executor: &mut Executor) -> Self {
        let shared = executor.adapter_shared().clone();
        Self {
            shared,
        }
    }
}

/**
Implements the [SomeExecutor] trait for [SomeExecutorAdapter].

For more details, see trait documentation.
*/
impl SomeExecutor for SomeExecutorAdapter {
    type ExecutorNotifier = Infallible;

    fn spawn<F: Future + Send + 'static, Notifier: ObserverNotified<F::Output> + Send>(&mut self, task: Task<F, Notifier>) -> impl Observer<Value=F::Output>
    where
        Self: Sized,
        F::Output: Send
    {
        let (task, observer) = task.spawn(self);
        self.shared.pending_tasks.lock().unwrap().push(Box::new(task));
        self.shared.new_pending_task_sender.send_by_ref();
        observer
    }

    fn spawn_async<F: Future + Send + 'static, Notifier: ObserverNotified<F::Output> + Send>(&mut self, task: Task<F, Notifier>) -> impl Future<Output=impl Observer<Value=F::Output>> + Send + 'static
    where
        Self: Sized,
        F::Output: Send
    {
        let (task, observer) = task.spawn(self);
        let move_shared = self.shared.clone();
        async move {
            move_shared.pending_tasks.lock().unwrap().push(Box::new(task));
            move_shared.new_pending_task_sender.send_by_ref();
            observer
        }
    }

    fn spawn_objsafe(&mut self, task: Task<Pin<Box<dyn Future<Output=Box<dyn Any + 'static + Send>> + 'static + Send>>, Box<dyn ObserverNotified<dyn Any + Send> + Send>>) -> Box<dyn Observer<Value=Box<dyn Any + Send>>>  {
        let (task, observer) = task.spawn_objsafe(self);
        self.shared.pending_tasks.lock().unwrap().push(Box::new(task));
        self.shared.new_pending_task_sender.send_by_ref();
        Box::new(observer) as Box<dyn Observer<Value=Box<dyn Any + Send>>>
    }

    fn clone_box(&self) -> Box<DynExecutor> {
        Box::new(Self {
            shared: self.shared.clone(),
        })
    }

    fn executor_notifier(&mut self) -> Option<Self::ExecutorNotifier> {
        None
    }
}



impl SomeExecutorExt for SomeExecutorAdapter {

}

//boilerplate.
impl PartialEq for SomeExecutorAdapter {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.shared, &other.shared)
    }
}

impl Eq for SomeExecutorAdapter {

}

impl Hash for SomeExecutorAdapter {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        Arc::as_ptr(&self.shared).hash(state);
    }
}

//default - no, we need the backend.

impl From<&mut Executor<'_>> for SomeExecutorAdapter {
    fn from(executor: &mut Executor) -> Self {
        Self::new(executor)
    }
}


#[cfg(test)]
mod tests {
    use std::future::Future;
    use std::pin::Pin;
    use std::task::{Context, Poll};
    use some_executor::observer::Observation;
    use some_executor::SomeExecutor;
    use some_executor::task::Configuration;
    use crate::Executor;
    use crate::some_executor_adapter::SomeExecutorAdapter;
    use some_executor::observer::Observer;

    #[test] fn spawn_task() {
        let mut ex = Executor::new();
        let mut adapter = SomeExecutorAdapter::new(&mut ex);

        let task = some_executor::task::Task::without_notifications("spawn_task".to_string(), async {
            println!("Hello, world!");
        }, Configuration::default());


        let o = adapter.spawn(task);
        assert_eq!(o.observe(), Observation::Pending);
        ex.drain();
        assert_eq!(o.observe(), Observation::Ready(()));
        assert_eq!(o.observe(), Observation::Done);

    }

    #[test] fn spawn_task_wake() {
        struct F(bool);
        impl Future for F {
            type Output = ();

            fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
                if self.0 {
                    Poll::Ready(())
                } else {
                    self.0 = true;
                    cx.waker().wake_by_ref();
                    Poll::Pending
                }
            }
        }
        let mut ex = Executor::new();
        let mut adapter = SomeExecutorAdapter::new(&mut ex);

        let task = some_executor::task::Task::without_notifications("spawn_task".to_string(), F(false), Configuration::default());

        let o = adapter.spawn(task);
        assert_eq!(o.observe(), Observation::Pending);
        ex.drain();
        assert_eq!(o.observe(), Observation::Ready(()));
        assert_eq!(o.observe(), Observation::Done);

    }

    #[test] fn test_send_sync() {
        fn is_send_sync<T: Send + Sync>() {}
        is_send_sync::<SomeExecutorAdapter>();
    }
}
