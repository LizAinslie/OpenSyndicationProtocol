#![feature(thin_box)]

pub mod registry;

use std::ops::Deref;
use bincode::{Decode, Encode};
use bincode::config::Configuration;
use bincode::error::{DecodeError, EncodeError};

use bytes::{Bytes, BytesMut};

use downcast_rs::{Downcast, DowncastSync, impl_downcast};

use uuid::Uuid;

/// Base type for all OSP data objects
///
/// ## Example implementation
/// ```rust
/// use bincode::{Decode, Encode};
/// use osp_data::{impl_data, Data};
/// use uuid::Uuid;
/// use std::str::FromStr;
///
/// #[derive(Encode, Decode, Clone)]
/// pub struct MyData {
///     test_val: bool,
/// }
///
/// impl_data!(MyData, "995f6806-7c36-4e27-ab03-a422952287b6");
/// ```
pub trait Data : Send + Downcast {
    fn get_id_static() -> Uuid where Self : Sized;

    fn get_id(&self) -> Uuid where Self : Sized {
        Self::get_id_static()
    }
}
impl_downcast!(Data);

/// Implement data methods more easily.
/// [Uuid], [std::str::FromStr] and [Data] must be in scope
///
/// **Usage:** (Given some concrete type `YourType`) `impl_data!(YourType, "your-type-uuid");`
#[macro_export]
macro_rules! impl_data {
    ($t:ident, $id:expr) => {
        impl Data for $t {
            fn get_id_static() -> Uuid
            where
                Self: Sized
            {
                Uuid::from_str($id).unwrap()
            }
        }
    };
}

/// A meta type that contains encode/decode methods for writing [Data] to
/// a buffer, handlers assigned to [TData], and associated markers.
pub struct DataType<TData>
where
    TData : Data + ?Sized,
{
    handlers: Vec<Box<dyn DataHandler<TData>>>
}

impl<TData> DataType<TData>
where
    TData : Data + ?Sized
{
    pub fn new() -> Self {
        DataType::<TData> {
            handlers: Vec::new()
        }
    }

    pub fn get_id(&self) -> Uuid
    where
        TData : Sized
    {
        TData::get_id_static()
    }

    /// Decode a [TData] off a buffer
    pub fn decode_from_bytes(&self, buf: &Bytes) -> Result<(Box<TData>, usize), DecodeError>
    where
        TData : Decode,
    {
        let config = bincode::config::standard();
        let res = bincode::decode_from_slice::<TData, Configuration>(buf, config)?;
        Ok((Box::new(res.0), res.1))
    }

    /// Encode a [TData] onto a buffer
    pub fn encode_to_bytes(&self, buf: &mut BytesMut, obj: TData) -> Result<usize, EncodeError>
    where
        TData : Encode + Sized,
    {
        let config = bincode::config::standard();
        let len = bincode::encode_into_slice(obj, buf, config)?;
        Ok(len)
    }

    pub fn handle(&self, obj: Box<&TData>)
    {
        for handler in &self.handlers {
            handler.handle(obj.clone())
        }
    }
}

pub trait DataHandler<TData> : DowncastSync + Send + Sync
where
    TData : Data + 'static + ?Sized
{
    fn handle(&self, obj: Box<&TData>);
}

impl_downcast!(sync DataHandler<TData> where TData : Data + 'static);

impl<TData : Data + ?Sized, F: Fn(Box<&TData>) + Send + Sync + 'static> DataHandler<TData> for F {
    fn handle(&self, obj: Box<&TData>) {
        self(obj)
    }
}


#[cfg(test)]
mod tests {}