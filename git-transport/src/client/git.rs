use crate::{client::SetServiceResponse, Protocol, Service};
use bstr::{BString, ByteVec};
use std::{io, net::TcpStream};

pub struct Connection<R, W> {
    read: R,
    write: W,
    path: BString,
    virtual_host: Option<(String, Option<u16>)>,
    protocol: Protocol,
}

impl<R, W> crate::client::Transport for Connection<R, W>
where
    R: io::Read,
    W: io::Write,
{
}

impl<R, W> crate::client::TransportSketch for Connection<R, W>
where
    R: io::Read,
    W: io::Write,
{
    fn set_service(&mut self, service: Service) -> Result<SetServiceResponse, crate::client::Error> {
        let mut out = bstr::BString::from(service.as_str());
        out.push(b' ');
        out.extend_from_slice(&self.path);
        out.push(0);
        if let Some((host, port)) = self.virtual_host.as_ref() {
            out.push_str("host=");
            out.extend_from_slice(host.as_bytes());
            out.push(0);
            if let Some(port) = port {
                out.push_byte(b':');
                out.push_str(&format!("{}", port));
            }
        }
        out.push(0);
        out.push_str(format!("version={}", self.protocol as usize));
        out.push(0);
        self.write.write_all(&out)?;
        self.write.flush()?;

        Ok(SetServiceResponse {
            actual_protocol: Protocol::V1, // TODO
            capabilities: vec![],          // TODO
            refs: None,                    // TODO
        })
    }
}

impl<R, W> Connection<R, W>
where
    R: io::Read,
    W: io::Write,
{
    pub fn new(
        read: R,
        write: W,
        desired_version: Protocol,
        repository_path: impl Into<BString>,
        virtual_host: Option<(impl Into<String>, Option<u16>)>,
    ) -> Self {
        Connection {
            read,
            write,
            path: repository_path.into(),
            virtual_host: virtual_host.map(|(h, p)| (h.into(), p)),
            protocol: desired_version,
        }
    }
}

use quick_error::quick_error;
quick_error! {
    #[derive(Debug)]
    pub enum Error {
        Tbd {
            display("tbd")
        }
    }
}

pub fn connect(
    _host: &str,
    _path: BString,
    _version: crate::Protocol,
    _port: Option<u16>,
) -> Result<Connection<TcpStream, TcpStream>, Error> {
    unimplemented!("file connection")
}
