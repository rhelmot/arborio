use std::marker::PhantomData;

use arborio_state::data::app::AppState;
use arborio_state::lenses::ClosureLens;
use arborio_state::palette_item::PaletteItem;
use arborio_utils::vizia::prelude::*;
use arborio_utils::vizia::resource::FontOrId;
use arborio_utils::vizia::vg::{Baseline, Paint, Path};

pub struct PaletteWidget<T, L> {
    lens: L,
    marker: PhantomData<T>,
    filter: String,
}

fn palette_widget_filter_lens<T: 'static, L: 'static>(
) -> impl Lens<Source = PaletteWidget<T, L>, Target = String> {
    ClosureLens::new(|source: &PaletteWidget<T, L>| Some(&source.filter))
}

impl<T: PaletteItem + Send + Sync, LI> PaletteWidget<T, LI>
where
    LI: Lens<Target = T>,
{
    pub fn new<F, LL>(cx: &mut Context, items: LL, selected: LI, callback: F) -> Handle<Self>
    where
        F: 'static + Send + Sync + Fn(&mut EventContext, T) + Copy,
        LL: Send + Sync + Lens<Target = Vec<T>>,
        <LI as Lens>::Source: Model,
        <LL as Lens>::Source: Model,
    {
        let result = Self {
            lens: selected.clone(),
            marker: PhantomData {},
            filter: "".to_owned(),
        }
        .build(cx, move |cx| {
            ScrollView::new(cx, 0.0, 0.0, false, true, move |cx| {
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
                        handle.checked(selected == mine);
                    })
                    .bind(
                        palette_widget_filter_lens::<T, LI>(),
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
                Color::black().into(),
                Color::blue().into(),
            ),
        );

        canvas.save();
        data.draw(cx.data::<AppState>().unwrap(), canvas);
        canvas.restore();

        let default_font = cx
            .resource_manager
            .fonts
            .get(cx.default_font())
            .and_then(|font| match font {
                FontOrId::Id(id) => Some(id),
                _ => None,
            })
            .expect("Failed to find default font");
        let text_paint = Paint::color(Color::white().into())
            .with_font_size(10.0 * dpi)
            .with_font(&[*default_font])
            .with_text_baseline(Baseline::Bottom);
        let text_black = text_paint.clone().with_color(Color::black().into());
        canvas
            .fill_text(11.0 * dpi, 91.0 * dpi, &self.filter, &text_black)
            .expect("Could not draw text");
        canvas
            .fill_text(9.0 * dpi, 89.0 * dpi, &self.filter, &text_black)
            .expect("Could not draw text");
        canvas
            .fill_text(10.0 * dpi, 90.0 * dpi, &self.filter, &text_paint)
            .expect("Could not draw text");

        canvas.restore();
    }

    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        event.map(|window_event, _| match window_event {
            WindowEvent::CharInput(ch) => {
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
            }
            WindowEvent::MouseDown(MouseButton::Left) => {
                cx.focus();
            }
            _ => {}
        });
    }
}
