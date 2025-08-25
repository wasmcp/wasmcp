use std::collections::HashMap;
use std::cell::RefCell;
use std::rc::Rc;
use std::task::Poll;
use std::future::Future;
use futures::{future, sink, stream, Sink, Stream, TryStreamExt, SinkExt};
use spin_executor::CancelOnDropToken;
use anyhow::Result;
use crate::wit::wasi::http0_2_0::types::{
    Headers, IncomingBody, IncomingResponse, Method as WasiMethod, OutgoingBody, OutgoingRequest,
    Scheme, FutureIncomingResponse, ErrorCode
};
use crate::wit::wasi::http0_2_0::outgoing_handler;
use wasi::io::streams::{InputStream, OutputStream, StreamError};

const READ_SIZE: u64 = 16 * 1024;

#[derive(Debug, Clone, PartialEq)]
pub enum Method {
    Get,
    Post,
    Put,
    Delete,
    Patch,
    Head,
    Options,
}

impl From<Method> for WasiMethod {
    fn from(method: Method) -> Self {
        match method {
            Method::Get => WasiMethod::Get,
            Method::Post => WasiMethod::Post,
            Method::Put => WasiMethod::Put,
            Method::Delete => WasiMethod::Delete,
            Method::Patch => WasiMethod::Patch,
            Method::Head => WasiMethod::Head,
            Method::Options => WasiMethod::Options,
        }
    }
}

pub struct Request {
    method: Method,
    uri: String,
    headers: HashMap<String, String>,
    body: Vec<u8>,
}

impl Request {
    pub fn new(method: Method, uri: impl Into<String>) -> Self {
        Self {
            method,
            uri: uri.into(),
            headers: HashMap::new(),
            body: Vec::new(),
        }
    }

    pub fn get(uri: impl Into<String>) -> Self {
        Self::new(Method::Get, uri)
    }

    pub fn post(uri: impl Into<String>, body: impl Into<Vec<u8>>) -> Self {
        let mut req = Self::new(Method::Post, uri);
        req.body = body.into();
        req
    }

    pub fn builder() -> RequestBuilder {
        RequestBuilder::new()
    }

    pub fn header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(name.into(), value.into());
        self
    }

    pub fn body(mut self, body: impl Into<Vec<u8>>) -> Self {
        self.body = body.into();
        self
    }

    pub fn build(self) -> Self {
        self
    }
}

pub struct RequestBuilder {
    method: Option<Method>,
    uri: Option<String>,
    headers: HashMap<String, String>,
    body: Vec<u8>,
}

impl RequestBuilder {
    pub fn new() -> Self {
        Self {
            method: None,
            uri: None,
            headers: HashMap::new(),
            body: Vec::new(),
        }
    }

    pub fn method(mut self, method: Method) -> Self {
        self.method = Some(method);
        self
    }

    pub fn uri(mut self, uri: impl Into<String>) -> Self {
        self.uri = Some(uri.into());
        self
    }

    pub fn header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(name.into(), value.into());
        self
    }

    pub fn body(mut self, body: impl Into<Vec<u8>>) -> Self {
        self.body = body.into();
        self
    }

    pub fn build(self) -> Request {
        Request {
            method: self.method.unwrap_or(Method::Get),
            uri: self.uri.unwrap_or_else(|| String::from("/")),
            headers: self.headers,
            body: self.body,
        }
    }
}

pub struct Response {
    pub status: u16,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
}

impl Response {
    pub fn builder() -> ResponseBuilder {
        ResponseBuilder::new()
    }

    pub fn status(&self) -> u16 {
        self.status
    }

    pub fn headers(&self) -> &HashMap<String, String> {
        &self.headers
    }

    pub fn body(&self) -> &[u8] {
        &self.body
    }

    pub fn into_body(self) -> Vec<u8> {
        self.body
    }
}

pub struct ResponseBuilder {
    status: u16,
    headers: HashMap<String, String>,
    body: Vec<u8>,
}

impl ResponseBuilder {
    pub fn new() -> Self {
        Self {
            status: 200,
            headers: HashMap::new(),
            body: Vec::new(),
        }
    }

    pub fn status(mut self, status: u16) -> Self {
        self.status = status;
        self
    }

