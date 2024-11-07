/*!
It's a simple single-threaded executor.

This is a reference executor for the [https://sealedabstract.com/code/some_executor](some_executor) crate.

By leveraging the features `some_executor`, this project provides a rich API while still being extremely simple to implement:
1.  Tasks and cancellation
2.  Observers and notifications
3.  Priorities and scheduling
4.  task-locals
*/

use some_executor::SomeLocalExecutor;

mod channel;
pub mod some_executor_adapter;

use std::any::Any;
use std::convert::Infallible;
use std::fmt::Debug;
use std::future::Future;
use std::mem::forget;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, RawWaker, RawWakerVTable, Waker};
use some_executor::{LocalExecutorExt};
use some_executor::observer::{ExecutorNotified, Observer, ObserverNotified};
use some_executor::task::{DynLocalSpawnedTask, DynSpawnedTask, TaskID};
use crate::channel::{FindSlot, Sender};

pub type Task<F,N> = some_executor::task::Task<F,N>;
pub type Configuration = some_executor::task::Configuration;

const VTABLE: RawWakerVTable = RawWakerVTable::new(
    |data| {
        let context = unsafe{
            Arc::from_raw(data as *const WakeContext)
        };
        let cloned = Arc::into_raw(context.clone());
        forget(context);
        RawWaker::new(cloned as *const (), &VTABLE)
    },
    |data| {
        let context = unsafe{
            // we effectively rely on the Send/Sync property of WakeContext here.
            // fortunately we can check it at compile time
            assert_send_sync::<WakeContext>();
            Arc::from_raw(data as *const WakeContext)
        };
        match Arc::try_unwrap(context) {
            Ok(context) => {
                context.sender.send();
            }
            Err(context) => {
                context.sender.send_by_ref();
            }
        }

    },
    |data| {
       let context = unsafe{
            Arc::from_raw(data as *const WakeContext)
        };
        context.sender.send_by_ref();
        forget(context);
    },
    |data| {
        unsafe{Arc::from_raw(data as *const WakeContext)}; //drop the arc
    }
);

fn assert_send_sync<T: Send + Sync>() {}

//wrapped in Arc
struct WakeContext {
    sender: Sender,
}

struct SubmittedTask<'task> {
    task: Pin<Box<dyn DynLocalSpawnedTask<Executor<'task>> + 'task>>,
    ///Providing a stable Waker for each task is optimal.
    waker: Waker,
    task_id: TaskID
}

impl Debug for SubmittedTask<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SubmittedTask")
            .field("task_id", &self.task_id)
            .finish()
    }
}

struct SubmittedRemoteTask<'a> {
    task: Pin<Box<dyn DynSpawnedTask<Executor<'a>>>>,
    ///Providing a stable Waker for each task is optimal.
    waker: Waker,
    task_id: TaskID
}

impl Debug for SubmittedRemoteTask<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SubmittedRemoteTask")
            .field("task_id", &self.task_id)
            .finish()
    }
}

impl<'executor> SubmittedTask<'executor> {
    fn new(task: Pin<Box<(dyn DynLocalSpawnedTask<Executor<'executor>> + 'executor)>>, sender: Sender) -> Self {
        let context = Arc::new(WakeContext{
            sender
        });
        let context_raw = Arc::into_raw(context);
        let waker = unsafe {
            // we effectively rely on the Send/Sync property of WakeContext here.
            // fortunately we can check it at compile time
            assert_send_sync::<WakeContext>();
            Waker::from_raw(RawWaker::new(context_raw as *const (), &VTABLE))
        };
        let task_id = task.task_id();
        SubmittedTask {
            task: task,
            waker,
            task_id,
        }
    }
}

impl<'task> SubmittedRemoteTask<'task> {
    fn new(task: Pin<Box<dyn DynSpawnedTask<Executor<'task>>>>, sender: Sender) -> Self {
        let context = Arc::new(WakeContext {
            sender
        });
        let context_raw = Arc::into_raw(context);
        let waker = unsafe {
            // we effectively rely on the Send/Sync property of WakeContext here.
            // fortunately we can check it at compile time
            assert_send_sync::<WakeContext>();
            Waker::from_raw(RawWaker::new(context_raw as *const (), &VTABLE))
        };
        let task_id = task.task_id();
        SubmittedRemoteTask {
            task: task,
            waker,
            task_id,
        }
    }
}



