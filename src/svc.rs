//! [`Service`] and [`BoxService`] traits.

use std::pin::Pin;
use std::sync::Arc;

/// A [`Service`] that produces rama services,
/// to serve requests with, be it transport layer requests or application layer requests.
pub trait Service<Request>: Sized + Send + Sync + 'static {
    /// The type of response returned by the service.
    type Response: Send + 'static;

    /// The type of error returned by the service.
    type Error: Send + 'static;

    /// Serve a response or error for the given request,
    /// using the given context.
    fn serve(
        &self,
        req: Request,
    ) -> impl Future<Output = Result<Self::Response, Self::Error>> + Send + '_;

    /// Box this service to allow for dynamic dispatch.
    fn boxed(self) -> BoxService<Request, Self::Response, Self::Error> {
        BoxService::new(self)
    }
}

impl<S, Request> Service<Request> for std::sync::Arc<S>
where
    S: Service<Request>,
{
    type Response = S::Response;
    type Error = S::Error;

    #[inline]
    fn serve(
        &self,
        req: Request,
    ) -> impl Future<Output = Result<Self::Response, Self::Error>> + Send + '_ {
        self.as_ref().serve(req)
    }
}

impl<S, Request> Service<Request> for &'static S
where
    S: Service<Request>,
{
    type Response = S::Response;
    type Error = S::Error;

    #[inline(always)]
    fn serve(
        &self,
        req: Request,
    ) -> impl Future<Output = Result<Self::Response, Self::Error>> + Send + '_ {
        (**self).serve(req)
    }
}

impl<S, Request> Service<Request> for Box<S>
where
    S: Service<Request>,
{
    type Response = S::Response;
    type Error = S::Error;

    #[inline]
    fn serve(
        &self,
        req: Request,
    ) -> impl Future<Output = Result<Self::Response, Self::Error>> + Send + '_ {
        self.as_ref().serve(req)
    }
}

/// Internal trait for dynamic dispatch of Async Traits,
/// implemented according to the pioneers of this Design Pattern
/// found at <https://rust-lang.github.io/async-fundamentals-initiative/evaluation/case-studies/builder-provider-api.html#dynamic-dispatch-behind-the-api>
/// and widely published at <https://blog.rust-lang.org/inside-rust/2023/05/03/stabilizing-async-fn-in-trait.html>.
trait DynService<Request> {
    type Response;
    type Error;

    #[allow(clippy::type_complexity)]
    fn serve_box(
        &self,
        req: Request,
    ) -> Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + '_>>;
}

impl<Request, T> DynService<Request> for T
where
    T: Service<Request>,
{
    type Response = T::Response;
    type Error = T::Error;

    fn serve_box(
        &self,
        req: Request,
    ) -> Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + '_>> {
        Box::pin(self.serve(req))
    }
}

/// A boxed [`Service`], to serve requests with,
/// for where you require dynamic dispatch.
pub struct BoxService<Request, Response, Error> {
    inner: Arc<dyn DynService<Request, Response = Response, Error = Error> + Send + Sync + 'static>,
}

impl<Request, Response, Error> Clone for BoxService<Request, Response, Error> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<Request, Response, Error> BoxService<Request, Response, Error> {
    /// Create a new [`BoxService`] from the given service.
    #[inline]
    pub fn new<T>(service: T) -> Self
    where
        T: Service<Request, Response = Response, Error = Error>,
    {
        Self {
            inner: Arc::new(service),
        }
    }
}

impl<Request, Response, Error> std::fmt::Debug for BoxService<Request, Response, Error> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BoxService").finish()
    }
}

impl<Request, Response, Error> Service<Request> for BoxService<Request, Response, Error>
where
    Request: 'static,
    Response: Send + 'static,
    Error: Send + 'static,
{
    type Response = Response;
    type Error = Error;

    #[inline]
    fn serve(
        &self,

        req: Request,
    ) -> impl Future<Output = Result<Self::Response, Self::Error>> + Send + '_ {
        self.inner.serve_box(req)
    }

    #[inline]
    fn boxed(self) -> Self {
        self
    }
}
