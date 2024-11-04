/*!
Provides an implementation of [some_executor::SomeExecutorExt].
*/

use std::any::Any;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use some_executor::{DynExecutor, SomeExecutor, SomeExecutorExt};
use some_executor::observer::{ExecutorNotified, NoNotified, Observer, ObserverNotified};
use some_executor::task::Task;
use crate::Executor;

pub(crate) struct Shared {

}
impl Shared {
    pub fn new() -> Self {
        Self {

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
    type ExecutorNotifier = NoNotified;

    fn spawn<F: Future + Send + 'static, Notifier: ObserverNotified<F::Output> + Send>(&mut self, task: Task<F, Notifier>) -> Observer<F::Output, Self::ExecutorNotifier>
    where
        Self: Sized,
        F::Output: Send
    {
        todo!()
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
