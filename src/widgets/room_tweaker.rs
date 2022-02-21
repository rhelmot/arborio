use std::str::FromStr;

use crate::lenses::CurrentRoomLens;
use crate::map_struct::CelesteMapLevel;
use vizia::*;

pub struct RoomTweakerWidget {}

impl RoomTweakerWidget {
    pub fn new(cx: &mut Context) -> Handle<'_, Self> {
        Self {}
            .build2(cx, |cx| {
                let room = CurrentRoomLens {};
                tweak_attr_text(
                    cx,
                    "Name",
                    room.then(CelesteMapLevel::name)
                        .map(|n| n.strip_prefix("lvl_").unwrap().to_owned()),
                    |_cx, name| println!("Changing name to {}", name),
                );
            })
            .class("tweaker")
    }
}

impl View for RoomTweakerWidget {
    fn element(&self) -> Option<String> {
        Some("room-tweaker".to_owned())
    }
}

fn tweak_attr_text<L, F>(cx: &mut Context, name: &'static str, lens: L, setter: F)
where
    L: Lens,
    <L as Lens>::Target: ToString + FromStr + Data,
    F: 'static + Send + Sync + Fn(&mut Context, <L as Lens>::Target),
{
    HStack::new(cx, move |cx| {
        Label::new(cx, name);
        Textbox::new(cx, lens).on_edit(move |cx, value| {
            if let Ok(parsed) = value.parse() {
                setter(cx, parsed);
            }
        });
    });
}