    pub fn header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(name.into(), value.into());
        self
    }

    pub fn body(mut self, body: impl Into<Vec<u8>>) -> Self {
        self.body = body.into();
        self
    }

    pub fn build(self) -> Response {
        Response {
            status: self.status,
            headers: self.headers,
            body: self.body,
        }
    }
}

// Internal executor functions copied from Spin SDK
fn outgoing_body(body: OutgoingBody) -> impl Sink<Vec<u8>, Error = StreamError> {
    struct Outgoing {
        stream_and_body: Option<(OutputStream, OutgoingBody)>,
        cancel_token: Option<CancelOnDropToken>,
    }

    impl Drop for Outgoing {
        fn drop(&mut self) {
            drop(self.cancel_token.take());

            if let Some((stream, body)) = self.stream_and_body.take() {
                drop(stream);
                _ = OutgoingBody::finish(body, None);
            }
        }
    }

    let stream = body.write().expect("response body should be writable");
    let outgoing = Rc::new(RefCell::new(Outgoing {
        stream_and_body: Some((stream, body)),
        cancel_token: None,
    }));

    sink::unfold((), {
        move |(), chunk: Vec<u8>| {
            future::poll_fn({
                let mut offset = 0;
                let mut flushing = false;
                let outgoing = outgoing.clone();

                move |context| {
                    let mut outgoing = outgoing.borrow_mut();
                    let (stream, _) = &outgoing.stream_and_body.as_ref().unwrap();
                    loop {
                        match stream.check_write() {
                            Ok(0) => {
                                outgoing.cancel_token = Some(CancelOnDropToken::from(
                                    spin_executor::push_waker_and_get_token(
                                        stream.subscribe(),
                                        context.waker().clone(),
                                    ),
                                ));
                                break Poll::Pending;
                            }
                            Ok(count) => {
                                if offset == chunk.len() {
                                    if flushing {
                                        break Poll::Ready(Ok(()));
                                    } else {
                                        match stream.flush() {
                                            Ok(()) => flushing = true,
                                            Err(StreamError::Closed) => break Poll::Ready(Ok(())),
                                            Err(e) => break Poll::Ready(Err(e)),
                                        }
                                    }
                                } else {
                                    let count =
                                        usize::try_from(count).unwrap().min(chunk.len() - offset);

                                    match stream.write(&chunk[offset..][..count]) {
                                        Ok(()) => {
                                            offset += count;
                                        }
                                        Err(e) => break Poll::Ready(Err(e)),
                                    }
                                }
                            }
                            // If the stream is closed but the entire chunk was
                            // written then we've done all we could so this
                            // chunk is now complete.
                            Err(StreamError::Closed) if offset == chunk.len() => {
                                break Poll::Ready(Ok(()))
                            }
                            Err(e) => break Poll::Ready(Err(e)),
                        }
                    }
                }
            })
        }
    })
}

fn outgoing_request_send(
    request: OutgoingRequest,
) -> impl Future<Output = Result<IncomingResponse, ErrorCode>> {
    struct State {
        response: Option<Result<FutureIncomingResponse, ErrorCode>>,
        cancel_token: Option<CancelOnDropToken>,
    }

    impl Drop for State {
        fn drop(&mut self) {
            drop(self.cancel_token.take());
            drop(self.response.take());
        }
    }

    let response = outgoing_handler::handle(request, None);
    let mut state = State {
        response: Some(response),
        cancel_token: None,
    };
    future::poll_fn({
        move |context| match &state.response.as_ref().unwrap() {
            Ok(response) => {
                if let Some(response) = response.get() {
                    Poll::Ready(response.unwrap())
                } else {
                    state.cancel_token = Some(CancelOnDropToken::from(
                        spin_executor::push_waker_and_get_token(
                            response.subscribe(),
                            context.waker().clone(),
                        ),
                    ));
                    Poll::Pending
                }
            }
            Err(error) => Poll::Ready(Err(error.clone())),
        }
    })
}

