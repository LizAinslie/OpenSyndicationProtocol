use bytes::BytesMut;
use tokio::io;
use crate::packet::{DeserializePacket, SerializePacket};

pub enum TransferPacketGuestToHost {

}

pub enum TransferPacketHostToGuest {

}

impl SerializePacket for TransferPacketGuestToHost {
    fn serialize(&self, buf: &mut BytesMut) -> io::Result<usize> {
        todo!()
    }
}

impl DeserializePacket for TransferPacketGuestToHost {
    type Output = TransferPacketGuestToHost;

    fn deserialize(buf: &mut BytesMut) -> io::Result<Self::Output> {
        todo!()
    }
}

impl SerializePacket for TransferPacketHostToGuest {
    fn serialize(&self, buf: &mut BytesMut) -> io::Result<usize> {
        todo!()
    }
}

impl DeserializePacket for TransferPacketHostToGuest {
    type Output = TransferPacketHostToGuest;

    fn deserialize(buf: &mut BytesMut) -> io::Result<Self::Output> {
        todo!()
    }
}
