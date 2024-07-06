use crate::http::{ClientAsync, ClientSync, Error, FromResponse, Request};
use std::fmt::Debug;
use std::future::Future;
#[cfg(not(feature = "async-traits"))]
use std::pin::Pin;

#[cfg(not(feature = "async-traits"))]
type SequenceFuture<'a, O, E> = Pin<Box<dyn Future<Output = Result<O, E>> + 'a>>;

/// Trait which can be use to link a sequence of request operations.
pub trait Sequence {
    type Output;
    type Error: From<Error> + Debug;

    fn do_sync<T: ClientSync>(self, client: &T) -> Result<Self::Output, Self::Error>;

    #[cfg(not(feature = "async-traits"))]
    fn do_async<'a, T: ClientAsync>(
        self,
        client: &'a T,
    ) -> SequenceFuture<'a, Self::Output, Self::Error>
    where
        Self: 'a;

    #[cfg(feature = "async-traits")]
    fn do_async<'a, T: ClientAsync>(
        self,
        client: &'a T,
    ) -> impl Future<Output = Result<Self::Output, Self::Error>> + 'a
    where
        Self: 'a;

    fn map<O, E, F: FnOnce(Self::Output) -> Result<O, E>>(self, f: F) -> MapSequence<Self, F>
    where
        Self: Sized,
        E: From<Self::Error> + From<Error> + Debug,
    {
        MapSequence { c: self, f }
    }

    fn map_err<E, F: FnOnce(Self::Error) -> Result<Self::Output, E>>(
        self,
        f: F,
    ) -> MapErrSequence<Self, F>
    where
        Self: Sized,
        E: From<Self::Error> + From<Error> + Debug,
    {
        MapErrSequence { c: self, f }
    }

    fn state<SS, F>(self, f: F) -> SequenceWithState<Self, F>
    where
        Self: Sized,
        SS: Sequence,
        F: FnOnce(Self::Output) -> SS,
        <SS as Sequence>::Error: From<Self::Error> + From<Error> + Debug,
    {
        SequenceWithState { seq: self, f }
    }

    fn chain<SS, F>(self, f: F) -> SequenceChain<Self, F>
    where
        SS: Sequence<Error = Self::Error>,
        F: FnOnce(Self::Output) -> Result<SS, Self::Error>,
        Self: Sized,
    {
        SequenceChain { s: self, f }
    }

    fn chain_err<SS, F>(self, f: F) -> SequenceErrChain<Self, F>
    where
        SS: Sequence<Output = Self::Output, Error = Self::Error>,
        F: FnOnce(Self::Error) -> Result<SS, Self::Error>,
        <SS as Sequence>::Error: From<Self::Error> + Debug,
        Self: Sized,
    {
        SequenceErrChain { s: self, f }
    }
}

impl<R: Request> Sequence for R {
    type Output = <R::Response as FromResponse>::Output;
    type Error = Error;

    fn do_sync<T: ClientSync>(self, client: &T) -> Result<Self::Output, Self::Error> {
        self.exec_sync(client)
    }

    #[cfg(not(feature = "async-traits"))]
    fn do_async<'a, T: ClientAsync>(
        self,
        client: &'a T,
    ) -> Pin<Box<dyn Future<Output = Result<Self::Output, Self::Error>> + 'a>>
    where
        Self: 'a,
    {
        Box::pin(async move { self.exec_async(client).await })
    }

    #[cfg(feature = "async-traits")]
    fn do_async<'a, T: ClientAsync>(
        self,
        client: &'a T,
    ) -> impl Future<Output = Result<<R as Sequence>::Output, <R as Sequence>::Error>> + 'a
    where
        R: 'a,
    {
        async move { self.exec_async(client).await }
    }
}

#[doc(hidden)]
pub struct MapSequence<C, F> {
    c: C,
    f: F,
}

