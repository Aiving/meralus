#![allow(clippy::missing_errors_doc)]

use futures::{SinkExt, StreamExt};
use glam::Vec3;
use serde::{Deserialize, Serialize};
use std::{
    io::{self, Cursor},
    marker::PhantomData,
    pin::Pin,
};
use tokio::net::{
    TcpStream,
    tcp::{OwnedReadHalf, OwnedWriteHalf},
};
use tokio_serde::{Deserializer, Framed, Serializer};
use tokio_util::{
    bytes::{Buf, Bytes, BytesMut},
    codec::{FramedRead, FramedWrite, LengthDelimitedCodec},
};

pub type WrappedStream = FramedRead<OwnedReadHalf, LengthDelimitedCodec>;
pub type WrappedSink = FramedWrite<OwnedWriteHalf, LengthDelimitedCodec>;

#[derive(Debug)]
pub struct Bson<Item, SinkItem> {
    phantom: PhantomData<(Item, SinkItem)>,
}

impl<Item, SinkItem> Default for Bson<Item, SinkItem> {
    fn default() -> Self {
        Self {
            phantom: PhantomData,
        }
    }
}

impl<Item, SinkItem> Deserializer<Item> for Bson<Item, SinkItem>
where
    for<'a> Item: Deserialize<'a>,
{
    type Error = io::Error;

    fn deserialize(self: Pin<&mut Self>, src: &BytesMut) -> Result<Item, Self::Error> {
        bson::from_reader(Cursor::new(src).reader()).map_err(|err| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Failed to deserialize BSON: {err}"),
            )
        })
    }
}

impl<Item, SinkItem: Serialize> Serializer<SinkItem> for Bson<Item, SinkItem> {
    type Error = io::Error;

    fn serialize(self: Pin<&mut Self>, item: &SinkItem) -> Result<Bytes, Self::Error> {
        bson::to_vec(item).map(Bytes::from).map_err(|err| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Failed to serialize BSON: {err}"),
            )
        })
    }
}

pub type InStream<T = ()> = Framed<WrappedStream, T, (), Bson<T, ()>>;
pub type OutSink<T = ()> = Framed<WrappedSink, (), T, Bson<(), T>>;

pub fn wrap_stream<I, O>(stream: TcpStream) -> (InStream<I>, OutSink<O>) {
    let (read, write) = stream.into_split();
    let stream = WrappedStream::new(read, LengthDelimitedCodec::new());
    let sink = WrappedSink::new(write, LengthDelimitedCodec::new());

    (
        InStream::new(stream, Bson::default()),
        OutSink::new(sink, Bson::default()),
    )
}

pub struct Client(InStream<OutgoingPacket>, OutSink<IncomingPacket>);

impl Client {
    pub fn new(stream: TcpStream) -> Self {
        let (in_stream, out_stream) = wrap_stream(stream);

        Self(in_stream, out_stream)
    }

    pub async fn receive(&mut self) -> Option<Result<OutgoingPacket, io::Error>> {
        self.0.next().await
    }

    pub async fn send(&mut self, packet: IncomingPacket) -> Result<(), io::Error> {
        self.1.send(packet).await
    }
}

pub struct ServerConnection(InStream<IncomingPacket>, OutSink<OutgoingPacket>);

impl ServerConnection {
    pub fn new(stream: TcpStream) -> Self {
        let (in_stream, out_stream) = wrap_stream(stream);

        Self(in_stream, out_stream)
    }

    pub async fn receive(&mut self) -> Option<Result<IncomingPacket, io::Error>> {
        self.0.next().await
    }

    pub async fn send(&mut self, packet: OutgoingPacket) -> Result<(), io::Error> {
        self.1.send(packet).await
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Player {
    pub nickname: String,
    pub position: Vec3,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum IncomingPacket {
    GetPlayers,
    PlayerConnected { name: String },
    PlayerMoved { position: Vec3 },
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum OutgoingPacket {
    PlayerConnected { name: String },
    PlayerMoved { name: String, position: Vec3 },
    PlayersList { players: Vec<Player> },
}
