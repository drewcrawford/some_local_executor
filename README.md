# some_local_executor



It's a simple single-threaded executor.

This is a reference executor for the [some_executor](https://sealedabstract.com/code/some_executor) crate.

By leveraging the features in `some_executor`, this project provides a rich API:
1.  Tasks and cancellation
2.  Observers and notifications
3.  Priorities and scheduling
4.  task-locals

...in about a page of code.

By writing code against this executor, you're basically compatible with every other executor supported
by `some_executor`.