impl<C, O, E, F> Sequence for MapSequence<C, F>
where
    C: Sequence,
    F: FnOnce(C::Output) -> Result<O, E>,
    E: From<Error> + Debug + From<C::Error>,
{
    type Output = O;
    type Error = E;

    fn do_sync<T: ClientSync>(self, client: &T) -> Result<Self::Output, Self::Error> {
        let v = self.c.do_sync(client)?;
        let r = (self.f)(v)?;
        Ok(r)
    }

    #[cfg(not(feature = "async-traits"))]
    fn do_async<'a, T: ClientAsync>(
        self,
        client: &'a T,
    ) -> Pin<Box<dyn Future<Output = Result<Self::Output, Self::Error>> + 'a>>
    where
        Self: 'a,
    {
        Box::pin(async move {
            let v = self.c.do_async(client).await?;
            let r = (self.f)(v)?;
            Ok(r)
        })
    }

    #[cfg(feature = "async-traits")]
    fn do_async<'a, T: ClientAsync>(
        self,
        client: &'a T,
    ) -> impl Future<
        Output = Result<
            <MapSequence<C, F> as Sequence>::Output,
            <MapSequence<C, F> as Sequence>::Error,
        >,
    > + 'a
    where
        F: 'a,
        C: 'a,
    {
        async move {
            let v = self.c.do_async(client).await?;
            let r = (self.f)(v)?;
            Ok(r)
        }
    }
}

#[doc(hidden)]
pub struct MapErrSequence<C, F> {
    c: C,
    f: F,
}

impl<C, E, F> Sequence for MapErrSequence<C, F>
where
    C: Sequence,
    F: FnOnce(C::Error) -> Result<C::Output, E>,
    E: From<Error> + Debug + From<C::Error>,
{
    type Output = C::Output;
    type Error = E;

    fn do_sync<T: ClientSync>(self, client: &T) -> Result<Self::Output, Self::Error> {
        match self.c.do_sync(client) {
            Ok(o) => Ok(o),
            Err(e) => (self.f)(e),
        }
    }

    #[cfg(not(feature = "async-traits"))]
    fn do_async<'a, T: ClientAsync>(
        self,
        client: &'a T,
    ) -> Pin<Box<dyn Future<Output = Result<Self::Output, Self::Error>> + 'a>>
    where
        Self: 'a,
    {
        Box::pin(async move {
            match self.c.do_async(client).await {
                Ok(o) => Ok(o),
                Err(e) => (self.f)(e),
            }
        })
    }

    #[cfg(feature = "async-traits")]
    fn do_async<'a, T: ClientAsync>(
        self,
        client: &'a T,
    ) -> impl Future<
        Output = Result<
            <MapErrSequence<C, F> as Sequence>::Output,
            <MapErrSequence<C, F> as Sequence>::Error,
        >,
    > + 'a
    where
        F: 'a,
        C: 'a,
    {
        async move {
            let v = self.c.do_async(client).await?;
            let r = (self.f)(v)?;
            Ok(r)
        }
    }
}

#[doc(hidden)]
pub struct SequenceWithState<S, F> {
    seq: S,
    f: F,
}

impl<S, SS, F> Sequence for SequenceWithState<S, F>
where
    S: Sequence,
    SS: Sequence,
    <SS as Sequence>::Error: From<<S as Sequence>::Error> + From<Error> + Debug,
    F: FnOnce(S::Output) -> SS,
{
    type Output = SS::Output;
    type Error = SS::Error;

    fn do_sync<T: ClientSync>(self, client: &T) -> Result<Self::Output, Self::Error> {
        let state = self.seq.do_sync(client)?;
        let ss = (self.f)(state);
        ss.do_sync(client)
    }

    #[cfg(not(feature = "async-traits"))]
    fn do_async<'a, T: ClientAsync>(
        self,
        client: &'a T,
    ) -> Pin<Box<dyn Future<Output = Result<Self::Output, Self::Error>> + 'a>>
    where
        Self: 'a,
    {
        Box::pin(async move {
            let state = self.seq.do_async(client).await?;
            let ss = (self.f)(state);
            ss.do_async(client).await
        })
    }

    #[cfg(feature = "async-traits")]
    fn do_async<'a, T: ClientAsync>(
        self,
        client: &'a T,
    ) -> impl Future<
        Output = Result<
            <SequenceWithState<S, F> as Sequence>::Output,
            <SequenceWithState<S, F> as Sequence>::Error,
        >,
    > + 'a
    where
        F: 'a,
        S: 'a,
    {
        async move {
            let state = self.seq.do_async(client).await?;
            let ss = (self.f)(state);
            ss.do_async(client).await
        }
    }
}

#[doc(hidden)]
pub struct SequenceFromState<S, F> {
    s: S,
    f: F,
}

impl<S, F> SequenceFromState<S, F> {
    pub fn new(s: S, f: F) -> Self {
        Self { s, f }
    }
}

