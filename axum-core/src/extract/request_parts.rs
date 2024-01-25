use super::{rejection::*, FromRequest, FromRequestParts, Request};
use crate::{body::Body, RequestExt};
use async_trait::async_trait;
use bytes::{Buf, BufMut, Bytes};
use http::{request::Parts, Extensions, HeaderMap, Method, Uri, Version};
use http_body_util::BodyExt;
use std::convert::Infallible;

#[async_trait]
impl<S> FromRequest<S> for Request
where
    S: Send + Sync,
{
    type Rejection = Infallible;

    async fn from_request(req: Request, _: &S) -> Result<Self, Self::Rejection> {
        Ok(req)
    }
}

#[async_trait]
impl<S> FromRequestParts<S> for Method
where
    S: Send + Sync,
{
    type Rejection = Infallible;

    async fn from_request_parts(parts: &mut Parts, _: &S) -> Result<Self, Self::Rejection> {
        Ok(parts.method.clone())
    }
}

#[async_trait]
impl<S> FromRequestParts<S> for Uri
where
    S: Send + Sync,
{
    type Rejection = Infallible;

    async fn from_request_parts(parts: &mut Parts, _: &S) -> Result<Self, Self::Rejection> {
        Ok(parts.uri.clone())
    }
}

#[async_trait]
impl<S> FromRequestParts<S> for Version
where
    S: Send + Sync,
{
    type Rejection = Infallible;

    async fn from_request_parts(parts: &mut Parts, _: &S) -> Result<Self, Self::Rejection> {
        Ok(parts.version)
    }
}

/// Clone the headers from the request.
///
/// Prefer using [`TypedHeader`] to extract only the headers you need.
///
/// [`TypedHeader`]: https://docs.rs/axum/0.7/axum/extract/struct.TypedHeader.html
#[async_trait]
impl<S> FromRequestParts<S> for HeaderMap
where
    S: Send + Sync,
{
    type Rejection = Infallible;

    async fn from_request_parts(parts: &mut Parts, _: &S) -> Result<Self, Self::Rejection> {
        Ok(parts.headers.clone())
    }
}

#[async_trait]
impl<S> FromRequest<S> for Bytes
where
    S: Send + Sync,
{
    type Rejection = BytesRejection;

    async fn from_request(req: Request, _: &S) -> Result<Self, Self::Rejection> {
        // let bytes = req
        //     .into_limited_body()
        //     .collect()
        //     .await
        //     .map_err(FailedToBufferBody::from_err)?
        //     .to_bytes();
        let bytes = to_bytes(req.into_limited_body())
            .await
            .map_err(FailedToBufferBody::from_err)?;

        Ok(bytes)
    }
}

async fn to_bytes<T>(body: T) -> Result<Bytes, T::Error>
where
    T: http_body::Body<Data = Bytes>,
{
    futures_util::pin_mut!(body);
    // body.frame().await

    // If there's only 1 chunk, we can just return Buf::to_bytes()
    let mut first = if let Some(buf) = body.frame().await {
        buf?
    } else {
        return Ok(Bytes::new());
    };
    let first = first.data_mut().unwrap();

    let mut second = if let Some(buf) = body.frame().await {
        buf?
    } else {
        return Ok(first.copy_to_bytes(first.remaining()));
    };
    let second = second.data_mut().unwrap();

    // With more than 1 buf, we gotta flatten into a Vec first.
    let cap = first.remaining() + second.remaining() + body.size_hint().lower() as usize;
    let mut vec = Vec::with_capacity(cap);
    vec.put(first);
    vec.put(second);

    while let Some(buf) = body.frame().await {
        let mut data = buf?;
        let data = data.data_mut().unwrap();
        vec.put(data);
    }

    Ok(vec.into())
}

#[async_trait]
impl<S> FromRequest<S> for String
where
    S: Send + Sync,
{
    type Rejection = StringRejection;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let bytes = Bytes::from_request(req, state)
            .await
            .map_err(|err| match err {
                BytesRejection::FailedToBufferBody(inner) => {
                    StringRejection::FailedToBufferBody(inner)
                }
            })?;

        let string = std::str::from_utf8(&bytes)
            .map_err(InvalidUtf8::from_err)?
            .to_owned();

        Ok(string)
    }
}

#[async_trait]
impl<S> FromRequestParts<S> for Parts
where
    S: Send + Sync,
{
    type Rejection = Infallible;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        Ok(parts.clone())
    }
}

#[async_trait]
impl<S> FromRequestParts<S> for Extensions
where
    S: Send + Sync,
{
    type Rejection = Infallible;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        Ok(parts.extensions.clone())
    }
}

#[async_trait]
impl<S> FromRequest<S> for Body
where
    S: Send + Sync,
{
    type Rejection = Infallible;

    async fn from_request(req: Request, _: &S) -> Result<Self, Self::Rejection> {
        Ok(req.into_body())
    }
}
