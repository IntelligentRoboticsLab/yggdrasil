use std::{future::Future, marker::Send, time::Duration};

pub trait Combinators<T>: Sized {
    fn with_timeout(self, duration: Duration) -> impl Future<Output = Option<T>> + Send
    where
        Self: Send + Future<Output = T>,
    {
        async move { async_std::future::timeout(duration, self).await.ok() }
    }
}

impl<T, F: Future<Output = T>> Combinators<T> for F {}
