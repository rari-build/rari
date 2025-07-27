// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use deno_core::{OpState, ResourceId, error::ResourceError, extension, op2};
use deno_http::http_create_conn_resource;
use deno_net::{io::TcpStreamResource, ops_tls::TlsStreamResource};
use std::rc::Rc;

extension!(deno_http_runtime, ops = [op_http_start]);

#[derive(Debug, deno_error::JsError)]
pub enum HttpStartError {
    #[class("Busy")]
    TcpStreamInUse,
    #[class("Busy")]
    TlsStreamInUse,
    #[class("Busy")]
    UnixSocketInUse,
    #[class(generic)]
    ReuniteTcp(tokio::net::tcp::ReuniteError),
    #[cfg(unix)]
    #[class(generic)]
    ReuniteUnix(tokio::net::unix::ReuniteError),
    #[class(inherit)]
    Io(std::io::Error),
    #[class(inherit)]
    Resource(ResourceError),
}

impl std::fmt::Display for HttpStartError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HttpStartError::TcpStreamInUse => write!(f, "TCP stream is currently in use"),
            HttpStartError::TlsStreamInUse => write!(f, "TLS stream is currently in use"),
            HttpStartError::UnixSocketInUse => write!(f, "Unix socket is currently in use"),
            HttpStartError::ReuniteTcp(err) => write!(f, "{err}"),
            #[cfg(unix)]
            HttpStartError::ReuniteUnix(err) => write!(f, "{err}"),
            HttpStartError::Io(err) => write!(f, "{err}"),
            HttpStartError::Resource(err) => write!(f, "{err}"),
        }
    }
}

impl std::error::Error for HttpStartError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            HttpStartError::ReuniteTcp(err) => Some(err),
            #[cfg(unix)]
            HttpStartError::ReuniteUnix(err) => Some(err),
            HttpStartError::Io(err) => Some(err),
            HttpStartError::Resource(err) => Some(err),
            _ => None,
        }
    }
}

impl From<tokio::net::tcp::ReuniteError> for HttpStartError {
    fn from(err: tokio::net::tcp::ReuniteError) -> Self {
        HttpStartError::ReuniteTcp(err)
    }
}

#[cfg(unix)]
impl From<tokio::net::unix::ReuniteError> for HttpStartError {
    fn from(err: tokio::net::unix::ReuniteError) -> Self {
        HttpStartError::ReuniteUnix(err)
    }
}

impl From<std::io::Error> for HttpStartError {
    fn from(err: std::io::Error) -> Self {
        HttpStartError::Io(err)
    }
}

impl From<ResourceError> for HttpStartError {
    fn from(err: ResourceError) -> Self {
        HttpStartError::Resource(err)
    }
}

#[op2(fast)]
#[smi]
fn op_http_start(
    state: &mut OpState,
    #[smi] tcp_stream_rid: ResourceId,
) -> Result<ResourceId, HttpStartError> {
    if let Ok(resource_rc) = state.resource_table.take::<TcpStreamResource>(tcp_stream_rid) {
        // This TCP connection might be used somewhere else. If it's the case, we cannot proceed with the
        // process of starting a HTTP server on top of this TCP connection, so we just return a Busy error.
        // See also: https://github.com/denoland/deno/pull/16242
        let resource = Rc::try_unwrap(resource_rc).map_err(|_| HttpStartError::TcpStreamInUse)?;
        let (read_half, write_half) = resource.into_inner();
        let tcp_stream = read_half.reunite(write_half)?;
        let addr = tcp_stream.local_addr()?;
        return Ok(http_create_conn_resource(state, tcp_stream, addr, "http"));
    }

    if let Ok(resource_rc) = state.resource_table.take::<TlsStreamResource>(tcp_stream_rid) {
        // This TLS connection might be used somewhere else. If it's the case, we cannot proceed with the
        // process of starting a HTTP server on top of this TLS connection, so we just return a Busy error.
        // See also: https://github.com/denoland/deno/pull/16242
        let resource = Rc::try_unwrap(resource_rc).map_err(|_| HttpStartError::TlsStreamInUse)?;
        let tls_stream = resource.into_tls_stream();
        let addr = tls_stream.local_addr()?;
        return Ok(http_create_conn_resource(state, tls_stream, addr, "https"));
    }

    #[cfg(unix)]
    if let Ok(resource_rc) =
        state.resource_table.take::<deno_net::io::UnixStreamResource>(tcp_stream_rid)
    {
        // This UNIX socket might be used somewhere else. If it's the case, we cannot proceed with the
        // process of starting a HTTP server on top of this UNIX socket, so we just return a Busy error.
        // See also: https://github.com/denoland/deno/pull/16242
        let resource = Rc::try_unwrap(resource_rc).map_err(|_| HttpStartError::UnixSocketInUse)?;
        let (read_half, write_half) = resource.into_inner();
        let unix_stream = read_half.reunite(write_half)?;
        let addr = unix_stream.local_addr()?;
        return Ok(http_create_conn_resource(state, unix_stream, addr, "http+unix"));
    }

    Err(HttpStartError::Resource(ResourceError::BadResourceId))
}