/**
The main executor type.

The lifetime parameter `'tasks` is the lifetime of the tasks that are spawned on this executor, that is
the lifetime of any data they may borrow, in case they are spawned with a reference to that data.
*/
#[derive(Debug)]
pub struct Executor<'tasks> {

    ready_for_poll: Vec<SubmittedTask<'tasks>>,
    ready_for_poll_remote: Vec<SubmittedRemoteTask<'tasks>>,
    waiting_for_wake: Vec<SubmittedTask<'tasks>>,
    waiting_for_wake_remote: Vec<SubmittedRemoteTask<'tasks>>,

    //slot so we can take
    wake_receiver: Option<channel::Receiver>,

    adapter_shared: Arc<some_executor_adapter::Shared>,
}

impl<'tasks> Executor<'tasks> {
    /**
    Creates a new executor.
    */
    pub fn new() -> Self {

        let mut wake_receiver = channel::Receiver::new();
        let adapter_shared = Arc::new(some_executor_adapter::Shared::new(&mut wake_receiver));
        Executor {
            ready_for_poll: Vec::new(),
            ready_for_poll_remote: Vec::new(),
            waiting_for_wake: Vec::new(),
            waiting_for_wake_remote: Vec::new(),
            wake_receiver: Some(wake_receiver),
            adapter_shared,
        }
    }
    /**
    Runs the executor until there is no more immediate work to be performed.

    It is intended ot be called in a loop with [Self::park_if_needed].
    */
    pub fn do_some(&mut self)  {

        let receiver = self.wake_receiver.take().expect("Receiver is not available");

        let attempt_tasks = self.ready_for_poll.drain(..).collect::<Vec<_>>();
        let _interval = logwise::perfwarn_begin!("do_some does not currently sort tasks well");
        for mut task in attempt_tasks {
            let mut context = Context::from_waker(&task.waker);
            let logwise_task = logwise::context::Context::new_task(Some(logwise::context::Context::current()), "single_threaded_executor::do_some");
            let context_id = logwise_task.context_id();
            logwise_task.set_current();
            logwise::debuginternal_sync!("Polling task {id} {label}", id=logwise::privacy::IPromiseItsNotPrivate(task.task_id), label=task.task.label());
            let e = task.task.as_mut().poll(&mut context, self,None);

            match e {
                std::task::Poll::Ready(_) => {
                    // do nothing; drop the future
                }
                std::task::Poll::Pending => {
                    //need to retain the task
                    self.waiting_for_wake.push(task);
                }
            }
            logwise::context::Context::pop(context_id);
        }

        let attempt_remote_tasks = self.ready_for_poll_remote.drain(..).collect::<Vec<_>>();
        for mut task in attempt_remote_tasks {
            let mut context = Context::from_waker(&task.waker);
            let logwise_task = logwise::context::Context::new_task(Some(logwise::context::Context::current()), "single_threaded_executor::do_some");
            let context_id = logwise_task.context_id();
            logwise_task.set_current();
            logwise::debuginternal_sync!("Polling task {id} {label}", id=logwise::privacy::IPromiseItsNotPrivate(task.task.task_id()), label=task.task.label());
            let e = task.task.as_mut().poll(&mut context, None);

            match e {
                std::task::Poll::Ready(_) => {
                    // do nothing; drop the future
                }
                std::task::Poll::Pending => {
                    //need to retain the task
                    self.waiting_for_wake_remote.push(task);
                }
            }
            logwise::context::Context::pop(context_id);
        }

        self.wake_receiver = Some(receiver);
        drop(_interval);
    }
    /**
    Parks the thread if there are no tasks to be performed, until tasks are ready to be performed again.
*/
    pub fn park_if_needed(&mut self) {
        if !self.has_unfinished_tasks() { return }

        if self.ready_for_poll.is_empty() && self.ready_for_poll_remote.is_empty() {
            let mut receiver = self.wake_receiver.take().expect("Receiver is not available");
            let task_id = receiver.recv_park();
            match task_id {
                FindSlot::FoundSlot(task_id) => {
                    {
                        let _interval = logwise::perfwarn_begin!("O(n) search for task_id");

                        if let Some(pos) = self.waiting_for_wake.iter().position(|x| x.task_id == task_id) {
                            let task = self.waiting_for_wake.remove(pos);
                            self.ready_for_poll.push(task);
                        }
                        else if let Some(pos) = self.waiting_for_wake_remote.iter().position(|x| x.task_id == task_id) {
                            let task = self.waiting_for_wake_remote.remove(pos);
                            self.ready_for_poll_remote.push(task);
                        }
                        else {
                            unreachable!("Task ID not found");
                        }
                    };

                }
                FindSlot::FoundTaskSubmitted => {
                    for task in self.adapter_shared.take_pending_tasks() {
                        let sender = Sender::with_receiver(&mut receiver, task.task_id());
                        self.ready_for_poll_remote.push(SubmittedRemoteTask::new(Box::into_pin(task), sender));
                    }
                }
                FindSlot::NoSlot => {
                    unreachable!("spurious wakeup")
                }
            }
            //remove the first value from waiting_for_wake that has the same task_id


            self.wake_receiver = Some(receiver);
        }
    }
    /**
    Drains the executor. After this call, the executor can no longer be used.

    This function will return when all spawned tasks complete.
*/
    pub fn drain(mut self) {
        while self.has_unfinished_tasks() {
            self.do_some();
            self.park_if_needed();
        }
    }

