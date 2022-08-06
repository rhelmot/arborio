use arborio_utils::vizia::prelude::*;
use arborio_utils::vizia::vg::{Paint, Path};
use std::marker::PhantomData;

use arborio_state::data::app::AppState;
use arborio_state::palette_item::PaletteItem;

pub struct PaletteWidget<T, L> {
    lens: L,
    marker: PhantomData<T>,
}

impl<T: PaletteItem, LI> PaletteWidget<T, LI>
where
    LI: Lens<Target = T>,
{
    pub fn new<F, LL>(cx: &mut Context, items: LL, selected: LI, callback: F) -> Handle<Self>
    where
        F: 'static + Fn(&mut EventContext, T) + Copy,
        LL: Lens<Target = Vec<T>>,
        <LI as Lens>::Source: Model,
        <LL as Lens>::Source: Model,
    {
        let result = Self {
            lens: selected.clone(),
            marker: PhantomData {},
        }
        .build(cx, move |cx| {
            ScrollView::new(cx, 0.0, 0.0, false, true, move |cx| {
                List::new(cx, items, move |cx, _, item| {
                    let item2 = item.clone();
                    let item3 = item.clone();
                    HStack::new(cx, move |cx| {
                        Label::new(cx, "").bind(item2, |handle, item| {
                            let app = handle.cx.data::<AppState>().unwrap();
                            let text = item.get(handle.cx).display_name(app);
                            handle.text(&text);
                        });
                    })
                    .class("palette_item")
                    .class("list_highlight")
                    .bind(selected.clone(), move |handle, selected| {
                        let mine = item3.get(handle.cx);
                        let selected = selected.get(handle.cx);
                        handle.checked(selected.same(&mine));
                    })
                    .on_press(move |cx| {
                        (callback)(cx, item.get(cx));
                    });
                });
            });
        });

        if T::CAN_DRAW {
            result.child_top(Units::Pixels(100.0))
        } else {
            result
        }
    }
}

impl<T: PaletteItem, L: Lens<Target = T>> View for PaletteWidget<T, L> {
    fn element(&self) -> Option<&'static str> {
        Some("palette")
    }

    fn draw(&self, cx: &mut DrawContext, canvas: &mut Canvas) {
        if !T::CAN_DRAW {
            return;
        }

        let bounds = cx.bounds();
        let data = self
            .lens
            .view(cx.data::<<L as Lens>::Source>().unwrap(), |x| *x.unwrap());

        canvas.save();
        canvas.translate(bounds.x, bounds.y);
        canvas.scissor(0.0, 0.0, bounds.w, 100.0);

        let mut path = Path::new();
        path.rect(0.0, 0.0, bounds.w, 100.0);
        canvas.fill_path(
            &mut path,
            Paint::linear_gradient(
                0.0,
                0.0,
                0.0,
                100.0,
                Color::black().into(),
                Color::blue().into(),
            ),
        );

        data.draw(cx.data::<AppState>().unwrap(), canvas);
        canvas.restore();
    }
}
