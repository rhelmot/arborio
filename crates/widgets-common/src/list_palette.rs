use std::marker::PhantomData;

use arborio_state::data::app::AppState;
use arborio_state::lenses::ClosureLens;
use arborio_state::palette_item::PaletteItem;
use arborio_utils::vizia::prelude::*;
use arborio_utils::vizia::vg::{Paint, Path};

pub struct PaletteWidget<T, L, L2> {
    lens: L,
    other_lens: L2,
    marker: PhantomData<T>,
    filter: String,
}

fn palette_widget_filter_lens<T: 'static, L: 'static, L2: 'static>(
) -> impl Lens<Source = PaletteWidget<T, L, L2>, Target = String> {
    ClosureLens::new(|source: &PaletteWidget<T, L, L2>| Some(&source.filter))
}

impl<T: PaletteItem + Send + Sync, LI, LO> PaletteWidget<T, LI, LO>
where
    LI: Lens<Target = T>,
    LO: Lens<Target = String>,
{
    pub fn new<F, F2, LL>(
        cx: &mut Context,
        items: LL,
        selected: LI,
        callback: F,
        other_lens: LO,
        other_callback: F2,
    ) -> Handle<Self>
    where
        F: 'static + Send + Sync + Fn(&mut EventContext, T) + Copy,
        F2: 'static + Send + Sync + Fn(&mut EventContext, String) + Copy,
        LL: Send + Sync + Lens<Target = Vec<T>>,
        <LI as Lens>::Source: Model,
        <LL as Lens>::Source: Model,
    {
        let result = Self {
            lens: selected.clone(),
            other_lens: other_lens.clone(),
            marker: PhantomData {},
            filter: "".to_owned(),
        }
        .build(cx, move |cx| {
            ScrollView::new(cx, 0.0, 0.0, false, true, move |cx| {
                let selected2 = selected.clone();
                List::new(cx, items, move |cx, _, item| {
                    let item2 = item.clone();
                    let item3 = item.clone();
                    let item4 = item.clone();
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
                    .bind(
                        palette_widget_filter_lens::<T, LI, LO>(),
                        move |handle, filter| {
                            let filter = filter.get(handle.cx);
                            let item = item4.get(handle.cx);
                            let visible = item
                                .search_text(handle.cx.data().unwrap())
                                .to_lowercase()
                                .contains(&filter.to_lowercase());
                            handle.display(visible);
                        },
                    )
                    .on_press(move |cx| {
                        let item = item.get(cx);
                        (callback)(cx.as_mut(), item);
                    });
                });
                HStack::new(cx, move |cx| {
                    Label::new(cx, "Other...");
                })
                .class("palette_item")
                .class("list_highlight")
                .bind(selected2.clone(), move |handle, selected| {
                    let selected = selected.get(handle.cx);
                    handle.checked(selected.same(&T::other()));
                })
                .on_press(move |cx| {
                    let item = T::other();
                    (callback)(cx.as_mut(), item);
                });
                Textbox::new(cx, other_lens).on_edit(other_callback).bind(
                    selected2,
                    move |handle, selected| {
                        let selected = selected.get(handle.cx);
                        handle.display(selected.same(&T::other()));
                    },
                );
            });
        });

        if T::CAN_DRAW {
            result.child_top(Pixels(100.0))
        } else {
            result
        }
    }
}

impl<T: PaletteItem, L: Lens<Target = T>, LO: Lens<Target = String>> View
    for PaletteWidget<T, L, LO>
{
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
            .view(cx.data::<<L as Lens>::Source>().unwrap())
            .unwrap();
        let other = self.other_lens.get(cx);

        canvas.save();
        canvas.translate(bounds.x, bounds.y);
        let dpi = cx.style.dpi_factor as f32;
        canvas.scissor(0.0, 0.0, bounds.w, 100.0 * dpi);

        let mut path = Path::new();
        path.rect(0.0, 0.0, bounds.w, 100.0 * dpi);
        canvas.fill_path(
            &mut path,
            &Paint::linear_gradient(
                0.0,
                0.0,
                0.0,
                100.0 * dpi,
                Color::blue().into(),
                Color::black().into(),
            ),
        );

        canvas.save();
        data.draw(cx.data::<AppState>().unwrap(), canvas, &other);
        canvas.restore();

        cx.sync_text_styles();
        cx.draw_text(canvas, (10. * dpi, 100. * dpi), (0., 1.));

        canvas.restore();
    }

    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        event.map(|window_event, _| match window_event {
            WindowEvent::CharInput(ch) => {
                if cx.focused() != cx.current() {
                    return;
                }
                cx.needs_redraw();
                match *ch {
                    '\u{1b}' => self.filter.clear(),
                    '\u{08}' => {
                        let mut chars = self.filter.chars();
                        chars.next_back();
                        self.filter = chars.as_str().to_owned();
                    }
                    c if !c.is_control() => {
                        self.filter.push(c);
                    }
                    _ => {}
                }
                cx.text_context.set_text(cx.current(), &self.filter);
            }
            WindowEvent::MouseDown(MouseButton::Left) => {
                cx.focus();
            }
            _ => {}
        });
    }
}