fn incoming_body(
    body: IncomingBody,
) -> impl Stream<Item = Result<Vec<u8>, wasi::io::streams::Error>> {
    struct Incoming {
        stream_and_body: Option<(InputStream, IncomingBody)>,
        cancel_token: Option<CancelOnDropToken>,
    }

    impl Drop for Incoming {
        fn drop(&mut self) {
            drop(self.cancel_token.take());

            if let Some((stream, body)) = self.stream_and_body.take() {
                drop(stream);
                IncomingBody::finish(body);
            }
        }
    }

    stream::poll_fn({
        let stream = body.stream().expect("response body should be readable");
        let mut incoming = Incoming {
            stream_and_body: Some((stream, body)),
            cancel_token: None,
        };

        move |context| {
            if let Some((stream, _)) = &incoming.stream_and_body {
                match stream.read(READ_SIZE) {
                    Ok(buffer) => {
                        if buffer.is_empty() {
                            incoming.cancel_token = Some(CancelOnDropToken::from(
                                spin_executor::push_waker_and_get_token(
                                    stream.subscribe(),
                                    context.waker().clone(),
                                ),
                            ));
                            Poll::Pending
                        } else {
                            Poll::Ready(Some(Ok(buffer)))
                        }
                    }
                    Err(StreamError::Closed) => Poll::Ready(None),
                    Err(StreamError::LastOperationFailed(error)) => Poll::Ready(Some(Err(error))),
                }
            } else {
                Poll::Ready(None)
            }
        }
    })
}

pub async fn send(request: Request) -> Result<Response> {
    // Parse the URI to extract components
    let uri = request.uri.parse::<http::Uri>()
        .map_err(|e| anyhow::anyhow!("Invalid URI: {}", e))?;
    
    let scheme = uri.scheme()
        .ok_or_else(|| anyhow::anyhow!("URI missing scheme"))?;
    let authority = uri.authority()
        .ok_or_else(|| anyhow::anyhow!("URI missing authority"))?;
    let path_and_query = uri.path_and_query()
        .map(|pq| pq.as_str())
        .unwrap_or("/");

    // Convert headers to WASI format
    let headers_vec: Vec<(String, Vec<u8>)> = request.headers
        .into_iter()
        .map(|(k, v)| (k, v.into_bytes()))
        .collect();
    
    let headers = Headers::from_list(&headers_vec)
        .map_err(|e| anyhow::anyhow!("Failed to create headers: {:?}", e))?;
    
    // Create the outgoing request
    let outgoing_request = OutgoingRequest::new(headers);
    
    outgoing_request
        .set_method(&WasiMethod::from(request.method))
        .map_err(|_| anyhow::anyhow!("Failed to set method"))?;
    
    outgoing_request
        .set_scheme(Some(&match scheme.as_str() {
            "https" => Scheme::Https,
            "http" => Scheme::Http,
            other => Scheme::Other(other.to_string()),
        }))
        .map_err(|_| anyhow::anyhow!("Failed to set scheme"))?;
    
    outgoing_request
        .set_authority(Some(authority.as_str()))
        .map_err(|_| anyhow::anyhow!("Failed to set authority"))?;
    
    outgoing_request
        .set_path_with_query(Some(path_and_query))
        .map_err(|_| anyhow::anyhow!("Failed to set path"))?;

    // Send the request with body if present
    let incoming_response = if !request.body.is_empty() {
        let body_handle = outgoing_request.body()
            .expect("request body should be available");
        let mut body_sink = outgoing_body(body_handle);
        let response_future = outgoing_request_send(outgoing_request);
        body_sink.send(request.body).await
            .map_err(|e| anyhow::anyhow!("Failed to send request body: {:?}", e))?;
        drop(body_sink);
        response_future.await
            .map_err(|e| anyhow::anyhow!("HTTP request failed: {:?}", e))?
    } else {
        outgoing_request_send(outgoing_request).await
            .map_err(|e| anyhow::anyhow!("HTTP request failed: {:?}", e))?
    };

    // Get response status and headers
    let status = incoming_response.status();
    let response_headers = incoming_response.headers();
    
    // Convert headers to HashMap
    let mut headers = HashMap::new();
    for (name, value) in response_headers.entries() {
        headers.insert(
            name.to_string(),
            String::from_utf8_lossy(&value).to_string(),
        );
    }

    // Read the response body using the streaming approach
    let body_stream = incoming_response.consume()
        .expect("response body should be available");
    
    let mut stream = incoming_body(body_stream);
    let mut body = Vec::new();
    while let Some(chunk) = stream.try_next().await? {
        body.extend(chunk);
    }

    Ok(Response {
        status,
        headers,
        body,
    })
}