/*!
Provides an implementation of [some_executor::SomeExecutorExt].
*/

use std::any::Any;
use std::convert::Infallible;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use some_executor::{DynExecutor, SomeExecutor, SomeExecutorExt};
use some_executor::observer::{ExecutorNotified, Observer, ObserverNotified};
use some_executor::task::{DynSpawnedTask, Task};
use crate::channel::{Receiver, Sender};
use crate::Executor;

pub(crate) struct Shared {
    pending_tasks: Mutex<Vec<Box<dyn DynSpawnedTask<SomeExecutorAdapter>>>>,
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
}

pub struct SomeExecutorAdapter {
    shared: Arc<Shared>,
}

impl SomeExecutorAdapter {
    pub fn new(executor: &mut Executor) -> Self {
        let shared = executor.adapter_shared().clone();
        Self {
            shared,
        }
    }
}

impl SomeExecutor for SomeExecutorAdapter {
    type ExecutorNotifier = Infallible;

    fn spawn<F: Future + Send + 'static, Notifier: ObserverNotified<F::Output> + Send>(&mut self, task: Task<F, Notifier>) -> Observer<F::Output, Self::ExecutorNotifier>
    where
        Self: Sized,
        F::Output: Send
    {
        let (task, observer) = task.spawn(self);

        self.shared.pending_tasks.lock().unwrap().push(Box::new(task));

        todo!("actual task");
    }

    fn spawn_async<F: Future + Send + 'static, Notifier: ObserverNotified<F::Output> + Send>(&mut self, task: Task<F, Notifier>) -> impl Future<Output=Observer<F::Output, Self::ExecutorNotifier>> + Send + 'static
    where
        Self: Sized,
        F::Output: Send
    {
        async {
            todo!()
        }
    }

    fn spawn_objsafe(&mut self, task: Task<Pin<Box<dyn Future<Output=Box<dyn Any + 'static + Send>> + 'static + Send>>, Box<dyn ObserverNotified<dyn Any + Send> + Send>>) -> Observer<Box<dyn Any + 'static + Send>, Box<dyn ExecutorNotified + 'static + Send>> {
        todo!()
    }

    fn clone_box(&self) -> Box<DynExecutor> {
        todo!()
    }

    fn executor_notifier(&mut self) -> Option<Self::ExecutorNotifier> {
        todo!()
    }
}

impl Clone for SomeExecutorAdapter {
    fn clone(&self) -> Self {
        todo!()
    }
}

impl SomeExecutorExt for SomeExecutorAdapter {

}
