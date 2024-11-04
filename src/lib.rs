/*!
It's a simple single-threaded executor.
*/

mod channel;

use std::any::Any;
use std::future::Future;
use std::marker::PhantomData;
use std::mem::forget;
use std::pin::Pin;
use std::rc::Rc;
use std::sync::Arc;
use std::task::{Context, RawWaker, RawWakerVTable, Waker};
use some_executor::{LocalExecutorExt, SomeExecutor, SomeLocalExecutor};
use some_executor::observer::{ExecutorNotified, NoNotified, Observer, ObserverNotified};
use some_executor::task::{DynLocalSpawnedTask, SpawnedLocalTask, SpawnedTask, Task, TaskID};
use crate::channel::Sender;

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
        todo!()
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




pub struct Executor<'tasks> {

    ready_for_poll: Vec<SubmittedTask<'tasks>>,
    waiting_for_wake: Vec<SubmittedTask<'tasks>>,

    //slot so we can take
    wake_receiver: Option<channel::Receiver>,
}

impl<'tasks> Executor<'tasks> {
    pub fn new() -> Self {

        let wake_receiver = channel::Receiver::new();
        Executor {
            ready_for_poll: Vec::new(),
            waiting_for_wake: Vec::new(),
            wake_receiver: Some(wake_receiver),
        }
    }
    /**
    Runs the executor until there is no more immediate work to be performed.

    Returns a reference to a receiver that can be [continuation_type::Receiver::park]ed to wait for more work to be available.
    */
    pub fn do_some(&mut self)  {

        let mut receiver = self.wake_receiver.take().expect("Receiver is not available");

        let attempt_tasks = self.ready_for_poll.drain(..).collect::<Vec<_>>();
        let _interval = logwise::perfwarn_begin!("do_some does not currently sort tasks well");
        for mut task in attempt_tasks {
            let mut context = Context::from_waker(&task.waker);
            let logwise_task = logwise::context::Context::new_task(Some(logwise::context::Context::current()), "single_threaded_executor::do_some");
            let context_id = logwise_task.context_id();
            logwise_task.set_current();
            logwise::debuginternal_sync!("Polling task {id} {label}", id=logwise::privacy::IPromiseItsNotPrivate(task.task_id), label=task.task.label());
            let e = task.task.as_mut().poll(&mut context, self);

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

        self.wake_receiver = Some(receiver);
        drop(_interval);
    }
    pub fn park_if_needed(&mut self) {
        if self.ready_for_poll.is_empty() {
            let mut receiver = self.wake_receiver.take().expect("Receiver is not available");
            let task_id = receiver.recv_park();
            //remove the first value from waiting_for_wake that has the same task_id
            let task = {
                let _interval = logwise::perfwarn_begin!("O(n) search for task_id");

                let pos = self.waiting_for_wake.iter().position(|x| x.task_id == task_id).expect("Task ID not found");
                self.waiting_for_wake.remove(pos)
            };
            self.ready_for_poll.push(task);

            self.wake_receiver = Some(receiver);
        }
    }
    /**
    Drains the executor. After this call, the executor can no longer be used.

    This function will return when all spawned tasks complete.
*/
    pub fn drain(self) {
        todo!()
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

