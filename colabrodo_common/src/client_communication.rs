//! Components for client-sourced messages to NOODLES servers

use ciborium::value::Value;
use serde::de::{self, Visitor};
use serde::{ser::SerializeStruct, Deserialize, Serialize};

use crate::nooid::*;

/// Message ID for client messages
pub trait ClientMessageID {
    fn message_id() -> u32;
}

/// Client introduction message
#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct IntroductionMessage {
    pub client_name: String,
}

impl ClientMessageID for IntroductionMessage {
    fn message_id() -> u32 {
        0
    }
}

// ============================================================================

/// Optional context for a method invocation
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InvokeIDType {
    Entity(EntityID),
    Table(TableID),
    Plot(PlotID),
}

impl serde::Serialize for InvokeIDType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut s = serializer.serialize_struct("Representation", 1)?;
        match self {
            InvokeIDType::Entity(i) => s.serialize_field("entity", i)?,
            InvokeIDType::Table(i) => s.serialize_field("table", i)?,
            InvokeIDType::Plot(i) => s.serialize_field("plot", i)?,
        }
        s.end()
    }
}

struct InvokeIDTypeDeVisitor;

impl<'de> Visitor<'de> for InvokeIDTypeDeVisitor {
    type Value = InvokeIDType;

    fn expecting(
        &self,
        formatter: &mut std::fmt::Formatter,
    ) -> std::fmt::Result {
        write!(formatter, "One of 'entity', 'table', or 'plot'.")
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::MapAccess<'de>,
    {
        let key: Option<String> = map.next_key()?;
        let value: Option<NooID> = map.next_value()?;

        if key.is_none() || value.is_none() {
            return Err(de::Error::missing_field("invocation target"));
        }

        let key = key.unwrap();
        let value = value.unwrap();

        match key.as_str() {
            "entity" => Ok(InvokeIDType::Entity(EntityID(value))),
            "table" => Ok(InvokeIDType::Table(TableID(value))),
            "plot" => Ok(InvokeIDType::Plot(PlotID(value))),
            _ => Err(de::Error::unknown_field(
                key.as_str(),
                &["entity", "table", "plot"],
            )),
        }
    }
}

impl<'de> Deserialize<'de> for InvokeIDType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_map(InvokeIDTypeDeVisitor)
    }
}

// ============================================================================

/// Method invocation message for the server
#[serde_with::skip_serializing_none]
#[derive(Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct MethodInvokeMessage {
    pub method: MethodID,
    pub context: Option<InvokeIDType>,
    pub invoke_id: Option<String>,
    pub args: Vec<Value>,
}

impl ClientMessageID for MethodInvokeMessage {
    fn message_id() -> u32 {
        1
    }
}

// ============================================================================

/// Collection of client-side message
pub enum AllClientMessages {
    Intro(IntroductionMessage),
    Invoke(MethodInvokeMessage),
}

// Root message from the client.
pub struct ClientRootMessage {
    pub list: Vec<AllClientMessages>,
}

struct ClientRootMessageVisitor;

fn parse_content<'de, T, A>(seq: &mut A) -> Result<T, A::Error>
where
    T: serde::de::Deserialize<'de>,
    A: serde::de::SeqAccess<'de>,
{
    let content: Option<T> = seq.next_element()?;

    let content =
        content.ok_or(serde::de::Error::missing_field("Missing content!"))?;

    Ok(content)
}

impl<'de> Visitor<'de> for ClientRootMessageVisitor {
    type Value = ClientRootMessage;

    fn expecting(
        &self,
        formatter: &mut std::fmt::Formatter,
    ) -> std::fmt::Result {
        write!(formatter, "An interleaved array of id and content")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::SeqAccess<'de>,
    {
        let mut ret = ClientRootMessage { list: Vec::new() };

        loop {
            let id = seq.next_element()?;

            if id.is_none() {
                break;
            }

            let id: u32 = id.unwrap();

            match id {
                0 => {
                    ret.list.push(AllClientMessages::Intro(parse_content::<
                        IntroductionMessage,
                        A,
                    >(
                        &mut seq
                    )?));
                }

                1 => {
                    ret.list.push(AllClientMessages::Invoke(parse_content::<
                        MethodInvokeMessage,
                        A,
                    >(
                        &mut seq
                    )?));
                }

                _ => {
                    return Err(serde::de::Error::invalid_value(
                        serde::de::Unexpected::Unsigned(id as u64),
                        &self,
                    ))
                }
            }
        }

        Ok(ret)
    }
}

impl<'de> Deserialize<'de> for ClientRootMessage {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_seq(ClientRootMessageVisitor)
    }
}

// ============================================================================
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_intro_messages() {
        let intro_message = IntroductionMessage {
            client_name: "test_name".to_string(),
        };

        let mut buffer: Vec<u8> = Vec::new();

        ciborium::ser::into_writer(&intro_message, &mut buffer).unwrap();

        //println!("{buffer:02X?}");

        let read: IntroductionMessage =
            ciborium::de::from_reader(buffer.as_slice()).unwrap();

        assert!(intro_message == read);
    }

    #[test]
    fn test_invoke_messages() {
        let m = MethodInvokeMessage {
            method: MethodID(NooID::new_with_slot(1)),
            context: Some(InvokeIDType::Table(TableID(NooID::new_with_slot(
                10,
            )))),
            ..Default::default()
        };

        let mut buffer: Vec<u8> = Vec::new();

        ciborium::ser::into_writer(&m, &mut buffer).unwrap();

        let read: MethodInvokeMessage =
            ciborium::de::from_reader(buffer.as_slice()).unwrap();

        //println!("{read:?}");

        assert_eq!(m, read);
    }
}
