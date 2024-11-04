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
        self.shared.new_pending_task_sender.send_by_ref();
        observer
    }

    fn spawn_async<F: Future + Send + 'static, Notifier: ObserverNotified<F::Output> + Send>(&mut self, task: Task<F, Notifier>) -> impl Future<Output=Observer<F::Output, Self::ExecutorNotifier>> + Send + 'static
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

    fn spawn_objsafe(&mut self, task: Task<Pin<Box<dyn Future<Output=Box<dyn Any + 'static + Send>> + 'static + Send>>, Box<dyn ObserverNotified<dyn Any + Send> + Send>>) -> Observer<Box<dyn Any + 'static + Send>, Box<dyn ExecutorNotified + 'static + Send>> {
        let (task, observer) = task.spawn_objsafe(self);
        self.shared.pending_tasks.lock().unwrap().push(Box::new(task));
        self.shared.new_pending_task_sender.send_by_ref();
        observer
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

impl Clone for SomeExecutorAdapter {
    fn clone(&self) -> Self {
        todo!()
    }
}

impl SomeExecutorExt for SomeExecutorAdapter {

}

#[cfg(test)]
mod tests {
    use some_executor::SomeExecutor;
    use some_executor::task::Configuration;
    use crate::Executor;
    use crate::some_executor_adapter::SomeExecutorAdapter;

    #[test] fn spawn_task() {
        let mut ex = Executor::new();
        let mut adapter = SomeExecutorAdapter::new(&mut ex);

        let task = some_executor::task::Task::without_notifications("spawn_task".to_string(), async {
            println!("Hello, world!");
        }, Configuration::default());

        let o = adapter.spawn(task);

        ex.drain();

    }
}
