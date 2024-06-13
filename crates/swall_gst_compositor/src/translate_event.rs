use gstreamer::{glib, Event, StructureRef};
use swall_compositor::ButtonState;

#[derive(Debug, thiserror::Error)]
pub enum EventTranslationError {
    #[error("Struct missing field '{0}'")]
    MissingFieldError(&'static str),
    #[error("Field '{0}' has wrong type")]
    FieldWrongTypeError(&'static str),
    #[error("Unknown event type '{0}'")]
    UnknownEventType(String),
    #[error("Unknown butten type '{0}'")]
    UnknownButtonType(i32),
}

pub fn extract_value<'a, T>(
    structure: &'a StructureRef,
    name: &'static str,
) -> Result<T, EventTranslationError>
where
    T: glib::value::FromValue<'a>,
{
    structure
        .value(name)
        .map_err(|_| EventTranslationError::MissingFieldError(name))?
        .get()
        .map_err(|_error| EventTranslationError::FieldWrongTypeError(name))
}

pub fn translate_event(event: Event) -> Result<swall_compositor::Event, EventTranslationError> {
    let event_contents = event.structure().unwrap();
    match extract_value(event_contents, "event")? {
        "mouse-move" | "touch-motion" => Ok(swall_compositor::Event::Move {
            pointer_x: extract_value(event_contents, "pointer_x")?,
            pointer_y: extract_value(event_contents, "pointer_y")?,
        }),
        press_raw @ ("mouse-button-press" | "mouse-button-release" | "touch-down" | "touch-up") => {
            let state = match press_raw {
                "mouse-button-press" | "touch-down" => ButtonState::Pressed,
                "mouse-button-release" | "touch-up" => ButtonState::Released,
                _ => unreachable!(),
            };

            let button = match press_raw {
                "mouse-button-release" | "mouse-button-press" => {
                    let button: i32 = extract_value(event_contents, "button")?;

                    match button {
                        1 => 0x110, // TODO: Unhardcode with smithay_client_toolkit::seat::pointer::BTN_LEFT
                        3 => 0x111,
                        // 2 => 0x111,
                        _ => {
                            println!("Unknown button type: {event:#?}");
                            return Err(EventTranslationError::UnknownButtonType(button));
                        }
                    }
                }
                "touch-down" | "touch-up" => 0x110, // Left click
                _ => unreachable!(),
            };

            Ok(swall_compositor::Event::Button {
                state,
                button,
                pointer_x: extract_value(event_contents, "pointer_x")?,
                pointer_y: extract_value(event_contents, "pointer_y")?,
            })
        }
        other_event => Err(EventTranslationError::UnknownEventType(
            other_event.to_string(),
        )),
    }
}
