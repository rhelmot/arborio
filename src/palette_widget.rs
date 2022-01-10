use std::fmt::Debug;
use std::marker::PhantomData;
use std::ops::Deref;
use std::rc::Rc;
use vizia::*;

use crate::atlas_img::SpriteReference;
use crate::assets;
use crate::units::*;

pub struct PaletteWidget<T, L> {
    lens: L,
    marker: PhantomData<T>,
}

impl<T: PaletteItem, L> PaletteWidget<T, L>
where
    L: Lens<Target = T>
{
    pub fn new<'a, F>(cx: &'a mut Context, items: &'static Vec<T>, selected: L, callback: F) -> Handle<'a, Self>
    where
        F: 'static + Fn(&mut Context, T) + Copy
    {
        assert_ne!(items.len(), 0, "Palette may not be constructed with zero items");
        Self { lens: selected, marker: PhantomData {} }
            .build2(cx, move |cx| {
                Binding::new(cx, selected, move |cx, selected_field| {
                    let selected = *selected_field.get(cx);
                    VStack::new(cx, move |cx| {
                        for elem in items.iter() {
                            let elem = elem.clone();
                            let checked = elem.same(&selected);
                            HStack::new(cx, move |cx| {
                                Label::new(cx, &elem.display_name());
                            })
                                .class("palette_item")
                                .checked(checked)
                                .on_press(move |cx| {
                                    (callback)(cx, elem);
                                });
                        }
                    });
                });
            })
            .child_top(Units::Pixels(100.0))
    }
}

impl<T: PaletteItem, L: Lens<Target = T>> View for PaletteWidget<T, L> {
    fn draw(&self, cx: &Context, canvas: &mut Canvas) {
        let entity = cx.current;
        let bounds = cx.cache.get_bounds(entity);
        let data = self.lens.view(cx.data::<<L as Lens>::Source>().unwrap());

        canvas.save();
        canvas.translate(bounds.x, bounds.y);
        canvas.scissor(0.0, 0.0, bounds.w, 100.0);
        data.draw(canvas);
        canvas.restore();
    }
}

pub trait PaletteItem: Copy + Clone + Data + Debug + Send {
    fn search_text(&self) -> String;
    fn display_name(&self) -> String;
    fn draw(&self, canvas: &mut Canvas);
}

#[derive(Copy, Clone, Debug)]
pub struct TileSelectable {
    pub id: char,
    pub name: &'static str,
    pub texture: Option<SpriteReference>,
}

impl Default for TileSelectable {
    fn default() -> Self {
        TileSelectable {
            id: '0',
            name: "Empty",
            texture: None
        }
    }
}

impl PartialEq for TileSelectable {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

unsafe impl Send for TileSelectable {}

impl Data for TileSelectable {
    fn same(&self, other: &Self) -> bool {
        self == other
    }
}

impl PaletteItem for TileSelectable {
    fn search_text(&self) -> String {
        self.name.to_owned()
    }

    fn display_name(&self) -> String {
        self.name.to_owned()
    }

    fn draw(&self, canvas: &mut Canvas) {
        canvas.scale(3.0, 3.0);
        if let Some(texture) = self.texture {
            let dim = assets::GAMEPLAY_ATLAS.sprite_dimensions(texture);
            let slice = Rect::new(Point2D::zero(), dim.cast());
            assets::GAMEPLAY_ATLAS.draw_sprite(canvas, texture, 0.0, 0.0, slice);
        }
    }
}