    /**
    Returns true if there are tasks that are not yet complete.
    */
    pub fn has_unfinished_tasks(&self) -> bool {

        !self.ready_for_poll.is_empty()
            || !self.ready_for_poll_remote.is_empty()
            || !self.waiting_for_wake.is_empty()
            || !self.waiting_for_wake_remote.is_empty()
            || self.wake_receiver.as_ref().expect("No receiver").has_data()
    }

    fn enqueue_task(&mut self, task: Pin<Box<(dyn DynLocalSpawnedTask<Executor<'tasks>> + 'tasks)>>) {
        if task.poll_after() < std::time::Instant::now() {
            let sender = Sender::with_receiver(self.wake_receiver.as_mut().expect("Receiver is not available"), task.task_id());
            self.ready_for_poll.push(SubmittedTask::new(task, sender));
        }
        else {
            todo!("Not yet implemented")
        }
    }

    pub(crate) fn adapter_shared(&self) -> &Arc<some_executor_adapter::Shared> {
        &self.adapter_shared
    }
}

/**
Implementation of the `SomeLocalExecutor` trait from the [some_executor](http://sealedabstract.com/code/some_executor) project.

This is the main interface to spawn tasks onto the executor.

For details on this trait, see [SomeLocalExecutor].
*/
impl<'future> SomeLocalExecutor<'future> for Executor<'future> {
    type ExecutorNotifier = Infallible;

    fn spawn_local<F: Future, Notifier: ObserverNotified<F::Output>>(&mut self, task: Task<F, Notifier>) -> Observer<F::Output, Self::ExecutorNotifier>
    where
        Self: Sized,
        F: 'future,
        <F as Future>::Output: Unpin,
    {
        let (task,observer) = task.spawn_local(self);
        self.enqueue_task(Box::pin(task));
        observer
    }

    fn spawn_local_async<F: Future, Notifier: ObserverNotified<F::Output>>(&mut self, task: Task<F, Notifier>) -> impl Future<Output=Observer<F::Output, Self::ExecutorNotifier>>
    where
        Self: Sized,
        F: 'future,
    {
        async {
            let (spawn,observer)  = task.spawn_local(self);
            self.enqueue_task(Box::pin(spawn));
            observer
        }
    }
    fn spawn_local_objsafe(&mut self, task: Task<Pin<Box<dyn Future<Output=Box<dyn Any>>>>, Box<dyn ObserverNotified<(dyn Any + 'static)>>>) -> Observer<Box<dyn Any>, Box<dyn ExecutorNotified>> {
        let (spawn, observer) = task.spawn_local_objsafe(self);
        self.enqueue_task(Box::pin(spawn));
        observer
    }

    fn spawn_local_objsafe_async<'executor>(&'executor mut self, task: Task<Pin<Box<dyn Future<Output=Box<dyn Any>>>>, Box<dyn ObserverNotified<(dyn Any + 'static)>>>) -> Box<dyn Future<Output=Observer<Box<dyn Any>, Box<dyn ExecutorNotified>>> + 'executor> {
        Box::new(async {
            let (spawn, observer) = task.spawn_local_objsafe(self);
            self.enqueue_task(Box::pin(spawn));
            observer
        })
    }

    fn executor_notifier(&mut self) -> Option<Self::ExecutorNotifier> {
        None
    }
}


impl<'tasks> LocalExecutorExt<'tasks> for Executor<'tasks> {

}

//executor boilerplate



#[cfg(test)] mod tests {
    use std::future::Future;
    use std::pin::Pin;
    use some_executor::observer::{Observation};
    use some_executor::SomeLocalExecutor;
    use some_executor::task::{Configuration, Task};
    use crate::{Executor};

    struct PollCounter(u8);
    impl Future for PollCounter {
        type Output = ();
        fn poll(self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Self::Output> {
            let this = self.get_mut();
            this.0 += 1;
            cx.waker().clone().wake();
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
            executor.park_if_needed();
        }

    }
}