impl<Seq, S, F> Sequence for SequenceFromState<S, F>
where
    Seq: Sequence,
    F: FnOnce(S) -> Seq,
{
    type Output = Seq::Output;
    type Error = Seq::Error;

    fn do_sync<T: ClientSync>(self, client: &T) -> Result<Self::Output, Self::Error> {
        let seq = (self.f)(self.s);
        seq.do_sync(client)
    }

    #[cfg(not(feature = "async-traits"))]
    fn do_async<'a, T: ClientAsync>(
        self,
        client: &'a T,
    ) -> Pin<Box<dyn Future<Output = Result<Self::Output, Self::Error>> + 'a>>
    where
        Self: 'a,
    {
        Box::pin(async move {
            let seq = (self.f)(self.s);
            seq.do_async(client).await
        })
    }

    #[cfg(feature = "async-traits")]
    fn do_async<'a, T: ClientAsync>(
        self,
        client: &'a T,
    ) -> impl Future<
        Output = Result<
            <SequenceFromState<S, F> as Sequence>::Output,
            <SequenceFromState<S, F> as Sequence>::Error,
        >,
    > + 'a
    where
        Self: 'a,
    {
        async move {
            let seq = (self.f)(self.s);
            seq.do_async(client).await
        }
    }
}

#[doc(hidden)]
pub struct SequenceChain<S, F> {
    s: S,
    f: F,
}

impl<SS, S, F> Sequence for SequenceChain<S, F>
where
    SS: Sequence<Error = S::Error>,
    S: Sequence,
    F: FnOnce(S::Output) -> Result<SS, S::Error>,
    <SS as Sequence>::Error: From<S::Error> + Debug,
{
    type Output = SS::Output;
    type Error = S::Error;

    fn do_sync<T: ClientSync>(self, client: &T) -> Result<Self::Output, Self::Error> {
        let v = self.s.do_sync(client)?;
        let ss = (self.f)(v)?;
        ss.do_sync(client)
    }

    #[cfg(not(feature = "async-traits"))]
    fn do_async<'a, T: ClientAsync>(
        self,
        client: &'a T,
    ) -> Pin<Box<dyn Future<Output = Result<Self::Output, Self::Error>> + 'a>>
    where
        Self: 'a,
    {
        Box::pin(async move {
            let v = self.s.do_async(client).await?;
            let ss = (self.f)(v)?;
            ss.do_async(client).await
        })
    }

    #[cfg(feature = "async-traits")]
    fn do_async<'a, T: ClientAsync>(
        self,
        client: &'a T,
    ) -> impl Future<
        Output = Result<
            <SequenceChain<S, F> as Sequence>::Output,
            <SequenceChain<S, F> as Sequence>::Error,
        >,
    > + 'a
    where
        F: 'a,
        S: 'a,
    {
        async move {
            let v = self.s.do_async(client).await?;
            let ss = (self.f)(v)?;
            ss.do_async(client).await
        }
    }
}

#[doc(hidden)]
pub struct SequenceErrChain<S, F> {
    s: S,
    f: F,
}
impl<SS, S, F> Sequence for SequenceErrChain<S, F>
where
    SS: Sequence<Output = S::Output, Error = S::Error>,
    S: Sequence,
    F: FnOnce(S::Error) -> Result<SS, S::Error>,
    <SS as Sequence>::Error: From<S::Error> + Debug,
{
    type Output = SS::Output;
    type Error = SS::Error;

    fn do_sync<T: ClientSync>(self, client: &T) -> Result<Self::Output, Self::Error> {
        match self.s.do_sync(client) {
            Err(e) => {
                let ss = (self.f)(e)?;
                ss.do_sync(client)
            }
            Ok(v) => Ok(v),
        }
    }

    #[cfg(not(feature = "async-traits"))]
    fn do_async<'a, T: ClientAsync>(
        self,
        client: &'a T,
    ) -> Pin<Box<dyn Future<Output = Result<Self::Output, Self::Error>> + 'a>>
    where
        Self: 'a,
    {
        Box::pin(async move {
            match self.s.do_async(client).await {
                Err(e) => {
                    let ss = (self.f)(e)?;
                    ss.do_async(client).await
                }
                Ok(v) => Ok(v),
            }
        })
    }

    #[cfg(feature = "async-traits")]
    fn do_async<'a, T: ClientAsync>(
        self,
        client: &'a T,
    ) -> impl Future<
        Output = Result<
            <SequenceChain<S, F> as Sequence>::Output,
            <SequenceChain<S, F> as Sequence>::Error,
        >,
    > + 'a
    where
        F: 'a,
        S: 'a,
    {
        async move {
            match self.s.do_async(client).await {
                Err(e) => {
                    let ss = (self.f)(e)?;
                    ss.do_async(client).await
                }
                Ok(v) => Ok(v),
            }
        }
    }
}
