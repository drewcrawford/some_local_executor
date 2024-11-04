/*!
Provides an implementation of [some_executor::SomeExecutorExt].
*/

use std::sync::Arc;
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

