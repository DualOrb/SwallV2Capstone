use bytes::Bytes;
use gstreamer::{
    event::Navigation,
    glib::{
        self, translate::UnsafeFrom, types::StaticType, value::ToValue, SendValue, Type, Value,
    },
    prelude::GstValueExt,
    Event, Structure, StructureRef,
};
use indexmap::set::IndexSet;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::{io::AsyncReadExt, net::TcpStream};

#[derive(Debug, Serialize, Deserialize)]
struct NavigationEventSerializer<'a> {
    name: &'a str,
    contents: Vec<(&'a str, String, u32)>,
}

#[derive(Debug, Error)]
pub enum NavigationSerializeError<'a> {
    #[error("Cannot serialize '{0}'. Returned {1}")]
    CannotSerializeValue(&'a str, glib::BoolError),
    #[error("Failed to serialize because {0}")]
    PostcardSerializeFailed(#[from] postcard::Error),
}

static SERIALIZABLE_TYPES: Lazy<IndexSet<Type>> = Lazy::new(|| {
    IndexSet::from_iter({
        // These types need to be [Send] in order for deserialization to be safe
        let types = [
            "gchararray",
            "gdouble",
            "gint",
            "GstNavigationModifierType",
            "guint",
        ];

        types
            .into_iter()
            .map(|type_name| {
                Type::from_name(type_name)
                    .unwrap_or_else(|| panic!("Could not find type with name '{type_name}'",))
            })
            .chain([gstreamer_video::NavigationModifierType::static_type()])
    })
});

pub fn serialize_event<'a>(event: &'a StructureRef) -> Result<Bytes, NavigationSerializeError<'a>> {
    let name = event.name();

    let iter = event.iter();
    let mut contents = Vec::with_capacity(iter.size_hint().0);
    for (field, value) in iter {
        // That these values are [Send] is an important safety invariance for deserialization to be safe
        let value: &glib::SendValue = value;

        let type_value = value.type_();
        let Some(type_id) = SERIALIZABLE_TYPES.get_index_of(&type_value) else {
            unimplemented!(
                "Cannot serialize type '{}'. Add the name to 'SERIALIZABLE_TYPES' (this type implements Send). (Content: {:?})",
                type_value.name(),
                value
            )
        };
        contents.push((
            field.as_str(),
            value
                .serialize()
                .map_err(|err| NavigationSerializeError::CannotSerializeValue(&field, err))?
                .as_str()
                .to_owned(),
            type_id.try_into().unwrap(),
        ));
    }

    let data = postcard::to_allocvec(&NavigationEventSerializer { name, contents })?;
    let len_data: u32 = data.len().try_into().unwrap();

    let payload = len_data.to_be_bytes().into_iter().chain(data);

    Ok(payload.collect())
}

#[derive(Debug, Error)]
pub enum NavigationDeserializeError {
    // #[error("Cannot serialize '{0}'. Returned {1}")]
    // CannotSerializeValue(&'a str, glib::BoolError),
    #[error("Io error: {}", 0)]
    IoError(#[from] std::io::Error),
    #[error("Failed to deserialize because {0}")]
    PostcardDeserializeFailed(#[from] postcard::Error),
    #[error("Invalid type id ({0})")]
    InvalidTypeId(u32),
    #[error("Deserialization error")]
    DeserializationError,
}

pub async fn deserialize_event(
    tcp_stream: &mut TcpStream,
    x_offset: u32,
    y_offset: u32,
) -> Result<Event, NavigationDeserializeError> {
    // TODO: Do client coordinate transformation here
    let msg_size = tcp_stream.read_u32().await?.try_into().unwrap();

    let mut buf = vec![]; // TODO: Reuse this allocation?
    buf.resize(msg_size, 0);

    let buf_slice = &mut buf[..msg_size];

    tcp_stream.read_exact(buf_slice).await?;

    let raw_event: NavigationEventSerializer = postcard::from_bytes(buf_slice).unwrap();

    let structure = raw_event
        .contents
        .into_iter()
        .try_fold(
            Structure::builder(raw_event.name),
            |structure_build, (name, value, type_id)| {
                let type_id_usize = type_id.try_into().unwrap();
                let mut value = Value::deserialize(
                    &value,
                    SERIALIZABLE_TYPES
                        .get_index(type_id_usize)
                        .ok_or(NavigationDeserializeError::InvalidTypeId(type_id))?
                        .clone(),
                )
                .map_err(|_| NavigationDeserializeError::DeserializationError)?;

                // Each screen sees their input locations locally, but screens must be offset to account for each screen's placement on the full canvas
                // (e.g. The right-most screen in the sWall would report an input on its top left pixel at (0, 0)
                // however in reference to the entire sWall it is (3240, 0))
                match name {
                    "pointer_x" => {
                        // Offset the coordinates where an input was detected to account for the screen of origin
                        value =
                            (value.get::<f64>().unwrap().floor() + (x_offset as f64)).to_value();
                    }
                    "pointer_y" => {
                        // This will not be used for the sWall, this is here for future compatability
                        value =
                            (value.get::<f64>().unwrap().floor() + (y_offset as f64)).to_value();
                    }
                    _ => {}
                }

                // SAFETY:  Casting [Value] to [SendValue] is safe if the value inside [Value] is [Send].
                //          This is always true here because the values in `SERIALIZABLE_TYPES` are all
                //          [Send]. Unfortunately determining if a value is [Send] cannot be queried at
                //          runtime.
                let send_value = unsafe { SendValue::unsafe_from(value.into_raw()) };
                Result::<_, NavigationDeserializeError>::Ok(structure_build.field(name, send_value))
            },
        )?
        .build();

    let event = Navigation::new(structure);

    Ok(event)
}
